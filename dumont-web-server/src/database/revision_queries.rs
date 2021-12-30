use crate::backend::models::PaginationOptions;
use crate::database::{
    entity::{self, prelude::*},
    repo_queries::{models::RepoParam, RepoQueries},
    revision_label_queries::RevisionLabelQueries,
    AlreadyExistsError, DatabaseError, DbResult, NotFoundError, PostgresDatabase,
};
use async_trait::async_trait;
use sea_orm::{entity::*, query::*};
use tracing::info;
use tracing_attributes::instrument;

/**
 * RevisionQueries is a collection of api calls against the database focused
 * on the revision.
 *
 * In the DB, we call things revision, and at the API level it's called version. This
 * is because revision is "more generic" than a version IMO, but version is
 * better used in the engineering lexicon.
 */
#[async_trait]
pub trait RevisionQueries {
    async fn create_revision(
        &self,
        revision_param: &RevisionParam<'_>,
        create_revision_param: &CreateRevisionParam<'_>,
    ) -> DbResult<DbRevisionModel>;

    async fn get_revision(&self, revision_param: &RevisionParam<'_>) -> DbResult<DbRevisionModel>;

    async fn sql_get_raw_revision(
        &self,
        revision_param: &RevisionParam<'_>,
    ) -> DbResult<Option<entity::repository_revision::Model>>;

    async fn sql_get_revision(
        &self,
        revision_param: &RevisionParam<'_>,
    ) -> DbResult<entity::repository_revision::Model>;

    async fn list_revisions(
        &self,
        repo_param: &RepoParam<'_>,
        pagination: PaginationOptions,
    ) -> DbResult<Vec<DbRevisionModel>>;

    async fn delete_revision(&self, revision_param: &RevisionParam<'_>) -> DbResult<bool>;
}

pub mod models {
    use crate::database::entity::{self};
    use crate::database::prelude::RevisionLabels;

    #[derive(Debug)]
    pub struct RevisionParam<'a> {
        pub org_name: &'a str,
        pub repo_name: &'a str,
        pub revision: &'a str,
    }

    impl<'a> RevisionParam<'a> {
        pub fn new(org: &'a str, repo: &'a str, revision: &'a str) -> Self {
            Self {
                org_name: org,
                repo_name: repo,
                revision,
            }
        }
    }

    #[derive(Debug)]
    pub struct CreateRevisionParam<'a> {
        pub artifact_url: Option<&'a str>,
        pub labels: RevisionLabels,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct DbRevisionModel {
        pub repo_id: i32,
        pub revision_id: i32,
        pub revision_name: String,
        pub artifact_url: Option<String>,
        pub labels: RevisionLabels,
    }

    impl DbRevisionModel {
        pub fn from(
            revision: entity::repository_revision::Model,
            labels: Vec<entity::repository_revision_label::Model>,
        ) -> Self {
            Self {
                repo_id: revision.repo_id,
                revision_id: revision.revision_id,
                revision_name: revision.revision_name,
                artifact_url: revision.artifact_url,
                labels: RevisionLabels::from(&labels),
            }
        }
    }
}

use models::*;

#[async_trait]
impl RevisionQueries for PostgresDatabase {
    #[instrument(skip(self))]
    async fn create_revision(
        &self,
        revision_param: &RevisionParam<'_>,
        create_revision_param: &CreateRevisionParam<'_>,
    ) -> DbResult<DbRevisionModel> {
        let repo_name = revision_param.repo_name.to_string();
        let org_name = revision_param.org_name.to_string();

        if let Some(found_revision) = self.sql_get_raw_revision(revision_param).await? {
            info!(
                revision = tracing::field::debug(&found_revision),
                "Found exiting revision for {:?}.", revision_param
            );
            return Err(DatabaseError::AlreadyExists {
                error: AlreadyExistsError::Revision {
                    org: revision_param.org_name.to_string(),
                    repo: revision_param.repo_name.to_string(),
                    revision: revision_param.revision.to_string(),
                },
            });
        }

        let repo = self.sql_get_repo(&org_name, &repo_name).await?;

        let model = entity::repository_revision::ActiveModel {
            repo_id: Set(repo.repo_id),
            revision_name: Set(revision_param.revision.to_string()),
            created_at: Set(self.date_time_provider.now().naive_utc()),
            artifact_url: Set(create_revision_param.artifact_url.map(|s| s.to_string())),
            ..Default::default()
        };

        let response: InsertResult<_> = RepositoryRevision::insert(model).exec(&self.db).await?;
        self.sql_set_revision_labels(
            response.last_insert_id,
            &create_revision_param.labels.labels,
        )
        .await?;

        self.get_revision(revision_param).await
    }

    #[instrument(skip(self))]
    async fn get_revision(&self, revision_param: &RevisionParam<'_>) -> DbResult<DbRevisionModel> {
        let revision = self.sql_get_revision(revision_param).await?;
        let labels = self.sql_get_revision_labels(&revision).await?;

        Ok(DbRevisionModel::from(revision, labels))
    }

    #[instrument(skip(self))]
    async fn delete_revision(&self, revision_param: &RevisionParam<'_>) -> DbResult<bool> {
        let revision = self.sql_get_raw_revision(revision_param).await?;

        let revision = match revision {
            Some(revision) => revision,
            None => {
                return Err(DatabaseError::NotFound {
                    error: NotFoundError::Revision {
                        org: revision_param.org_name.to_owned(),
                        repo: revision_param.repo_name.to_owned(),
                        revision: revision_param.revision.to_owned(),
                    },
                });
            }
        };
        let revision: entity::repository_revision::ActiveModel = revision.into();
        let res = revision.delete(&self.db).await?;

        Ok(res.rows_affected == 1)
    }

    #[instrument(skip(self))]
    async fn list_revisions(
        &self,
        repo_param: &RepoParam<'_>,
        pagination: PaginationOptions,
    ) -> DbResult<Vec<DbRevisionModel>> {
        let repo = self
            .sql_get_repo(repo_param.org_name, repo_param.repo_name)
            .await?;
        let select = repo
            .find_related(RepositoryRevision)
            .order_by_asc(entity::repository_revision::Column::RepoId)
            .paginate(&self.db, pagination.page_size)
            .fetch_page(pagination.page_number)
            .await?;

        let mut revisions = Vec::new();
        for revision in select {
            let labels = self.sql_get_revision_labels(&revision).await?;
            revisions.push(DbRevisionModel::from(revision, labels));
        }

        Ok(revisions)
    }

    #[instrument(skip(self))]
    async fn sql_get_raw_revision(
        &self,
        revision_param: &RevisionParam<'_>,
    ) -> DbResult<Option<entity::repository_revision::Model>> {
        let condition = Condition::all()
            .add(entity::organization::Column::OrgName.eq(revision_param.org_name))
            .add(entity::repository::Column::RepoName.eq(revision_param.repo_name))
            .add(entity::repository_revision::Column::RevisionName.eq(revision_param.revision));

        let revision = RepositoryRevision::find()
            .filter(condition)
            .join(
                JoinType::Join,
                entity::repository_revision::Relation::Repository.def(),
            )
            .join(
                JoinType::Join,
                entity::repository::Relation::Organization.def(),
            )
            .one(&self.db)
            .await?;

        Ok(revision)
    }

    #[instrument(skip(self))]
    async fn sql_get_revision(
        &self,
        revision_param: &RevisionParam<'_>,
    ) -> DbResult<entity::repository_revision::Model> {
        let revision = self.sql_get_raw_revision(revision_param).await?;

        match revision {
            None => {
                return Err(DatabaseError::NotFound {
                    error: NotFoundError::Revision {
                        org: revision_param.org_name.to_string(),
                        repo: revision_param.repo_name.to_string(),
                        revision: revision_param.revision.to_string(),
                    },
                })
            }
            Some(repo) => Ok(repo),
        }
    }
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::database::{
        org_queries::*, revision_label_queries::models::RevisionLabels, DateTimeProvider,
    };
    use crate::test_utils::*;
    use serial_test::serial;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_revision_create() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        db.create_org("foo").await.unwrap();
        create_repo(&db, "foo", "bar").await.unwrap();

        let revision = db
            .create_revision(
                &RevisionParam::new("foo", "bar", "1.2.3"),
                &CreateRevisionParam {
                    artifact_url: None,
                    labels: RevisionLabels::default(),
                },
            )
            .await
            .unwrap();

        assert_eq!(revision.revision_name, "1.2.3");
        assert_eq!(revision.artifact_url, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_duplicate_version() {
        // let _logging = logging_setup();
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        db.create_org("foo").await.unwrap();
        create_repo(&db, "foo", "bar").await.unwrap();

        let revision = db
            .create_revision(
                &RevisionParam::new("foo", "bar", "1.2.3"),
                &CreateRevisionParam {
                    artifact_url: None,
                    labels: RevisionLabels::default(),
                },
            )
            .await
            .unwrap();

        assert_eq!(revision.revision_name, "1.2.3");
        assert_eq!(revision.artifact_url, None);

        let revision = db
            .create_revision(
                &RevisionParam::new("foo", "bar", "1.2.3"),
                &CreateRevisionParam {
                    artifact_url: None,
                    labels: RevisionLabels::default(),
                },
            )
            .await;

        assert!(revision.is_err());
        let error = revision.unwrap_err();

        assert_eq!(error.to_string(), "Revision foo/bar/1.2.3 exists");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_delete_revision() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        create_org_and_repos(&db, "example", vec!["example-repo-1"])
            .await
            .unwrap();
        create_test_version(&db, "example", "example-repo-1", "1.2.3")
            .await
            .unwrap();

        db.delete_revision(&RevisionParam::new("example", "example-repo-1", "1.2.3"))
            .await
            .unwrap();

        let revisions = db
            .list_revisions(
                &RepoParam::new("example", "example-repo-1"),
                PaginationOptions::new(0, 50),
            )
            .await
            .unwrap();
        assert_eq!(revisions.len(), 0)
    }
}
