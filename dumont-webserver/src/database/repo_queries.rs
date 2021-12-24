use super::org_queries::OrganizationQueries;
use super::prelude::*;
use crate::backend::models::PaginationOptions;
use crate::database::entity::{self, prelude::*};
use async_trait::async_trait;
use sea_orm::{entity::*, query::*};
use tracing_attributes::instrument;

#[async_trait]
pub trait RepoQueries {
    async fn create_repo(&self, org_name: &str, repo_name: &str) -> DbResult<DbRepoModel>;

    async fn update_repo_metadata(
        &self,
        org_name: &str,
        repo_name: &str,
        new_metadata: UpdateRepoMetadata,
    ) -> DbResult<DbRepoModel>;

    async fn get_repo_metadata(&self, org_name: &str, repo_name: &str) -> DbResult<RepoMetadata>;
    async fn sql_get_repo_metadata(
        &self,
        repo: &entity::repository::Model,
    ) -> DbResult<entity::repository_metadata::Model>;

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
            ..Default::default()
        };

        let res: InsertResult<repository::ActiveModel> =
            Repository::insert(model).exec(&self.db).await?;
        self.get_repo_by_id(res.last_insert_id).await
    }

    #[instrument(level = "debug", skip(repo, self))]
    async fn sql_get_repo_metadata(
        &self,
        repo: &entity::repository::Model,
    ) -> DbResult<entity::repository_metadata::Model> {
        let metadata_resp = repo.find_related(RepositoryMetadata).one(&self.db).await?;

        match metadata_resp {
            Some(metadata) => Ok(metadata),
            None => {
                let new_model = entity::repository_metadata::ActiveModel {
                    repository_metadata_id: Unset(None),
                    repo_id: Set(repo.repo_id),
                    repo_url: Unset(None),
                };

                let model = new_model.save(&self.db).await?;
                Ok(
                    RepositoryMetadata::find_by_id(model.repository_metadata_id.unwrap())
                        .one(&self.db)
                        .await?
                        .expect("ID provided should be avaliable"),
                )
            }
        }
    }

    async fn get_repo_metadata(&self, org_name: &str, repo_name: &str) -> DbResult<RepoMetadata> {
        let repo = self
            .sql_get_repo(&org_name.to_string(), &repo_name.to_string())
            .await?;
        let metadata = self.sql_get_repo_metadata(&repo).await?;

        Ok(metadata.into())
    }

    #[instrument(level = "debug", skip(self))]
    async fn update_repo_metadata(
        &self,
        org_name: &str,
        repo_name: &str,
        new_metadata: UpdateRepoMetadata,
    ) -> DbResult<DbRepoModel> {
        let repo = self
            .sql_get_repo(&org_name.to_string(), &repo_name.to_string())
            .await?;
        let metadata = self.sql_get_repo_metadata(&repo).await?;
        let mut metadata: entity::repository_metadata::ActiveModel = metadata.into();

        if let Some(repo_url) = new_metadata.repo_url {
            metadata.repo_url = Set(Some(repo_url));
        }

        metadata.save(&self.db).await?;
        self.get_repo_by_id(repo.repo_id).await
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
        let metadata = self.sql_get_repo_metadata(&found_repo).await?;

        Ok(DbRepoModel::from(&found_org, &found_repo, &metadata))
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_repo(&self, org_name: &str, repo_name: &str) -> DbResult<DbRepoModel> {
        let repo = self.sql_get_repo(org_name, repo_name).await?;
        let org = repo
            .find_related(Organization)
            .one(&self.db)
            .await?
            .expect("Org to exist for repo");
        let metadata = self.sql_get_repo_metadata(&repo).await?;

        Ok(DbRepoModel::from(&org, &repo, &metadata))
    }

    #[instrument(level = "debug", skip(self))]
    async fn list_repo(
        &self,
        org_name: &str,
        pagination: PaginationOptions,
    ) -> DbResult<Vec<DbRepoModel>> {
        use entity::repository::Column;

        let org = self.sql_get_org(org_name).await?;
        let resp = org
            .find_related(Repository)
            .find_also_related(RepositoryMetadata)
            .order_by_asc(Column::OrgId)
            .paginate(&self.db, pagination.page_size)
            .fetch_page(pagination.page_number)
            .await?;

        Ok(resp
            .iter()
            .map(|(repo, metadata)| DbRepoModel::from_optional_meta(&org, repo, metadata))
            .collect())
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
    use crate::database::common_tests::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_repos() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
        };

        db.create_org("foo".to_owned()).await.unwrap();
        let repo = db.create_repo("foo", "bar").await.unwrap();

        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org_name, "foo");

        let metadata = db.get_repo_metadata("foo", "bar").await.unwrap();
        assert_eq!(metadata.repo_url, None);

        let metadata = db
            .update_repo_metadata(
                "foo",
                "bar",
                UpdateRepoMetadata {
                    repo_url: Some("https://google.com".to_owned()),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            metadata.metadata.repo_url,
            Some("https://google.com".to_owned())
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
