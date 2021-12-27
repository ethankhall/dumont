use super::prelude::*;
use crate::backend::models::PaginationOptions;
use crate::database::entity::{self, prelude::*};
use async_trait::async_trait;
use sea_orm::{entity::*, query::*};
use std::collections::BTreeMap;
use tracing::info;
use tracing_attributes::instrument;


#[async_trait]
pub trait RevisionQueries {
    async fn create_revision(&self, revision_param: &RevisionParam<'_>, create_revision_param: &CreateRevisionParam<'_>) -> DbResult<DbRevisionModel>;

    async fn set_revision_labels(
        &self,
        revision_param: &RevisionParam<'_>,
        labels: BTreeMap<String, String>,
    ) -> DbResult<()>;

    async fn sql_set_revision_labels(
        &self,
        revision: &entity::repository_revision::Model,
        labels: BTreeMap<String, String>,
    ) -> DbResult<()>;

    async fn get_revision_labels(&self, revision_param: &RevisionParam<'_>) -> DbResult<RevisionLabels>;
    async fn sql_get_revision_labels(
        &self,
        repo: &entity::repository_revision::Model,
    ) -> DbResult<Vec<entity::repository_revision_label::Model>>;

    async fn get_revision(&self, revision_param: &RevisionParam<'_>) -> DbResult<DbRevisionModel>;
    async fn sql_get_revision(
        &self,
        revision_param: &RevisionParam<'_>,
    ) -> DbResult<entity::repository_revision::Model>;

    async fn list_revision(
        &self,
        org_name: &str, repo_name: &str,
        pagination: PaginationOptions,
    ) -> DbResult<Vec<DbRevisionModel>>;

    // async fn delete_revision(&self, revision_param: &RevisionParam<'_>) -> DbResult<bool>;
}

pub mod models {
    use std::collections::BTreeMap;
    use crate::database::entity::{self};

    #[derive(Debug)]
    pub struct RevisionParam<'a> {
        pub org_name: &'a str,
        pub repo_name: &'a str,
        pub revision: &'a str
    }

    #[derive(Debug)]
    pub struct CreateRevisionParam<'a> {
        pub scm_id: &'a str,
        pub artifact_url: Option<&'a str>,
        pub labels: RevisionLabels,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct DbRevisionModel {
        pub repo_id: i32,
        pub revision_id: i32,
        pub revision_name: String,
        pub scm_id: String,
        pub artifact_url: Option<String>,
        pub labels: RevisionLabels,
    }

    impl DbRevisionModel {
        pub fn from(revision: entity::repository_revision::Model, labels: Vec<entity::repository_revision_label::Model>) -> Self {
            Self {
                repo_id: revision.repo_id,
                revision_id: revision.revision_id,
                revision_name: revision.revision_name,
                scm_id: revision.scm_id,
                artifact_url: revision.artifact_url,
                labels: RevisionLabels::from(&labels)
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct RevisionLabels {
        pub labels: BTreeMap<String, String>,
    }

    impl From<&Vec<entity::repository_revision_label::Model>> for RevisionLabels {
        fn from(source: &Vec<entity::repository_revision_label::Model>) -> Self {
            let mut labels: BTreeMap<String, String> = Default::default();
            for value in source.iter() {
                labels.insert(value.label_name.to_string(), value.label_value.to_string());
            }

            Self { labels }
        }
    }

    impl From<Vec<entity::repository_revision_label::Model>> for RevisionLabels {
        fn from(source: Vec<entity::repository_revision_label::Model>) -> Self {
            (&source).into()
        }
    }

    impl Default for RevisionLabels {
        fn default() -> Self {
            Self {
                labels: Default::default()
            }
        }
    }
}

use models::*;

#[async_trait]
impl RevisionQueries for PostresDatabase {
    #[instrument(level = "debug", skip(self))]
    async fn create_revision(&self, revision_param: &RevisionParam<'_>, create_revision_param: &CreateRevisionParam<'_>) -> DbResult<DbRevisionModel> {
        let repo_name = revision_param.repo_name.to_string();
        let org_name = revision_param.org_name.to_string();

        let repo = self.sql_get_repo(&org_name, &repo_name).await?;

        let model = entity::repository_revision::ActiveModel {
            repo_id: Set(repo.repo_id),
            revision_name: Set(revision_param.revision.to_string()),
            scm_id: Set(create_revision_param.scm_id.to_string()),
            created_at: Set(self.date_time_provider.now().naive_utc()),
            artifact_url: Set(create_revision_param.artifact_url.map(|s| s.to_string())),
            ..Default::default()
        };

        RepositoryRevision::insert(model).exec(&self.db).await?;

        self.get_revision(&revision_param).await
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_revision(&self, revision_param: &RevisionParam<'_>) -> DbResult<DbRevisionModel> {
        let revision = self.sql_get_revision(&revision_param).await?;
        let labels = self.sql_get_revision_labels(&revision).await?;

        Ok(DbRevisionModel::from(revision, labels))
    }

    #[instrument(level = "debug", skip(self))]
    async fn sql_get_revision_labels(
        &self,
        repo: &entity::repository_revision::Model,
    ) -> DbResult<Vec<entity::repository_revision_label::Model>> {

        let condition = Condition::all()
            .add(entity::repository_revision::Column::RevisionId.eq(repo.revision_id));

        Ok(RepositoryRevisionLabel::find()
            .join(JoinType::Join, entity::repository_revision_label::Relation::RepositoryRevision.def())
            .filter(condition)
            .all(&self.db)
            .await?)
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_revision_labels(&self, revision_param: &RevisionParam<'_>) -> DbResult<RevisionLabels> {
        let revision = self.sql_get_revision(&revision_param).await?;
        let labels = self.sql_get_revision_labels(&revision).await?;
        Ok(labels.into())
    }

    #[instrument(level = "debug", skip(self))]
    async fn sql_set_revision_labels(
        &self,
        revision: &entity::repository_revision::Model,
        labels: BTreeMap<String, String>,
    ) -> DbResult<()> {
        let mut new_labels = Vec::default();

        for (key, value) in labels {
            new_labels.push(entity::repository_revision_label::ActiveModel {
                revision_id: Set(revision.revision_id),
                label_name: Set(key.to_string()),
                label_value: Set(value.to_string()),
                created_at: Set(self.date_time_provider.now().naive_utc()),
                ..Default::default()
            })
        }

        let new_label_count = new_labels.len();

        let txn = self.db.begin().await?;

        let del = RepositoryRevisionLabel::delete_many()
            .filter(entity::repository_revision_label::Column::RevisionId.eq(revision.revision_id))
            .exec(&txn)
            .await?;
        if !new_labels.is_empty() {
            RepositoryRevisionLabel::insert_many(new_labels).exec(&txn).await?;
        }

        txn.commit().await?;

        info!(
            "Deleted {} rows, Inserted {} rows",
            del.rows_affected, new_label_count
        );
        Ok(())
    }

    #[instrument(level = "debug", skip(self))]
    async fn set_revision_labels(
        &self,
        revision_param: &RevisionParam<'_>,
        labels: BTreeMap<String, String>,
    ) -> DbResult<()> {

        let revision = self.sql_get_revision(&revision_param).await?;

        self.sql_set_revision_labels(&revision, labels).await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self))]
    async fn list_revision(
        &self,
        org_name: &str, repo_name: &str,
        pagination: PaginationOptions,
    ) -> DbResult<Vec<DbRevisionModel>> {
        
        let repo = self.sql_get_repo(org_name, repo_name).await?;
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

    #[instrument(level = "debug", skip(self))]
    async fn sql_get_revision(
        &self,
        revision_param: &RevisionParam<'_>,
    ) -> DbResult<entity::repository_revision::Model> {
        let condition = Condition::all()
            .add(entity::organization::Column::OrgName.eq(revision_param.org_name.clone()))
            .add(entity::repository::Column::RepoName.eq(revision_param.repo_name.clone()))
            .add(entity::repository_revision::Column::RevisionName.eq(revision_param.revision.clone()));
        
        let revision = RepositoryRevision::find()
            .filter(condition)
            .join(JoinType::Join, entity::repository_revision::Relation::Repository.def())
            .join(JoinType::Join, entity::repository::Relation::Organization.def())
            .one(&self.db)
            .await?;
            
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
    use crate::database::{DateTimeProvider, common_tests::*};

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_revison_create() {
        // let _logger = logging_setup();
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

      
        db.create_org("foo".to_owned()).await.unwrap();
        db.create_repo("foo", "bar").await.unwrap();
        
        let revision = db.create_revision(
            &RevisionParam {
                org_name: "foo",
                repo_name: "bar",
                revision: "1.2.3"
            },
            &CreateRevisionParam {
                scm_id: "1",
                artifact_url: None,
                labels: RevisionLabels::default(),
            }
        ).await.unwrap();

        assert_eq!(revision.revision_name, "1.2.3");
        assert_eq!(revision.scm_id, "1");
        assert_eq!(revision.artifact_url, None);
    }
}
