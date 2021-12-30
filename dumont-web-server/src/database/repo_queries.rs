use crate::backend::models::PaginationOptions;
use crate::database::{
    entity::{self, prelude::*},
    org_queries::OrganizationQueries,
    repo_label_queries::RepoLabelQueries,
    AlreadyExistsError, DatabaseError, DbResult, NotFoundError, PostgresDatabase,
};
use async_trait::async_trait;
use futures_util::future::join_all;
use futures_util::future::TryFutureExt;
use sea_orm::{entity::*, query::*};
use tracing::info;
use tracing_attributes::instrument;

pub trait DbRepo {
    fn get_repo_id(&self) -> i32;
    fn get_repo_name(&self) -> String;
}

pub mod models {
    use crate::database::{entity, org_queries::DbOrganization, prelude::RepoLabels};

    #[derive(Debug, PartialEq, Eq)]
    pub struct DbRepoModel {
        pub org_id: i32,
        pub org_name: String,
        pub repo_id: i32,
        pub repo_name: String,
        pub labels: RepoLabels,
    }

    impl DbRepoModel {
        pub fn from(
            org: &entity::organization::Model,
            repo: &entity::repository::Model,
            labels: &[entity::repository_label::Model],
        ) -> Self {
            Self {
                org_id: org.org_id,
                org_name: org.org_name.clone(),
                repo_id: repo.repo_id,
                repo_name: repo.repo_name.clone(),
                labels: labels.into(),
            }
        }
    }

    impl super::DbRepo for DbRepoModel {
        fn get_repo_id(&self) -> i32 {
            self.repo_id
        }

        fn get_repo_name(&self) -> String {
            self.repo_name.clone()
        }
    }

    impl DbOrganization for DbRepoModel {
        fn get_org_id(&self) -> i32 {
            self.org_id
        }
        fn get_org_name(&self) -> String {
            self.org_name.clone()
        }
    }

    #[derive(Debug)]
    pub struct RepoParam<'a> {
        pub org_name: &'a str,
        pub repo_name: &'a str,
    }

    impl<'a> RepoParam<'a> {
        pub fn new(org: &'a str, repo: &'a str) -> Self {
            Self {
                org_name: org,
                repo_name: repo,
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct CreateRepoParam {
        pub labels: RepoLabels,
    }
}

pub use models::*;

/**
 * RepoQueries is a collection of api calls against the database focused
 * on the "repo".
 *
 * In general this is a CRUD API, with access to some lower level API's for other
 * traits to use to use the ORM.
 */
#[async_trait]
pub trait RepoQueries {
    async fn create_repo(
        &self,
        repo: &RepoParam<'_>,
        create_params: CreateRepoParam,
    ) -> DbResult<DbRepoModel>;

    async fn get_repo(&self, repo: &RepoParam<'_>) -> DbResult<DbRepoModel>;
    async fn sql_get_raw_repo(
        &self,
        org_name: &str,
        repo_name: &str,
    ) -> DbResult<Option<entity::repository::Model>>;

    async fn sql_get_repo(
        &self,
        org_name: &str,
        repo_name: &str,
    ) -> DbResult<entity::repository::Model>;

    async fn get_repo_by_id(&self, repo_id: i32) -> DbResult<DbRepoModel>;

    async fn list_repo(
        &self,
        org_name: &str,
        pagination: PaginationOptions,
    ) -> DbResult<Vec<DbRepoModel>>;

    async fn delete_repo(&self, repo: &RepoParam<'_>) -> DbResult<bool>;
}

#[async_trait]
impl RepoQueries for PostgresDatabase {
    #[instrument(skip(self))]
    async fn create_repo(
        &self,
        repo_param: &RepoParam<'_>,
        create_params: CreateRepoParam,
    ) -> DbResult<DbRepoModel> {
        use entity::repository;

        let repo_name = repo_param.repo_name.to_string();
        let org_name = repo_param.org_name.to_string();

        if let Some(found_repo) = self.sql_get_raw_repo(&org_name, &repo_name).await? {
            info!(
                repo = tracing::field::debug(&found_repo),
                "Found existing repo for {}/{}.", org_name, repo_name
            );
            return Err(DatabaseError::AlreadyExists {
                error: AlreadyExistsError::Repo {
                    org: org_name.clone(),
                    repo: repo_name.clone(),
                },
            });
        }

        let org = self.sql_get_org(&org_name).await?;

        let model = repository::ActiveModel {
            org_id: Set(org.org_id),
            repo_name: Set(repo_name),
            created_at: Set(self.date_time_provider.now().naive_utc()),
            ..Default::default()
        };

        let res: InsertResult<repository::ActiveModel> =
            Repository::insert(model).exec(&self.db).await?;
        self.sql_set_repo_labels(res.last_insert_id, &create_params.labels)
            .await?;
        self.get_repo_by_id(res.last_insert_id).await
    }

    #[instrument(skip(self))]
    async fn get_repo_by_id(&self, repo_id: i32) -> DbResult<DbRepoModel> {
        let found_repo = Repository::find_by_id(repo_id).one(&self.db).await?;
        let found_repo = match found_repo {
            Some(repo) => repo,
            None => {
                return Err(DatabaseError::NotFound {
                    error: NotFoundError::RepoById { repo_id },
                });
            }
        };

        let found_org = found_repo
            .find_related(Organization)
            .one(&self.db)
            .await?
            .expect("Org to exist for repo");
        let labels = self.sql_get_repo_labels(&found_repo).await?;

        Ok(DbRepoModel::from(&found_org, &found_repo, &labels))
    }

    #[instrument(skip(self))]
    async fn get_repo(&self, repo_param: &RepoParam<'_>) -> DbResult<DbRepoModel> {
        let repo = self
            .sql_get_repo(repo_param.org_name, repo_param.repo_name)
            .await?;
        let org = repo
            .find_related(Organization)
            .one(&self.db)
            .await?
            .expect("Org to exist for repo");
        let labels = self.sql_get_repo_labels(&repo).await?;

        Ok(DbRepoModel::from(&org, &repo, &labels))
    }

    #[instrument(skip(self))]
    async fn list_repo(
        &self,
        org_name: &str,
        pagination: PaginationOptions,
    ) -> DbResult<Vec<DbRepoModel>> {
        use entity::repository::Column;

        let org = self.sql_get_org(org_name).await?;
        let select = org
            .find_related(Repository)
            .order_by_asc(Column::OrgId)
            .paginate(&self.db, pagination.page_size)
            .fetch_page(pagination.page_number)
            .await?;

        let mut future_repos = Vec::new();
        for repo in select {
            let repo = repo.clone();
            let org = org.clone();
            future_repos.push(
                self.sql_get_repo_labels_by_repo_id(repo.repo_id)
                    .map_ok(move |labels| DbRepoModel::from(&org, &repo, &labels)),
            );
        }

        let resolved_futures = join_all(future_repos).await;

        let mut repos = Vec::new();
        for future in resolved_futures {
            repos.push(future?);
        }

        Ok(repos)
    }

    #[instrument(skip(self))]
    async fn delete_repo(&self, repo_param: &RepoParam<'_>) -> DbResult<bool> {
        let repo = self
            .sql_get_repo(repo_param.org_name, repo_param.repo_name)
            .await?;
        let repo: entity::repository::ActiveModel = repo.into();
        let res = repo.delete(&self.db).await?;

        if res.rows_affected == 0 {
            return Err(DatabaseError::NotFound {
                error: NotFoundError::Repo {
                    org: repo_param.org_name.to_owned(),
                    repo: repo_param.repo_name.to_owned(),
                },
            });
        }

        Ok(res.rows_affected == 1)
    }

    async fn sql_get_raw_repo(
        &self,
        org_name: &str,
        repo_name: &str,
    ) -> DbResult<Option<entity::repository::Model>> {
        use entity::repository::Column;

        let org = self.sql_get_org(org_name).await?;

        let condition = Condition::all()
            .add(Column::RepoName.eq(repo_name))
            .add(Column::OrgId.eq(org.org_id));
        let resp = Repository::find().filter(condition).one(&self.db).await?;
        Ok(resp)
    }

    async fn sql_get_repo(
        &self,
        org_name: &str,
        repo_name: &str,
    ) -> DbResult<entity::repository::Model> {
        let resp = self.sql_get_raw_repo(org_name, repo_name).await?;
        match resp {
            None => {
                return Err(DatabaseError::NotFound {
                    error: NotFoundError::Repo {
                        org: org_name.to_string(),
                        repo: repo_name.to_string(),
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
    use crate::database::DateTimeProvider;
    use crate::test_utils::*;
    use serial_test::serial;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_repos() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        db.create_org("foo").await.unwrap();
        let repo = db
            .create_repo(
                &RepoParam {
                    org_name: "foo",
                    repo_name: "bar",
                },
                CreateRepoParam::default(),
            )
            .await
            .unwrap();

        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org_name, "foo");

        let repo = db.get_repo_by_id(repo.repo_id).await.unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org_name, "foo");

        let repo = db
            .get_repo(&RepoParam {
                org_name: "foo",
                repo_name: "bar",
            })
            .await
            .unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org_name, "foo");

        let repo_not_found = db
            .get_repo(&RepoParam {
                org_name: "foo",
                repo_name: "flig",
            })
            .await;
        assert_eq!(
            repo_not_found.unwrap_err().to_string(),
            "Repo foo/flig not found"
        );

        db.create_repo(
            &RepoParam {
                org_name: "foo",
                repo_name: "flig",
            },
            CreateRepoParam::default(),
        )
        .await
        .unwrap();

        let found_repos = db
            .list_repo("foo", PaginationOptions::new(0, 50))
            .await
            .unwrap();
        assert_eq!(found_repos.len(), 2);
        assert_eq!(found_repos[0].repo_name, "bar");
        assert_eq!(found_repos[1].repo_name, "flig");

        assert_eq!(
            db.list_repo("foo", PaginationOptions::new(1, 50))
                .await
                .unwrap()
                .len(),
            0
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_repo_pagination() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        db.create_org("foo").await.unwrap();

        for i in 0..100 {
            db.create_repo(
                &RepoParam {
                    org_name: "foo",
                    repo_name: &format!("repo-{}", i),
                },
                CreateRepoParam::default(),
            )
            .await
            .unwrap();
        }

        let found_repos = db
            .list_repo("foo", PaginationOptions::new(0, 50))
            .await
            .unwrap();
        assert_eq!(found_repos.len(), 50);

        for i in 0..50 {
            assert_eq!(found_repos[i].repo_name, format!("repo-{}", i));
        }

        let found_repos = db
            .list_repo("foo", PaginationOptions::new(1, 50))
            .await
            .unwrap();
        assert_eq!(found_repos.len(), 50);

        for i in 0..50 {
            assert_eq!(found_repos[i].repo_name, format!("repo-{}", i + 50));
        }

        let found_repos = db.list_orgs(PaginationOptions::new(2, 50)).await.unwrap();
        assert_eq!(found_repos.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_delete_repo() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        db.create_org("foo").await.unwrap();

        for i in 0..100 {
            db.create_repo(
                &RepoParam {
                    org_name: "foo",
                    repo_name: &format!("repo-{}", i),
                },
                CreateRepoParam::default(),
            )
            .await
            .unwrap();
        }

        for i in 0..100 {
            db.delete_repo(&RepoParam {
                org_name: "foo",
                repo_name: &format!("repo-{}", i),
            })
            .await
            .unwrap();
        }

        let found_repos = db
            .list_repo("foo", PaginationOptions::new(0, 50))
            .await
            .unwrap();
        assert_eq!(found_repos.len(), 0);
    }
}
