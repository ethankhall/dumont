use super::org_queries::OrganizationQueries;
use super::prelude::*;
use crate::backend::models::PaginationOptions;
use crate::database::entity::{self, prelude::*};
use async_trait::async_trait;
use sea_orm::{entity::*, query::*};
use std::collections::BTreeMap;
use tracing::info;
use tracing_attributes::instrument;


pub trait DbRepo {
    fn get_repo_id(&self) -> i32;
    fn get_repo_name(&self) -> String;
}

pub mod models {
    use std::collections::BTreeMap;
    use crate::database::entity;

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
            labels: &Vec<entity::repository_label::Model>,
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

    impl super::DbOrganization for DbRepoModel {
        fn get_org_id(&self) -> i32 {
            self.org_id
        }
        fn get_org_name(&self) -> String {
            self.org_name.clone()
        }
    }


    #[derive(Debug, PartialEq, Eq)]
    pub struct RepoLabels {
        pub labels: BTreeMap<String, String>,
    }

    impl From<&Vec<entity::repository_label::Model>> for RepoLabels {
        fn from(source: &Vec<entity::repository_label::Model>) -> Self {
            let mut labels: BTreeMap<String, String> = Default::default();
            for value in source.iter() {
                labels.insert(value.label_name.to_string(), value.label_value.to_string());
            }

            Self { labels }
        }
    }

    impl From<Vec<entity::repository_label::Model>> for RepoLabels {
        fn from(source: Vec<entity::repository_label::Model>) -> Self {
            (&source).into()
        }
    }

    impl Default for RepoLabels {
        fn default() -> Self {
            Self {
                labels: Default::default(),
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct UpdateRepoMetadata {
        pub labels: BTreeMap<String, String>,
    }
}

pub use models::*;

#[async_trait]
pub trait RepoQueries {
    async fn create_repo(&self, org_name: &str, repo_name: &str) -> DbResult<DbRepoModel>;

    async fn set_repo_labels(
        &self,
        org_name: &str,
        repo_name: &str,
        labels: BTreeMap<String, String>,
    ) -> DbResult<()>;

    async fn get_repo_labels(&self, org_name: &str, repo_name: &str) -> DbResult<RepoLabels>;
    async fn sql_get_repo_labels(
        &self,
        repo: &entity::repository::Model,
    ) -> DbResult<Vec<entity::repository_label::Model>>;

    async fn get_repo(&self, org_name: &str, repo_name: &str) -> DbResult<DbRepoModel>;
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

    async fn delete_repo(&self, oorg_name: &str, repo_name: &str) -> DbResult<bool>;
}

#[async_trait]
impl RepoQueries for PostresDatabase {
    #[instrument(level = "debug", skip(self))]
    async fn create_repo(&self, org_name: &str, repo_name: &str) -> DbResult<DbRepoModel> {
        use entity::repository;

        let repo_name = repo_name.to_string();
        let org_name = org_name.to_string();

        let org = self.sql_get_org(&org_name).await?;

        let condition = Condition::all()
            .add(repository::Column::RepoName.eq(repo_name.clone()))
            .add(repository::Column::OrgId.eq(org.org_id));
        let resp = Repository::find().filter(condition).count(&self.db).await?;
        if resp != 0 {
            return Err(DatabaseError::AlreadyExists {
                error: AlreadyExistsError::Repo {
                    org: org.org_name.clone(),
                    repo: repo_name,
                },
            });
        }

        let model = repository::ActiveModel {
            org_id: Set(org.org_id),
            repo_name: Set(repo_name),
            created_at: Set(self.date_time_provider.now().naive_utc()),
            ..Default::default()
        };

        let res: InsertResult<repository::ActiveModel> =
            Repository::insert(model).exec(&self.db).await?;
        self.get_repo_by_id(res.last_insert_id).await
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_repo_labels(&self, org_name: &str, repo_name: &str) -> DbResult<RepoLabels> {
        let repo = self
            .sql_get_repo(&org_name.to_string(), &repo_name.to_string())
            .await?;

        let labels = self.sql_get_repo_labels(&repo).await?;
        Ok(labels.into())
    }

    #[instrument(level = "debug", skip(self))]
    async fn set_repo_labels(
        &self,
        org_name: &str,
        repo_name: &str,
        labels: BTreeMap<String, String>,
    ) -> DbResult<()> {
        let repo = self
            .sql_get_repo(&org_name.to_string(), &repo_name.to_string())
            .await?;

        let mut new_labels = Vec::default();

        for (key, value) in labels {
            new_labels.push(entity::repository_label::ActiveModel {
                repo_id: Set(repo.repo_id),
                label_name: Set(key.to_string()),
                label_value: Set(value.to_string()),
                created_at: Set(self.date_time_provider.now().naive_utc()),
                ..Default::default()
            })
        }

        let new_label_count = new_labels.len();

        let txn = self.db.begin().await?;

        let del = RepositoryLabel::delete_many()
            .filter(entity::repository_label::Column::RepoId.eq(repo.repo_id))
            .exec(&txn)
            .await?;
        if !new_labels.is_empty() {
            RepositoryLabel::insert_many(new_labels).exec(&txn).await?;
        }

        txn.commit().await?;

        info!(
            "Deleted {} rows, Inserted {} rows",
            del.rows_affected, new_label_count
        );
        Ok(())
    }

    #[instrument(level = "debug", skip(repo, self))]
    async fn sql_get_repo_labels(
        &self,
        repo: &entity::repository::Model,
    ) -> DbResult<Vec<entity::repository_label::Model>> {
        Ok(repo.find_related(RepositoryLabel).all(&self.db).await?)
    }

    #[instrument(level = "debug", skip(self))]
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

    #[instrument(level = "debug", skip(self))]
    async fn get_repo(&self, org_name: &str, repo_name: &str) -> DbResult<DbRepoModel> {
        let repo = self.sql_get_repo(org_name, repo_name).await?;
        let org = repo
            .find_related(Organization)
            .one(&self.db)
            .await?
            .expect("Org to exist for repo");
        let labels = self.sql_get_repo_labels(&repo).await?;

        Ok(DbRepoModel::from(&org, &repo, &labels))
    }

    #[instrument(level = "debug", skip(self))]
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

        let mut repos = Vec::new();
        for repo in select {
            let labels = self.sql_get_repo_labels(&repo).await?;
            repos.push(DbRepoModel::from(&org, &repo, &labels));
        }

        Ok(repos)
    }

    #[instrument(level = "debug", skip(self))]
    async fn delete_repo(&self, org_name: &str, repo_name: &str) -> DbResult<bool> {
        let repo = self.sql_get_repo(org_name, repo_name).await?;
        let repo: entity::repository::ActiveModel = repo.into();
        let res = repo.delete(&self.db).await?;

        if res.rows_affected == 0 {
            return Err(DatabaseError::NotFound {
                error: NotFoundError::Repo {
                    org: org_name.to_owned(),
                    repo: repo_name.to_owned(),
                },
            });
        }

        Ok(res.rows_affected == 1)
    }

    async fn sql_get_repo(
        &self,
        org_name: &str,
        repo_name: &str,
    ) -> DbResult<entity::repository::Model> {
        use entity::repository::Column;

        let org = self.sql_get_org(org_name).await?;

        let condition = Condition::all()
            .add(Column::RepoName.eq(repo_name))
            .add(Column::OrgId.eq(org.org_id));
        let resp = Repository::find().filter(condition).one(&self.db).await?;
        match resp {
            None => {
                return Err(DatabaseError::NotFound {
                    error: NotFoundError::Repo {
                        org: org.org_name.clone(),
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
    use crate::database::{DateTimeProvider, common_tests::*};

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_repos() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        db.create_org("foo".to_owned()).await.unwrap();
        let repo = db.create_repo("foo", "bar").await.unwrap();

        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org_name, "foo");

        let metadata = db.get_repo_labels("foo", "bar").await.unwrap();
        assert_eq!(metadata.labels.len(), 0);

        let mut labels = BTreeMap::new();
        labels.insert("scm_url".to_owned(), "https://google.com".to_owned());

        db.set_repo_labels("foo", "bar", labels).await.unwrap();

        let labels = db.get_repo_labels("foo", "bar").await.unwrap();
        assert_eq!(
            labels.labels.get("scm_url"),
            Some(&"https://google.com".to_owned())
        );

        let repo = db.get_repo_by_id(repo.repo_id).await.unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org_name, "foo");

        let repo = db.get_repo("foo", "bar").await.unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org_name, "foo");

        match db.get_repo("foo", "flig").await {
            Err(DatabaseError::NotFound {
                error: NotFoundError::Repo { org, repo },
            }) => {
                assert_eq!(org, "foo");
                assert_eq!(repo, "flig");
            }
            failed => unreachable!("Should not have gotten {:?}", failed),
        }

        db.create_repo("foo", "flig").await.unwrap();

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
    async fn test_repo_pagination() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        db.create_org("foo".to_owned()).await.unwrap();

        for i in 0..100 {
            db.create_repo("foo", &format!("repo-{}", i)).await.unwrap();
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
    async fn test_delete_repo() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        db.create_org("foo".to_owned()).await.unwrap();

        for i in 0..100 {
            db.create_repo("foo", &format!("repo-{}", i)).await.unwrap();
        }

        for i in 0..100 {
            db.delete_repo("foo", &format!("repo-{}", i)).await.unwrap();
        }

        let found_repos = db
            .list_repo("foo", PaginationOptions::new(0, 50))
            .await
            .unwrap();
        assert_eq!(found_repos.len(), 0);
    }
}
