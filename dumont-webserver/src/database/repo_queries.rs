use crate::database::entity::{self, prelude::*};
use async_trait::async_trait;
use super::prelude::*;
use super::org_queries::OrganizationQueries;
use tracing_attributes::instrument;
use sea_orm::{entity::*, query::*};
use crate::backend::models::PaginationOptions;

#[async_trait]
pub trait RepoQueries {
    async fn create_repo<T>(&self, org: &DbOrganization, repo_name: T) -> DbResult<DbRepo> where T: ToString + Send;
    async fn update_repo_metadata(
        &self,
        repo: &DbRepo,
        new_metadata: UpdateRepoMetadata,
    ) -> DbResult<RepoMetadata>;
    async fn get_repo_metadata(
        &self,
        repo: &DbRepo
    ) -> DbResult<RepoMetadata>;

    async fn find_repo<T>(&self, org: &DbOrganization, repo_name: T) -> DbResult<DbRepo> where T: ToString + Send;
    async fn get_repo_by_id(&self, org: &DbOrganization, repo_id: i32) -> DbResult<DbRepo>;
    async fn list_repo(
        &self,
        org: &DbOrganization,
        pagination: PaginationOptions,
    ) -> DbResult<Vec<DbRepo>>;
    async fn delete_repo<T>(&self, org: &DbOrganization, repo_name: T) -> DbResult<()> where T: ToString + Send;
}

#[async_trait]
impl RepoQueries for PostresDatabase {
    #[instrument(level = "debug", fields(repo_name = %repo_name.to_string()), skip(self, repo_name))]
    async fn create_repo<T>(&self, org: &DbOrganization, repo_name: T)  -> DbResult<DbRepo> where T: ToString + Send {
        use entity::repository::{ActiveModel, Column};

        let repo_name = repo_name.to_string();

        let condition = Condition::all()
            .add(Column::RepoName.eq(repo_name.clone()))
            .add(Column::OrgId.eq(org.org_id));
        let resp = Repository::find().filter(condition).count(&self.db).await?;
        if resp != 0 {
            return Err(DatabaseError::AlreadyExists { error: AlreadyExistsError::Repo {
                org: org.org_name.clone(),
                repo: repo_name
            }});
        }

        let model = ActiveModel {
            org_id: Set(org.org_id),
            repo_name: Set(repo_name),
            ..Default::default()
        };

        let res: InsertResult<ActiveModel> = Repository::insert(model).exec(&self.db).await?;
        self.get_repo_by_id(&org, res.last_insert_id).await
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_repo_metadata(&self, db_repo: &DbRepo) -> DbResult<RepoMetadata> {
        use entity::repository_metadata::{Column};

        let condition = Condition::all().add(Column::RepoId.eq(db_repo.repo_id));
        let metadata_resp = RepositoryMetadata::find().filter(condition).one(&self.db).await?;

        match metadata_resp {
            Some(metadata) => Ok(RepoMetadata { repo_url: metadata.repo_url }),
            None => {
                Ok(RepoMetadata { repo_url: None })
            }
        }
    }

    #[instrument(level = "debug", skip(self))]
    async fn update_repo_metadata(
        &self,
        db_repo: &DbRepo,
        new_metadata: UpdateRepoMetadata,
    ) -> DbResult<RepoMetadata> {
        use entity::repository_metadata::{Column, ActiveModel};
        let condition = Condition::all().add(Column::RepoId.eq(db_repo.repo_id));
        let metadata = RepositoryMetadata::find().filter(condition).one(&self.db).await?;

        let mut metadata: ActiveModel = match metadata {
            Some(metadata) => metadata.into(),
            None => {
                ActiveModel {
                    repository_metadata_id: Unset(None),
                    repo_id: Set(db_repo.repo_id),
                    repo_url: Unset(None),
                }
            }
        };
    

        if let Some(repo_url) = new_metadata.repo_url {
            metadata.repo_url = Set(Some(repo_url));
        }

        metadata.save(&self.db).await?;
        self.get_repo_metadata(db_repo).await
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_repo_by_id(&self, org: &DbOrganization, repo_id: i32) -> DbResult<DbRepo> {
        let found_repo = Repository::find_by_id(repo_id).one(&self.db).await?;
        let found_repo = match found_repo {
            Some(repo) => repo,
            None => {
                return Err(DatabaseError::NotFound { error: NotFoundError::Repo {
                    org: org.org_name.clone(),
                    repo: repo_id.to_string()
                }});
            }
        };

        Ok(DbRepo::from(&org, &found_repo))
    }

    #[instrument(level = "debug", fields(repo_name = %repo_name.to_string()), skip(self, repo_name))]
    async fn find_repo<T>(&self, org: &DbOrganization, repo_name: T) -> DbResult<DbRepo> where T: ToString + Send {
        use entity::repository::{Column};
        let repo_name = repo_name.to_string();
        let org = self.find_org(org.org_name.clone()).await?;

        let condition = Condition::all()
            .add(Column::RepoName.eq(repo_name.clone()))
            .add(Column::OrgId.eq(org.org_id));
        let resp = Repository::find().filter(condition).one(&self.db).await?;
        let repo = match resp  {
            None => return Err(DatabaseError::NotFound { error: NotFoundError::Repo {
                org: org.org_name.clone(),
                repo: repo_name.clone()
            }}),
            Some(repo) => repo
        };

        Ok(DbRepo::from(&org, &repo))
    }

    #[instrument(level = "debug", skip(self))]
    async fn list_repo(
        &self,
        org: &DbOrganization,
        pagination: PaginationOptions,
    ) -> DbResult<Vec<DbRepo>> {
        use entity::repository::Column;

        let condition = Condition::all().add(Column::OrgId.eq(org.org_id));

        let resp = Repository::find()
            .filter(condition)
            .order_by_asc(Column::OrgId)
            .paginate(&self.db, pagination.page_size)
            .fetch_page(pagination.page_number)
            .await?;

        Ok(resp.iter().map(|it| DbRepo::from(org, it)).collect())
    }


    #[instrument(level = "debug", fields(repo_name = %repo_name.to_string()), skip(self, repo_name))]
    async fn delete_repo<T>(&self, org: &DbOrganization, repo_name: T) -> DbResult<()> where T: ToString + Send {
        use entity::repository::{ActiveModel, Column};
        let repo_name = repo_name.to_string();
        let org = self.find_org(org.org_name.clone()).await?;

        let condition = Condition::all()
            .add(Column::RepoName.eq(repo_name.clone()))
            .add(Column::OrgId.eq(org.org_id));
        let resp = Repository::find().filter(condition).one(&self.db).await?;

        let repo: ActiveModel = match resp  {
            None => return Err(DatabaseError::NotFound { error: NotFoundError::Repo {
                org: org.org_name.clone(),
                repo: repo_name.clone()
            }}),
            Some(repo) => repo.into()
        };

        repo.delete(&self.db).await?;

        Ok(())
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

        let new_org = db.create_org("foo".to_owned()).await.unwrap();
        let repo = db
            .create_repo(&new_org, "bar".to_owned())
            .await
            .unwrap();

        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org, new_org);
        
        let metadata = db.get_repo_metadata(&repo).await.unwrap();
        assert_eq!(metadata.repo_url, None);

        let metadata = db
            .update_repo_metadata(
                &repo,
                UpdateRepoMetadata {
                    repo_url: Some("https://google.com".to_owned()),
                },
            )
            .await
            .unwrap();
        assert_eq!(metadata.repo_url, Some("https://google.com".to_owned()));

        let repo = db.get_repo_by_id(&new_org, repo.repo_id).await.unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org, new_org);

        let repo = db.find_repo(&new_org, "bar".to_owned()).await.unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org, new_org);

        match db.find_repo(&new_org, "flig".to_owned()).await {
            Err(DatabaseError::NotFound { error: NotFoundError::Repo { org, repo } }) => {
                assert_eq!(org, "foo");
                assert_eq!(repo, "flig");
            }
            failed => unreachable!("Should not have gotten {:?}", failed),
        }

        db.create_repo(&new_org, "flig".to_owned())
            .await
            .unwrap();

        let found_repos = db.list_repo(&new_org, PaginationOptions::new(0, 50)).await.unwrap();
        assert_eq!(found_repos.len(), 2);
        assert_eq!(found_repos[0].repo_name, "bar");
        assert_eq!(found_repos[1].repo_name, "flig");

        assert_eq!(db.list_repo(&new_org, PaginationOptions::new(1, 50)).await.unwrap().len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_repo_pagination() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
        };

        let org = db.create_org("foo".to_owned()).await.unwrap();

        for i in 0..100 {
            db.create_repo(&org, format!("repo-{}", i)).await.unwrap();
        }

        let found_repos = db.list_repo(&org, PaginationOptions::new(0, 50)).await.unwrap();
        assert_eq!(found_repos.len(), 50);
        
        for i in 0..50 {
            assert_eq!(found_repos[i].repo_name, format!("repo-{}", i));
        }

        let found_repos = db.list_repo(&org, PaginationOptions::new(1, 50)).await.unwrap();
        assert_eq!(found_repos.len(), 50);
        
        for i in 0..50 {
            assert_eq!(found_repos[i].repo_name, format!("repo-{}", i+50));
        }

        let found_repos = db.list_orgs(PaginationOptions::new(2, 50)).await.unwrap();
        assert_eq!(found_repos.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_delete_repo() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
        };

        let org = db.create_org("foo".to_owned()).await.unwrap();

        for i in 0..100 {
            db.create_repo(&org, format!("repo-{}", i)).await.unwrap();
        }

        for i in 0..100 {
            db.delete_repo(&org, format!("repo-{}", i)).await.unwrap();
        }

        let found_repos = db.list_repo(&org, PaginationOptions::new(0, 50)).await.unwrap();
        assert_eq!(found_repos.len(), 0);
    }
}
