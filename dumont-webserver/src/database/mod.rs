use async_trait::async_trait;

// Generated with `sea-orm-cli generate entity -s public -o src/database/entity`
mod entity;
mod models;

use entity::prelude::*;
use sea_orm::{entity::*, query::*, Database, DatabaseConnection};
use thiserror::Error;

use self::models::*;

type DbResult<T> = Result<T, DatabaseError>;

#[derive(Error, Debug)]
pub enum NotFoundError {
    #[error("{org} not found")]
    Organization { org: String },
    #[error("{org}/{repo} not found")]
    Repo { org: String, repo: String },
}

#[derive(Error, Debug)]
pub enum AlreadyExistsError {
    #[error("{org} exists")]
    Organization { org: String },
    #[error("{org}/{repo} exists")]
    Repo { org: String, repo: String },
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error(transparent)]
    NotFound { error: NotFoundError },
    #[error(transparent)]
    AlreadyExists { error: AlreadyExistsError },
    #[error(transparent)]
    BackendError {
        #[from]
        source: anyhow::Error,
    },
    #[error(transparent)]
    SeaOrmError {
        #[from]
        source: sea_orm::DbErr,
    },
}

pub struct PostresDatabase {
    db: DatabaseConnection,
}

impl PostresDatabase {
    pub async fn new<S: Into<String>>(connection_url: S) -> DbResult<Self> {
        let db: DatabaseConnection = Database::connect(&connection_url.into()).await?;

        Ok(Self { db })
    }
}

#[async_trait]
trait OrganizationQueries {
    async fn create_org(&self, org_name: String) -> DbResult<DbOrganization>;
    async fn find_org(&self, org_name: String) -> DbResult<DbOrganization>;
    async fn list_orgs(&self, page_number: usize, limit: usize) -> DbResult<Vec<DbOrganization>>;
    async fn delete_org(&self, org: DbOrganization) -> DbResult<()>;
}

#[async_trait]
impl OrganizationQueries for PostresDatabase {
    async fn create_org(&self, org_name: String) -> DbResult<DbOrganization> {
        use entity::organization::{ActiveModel, Column};

        let resp = Organization::find()
            .filter(Column::OrgName.eq(org_name.clone()))
            .count(&self.db)
            .await?;
        if resp != 0 {
            return Err(DatabaseError::AlreadyExists { error: AlreadyExistsError::Organization {
                org: org_name.clone(),
            }});
        }

        let model = ActiveModel {
            org_name: Set(org_name),
            ..Default::default()
        };

        let res: InsertResult<ActiveModel> = Organization::insert(model).exec(&self.db).await?;
        let model = Organization::find_by_id(res.last_insert_id)
            .one(&self.db)
            .await?
            .unwrap();
        Ok(DbOrganization::from(model))
    }

    async fn delete_org(&self, org: DbOrganization) -> DbResult<()> {
        use entity::organization::ActiveModel;

        let resp = Organization::find_by_id(org.org_id).one(&self.db).await?;
        let org: ActiveModel = match resp {
            Some(org) => org.into(),
            None => return Err(DatabaseError::NotFound { error: NotFoundError::Organization { org: org.org_name }}),
        };

        org.delete(&self.db).await?;

        Ok(())
    }

    async fn find_org(&self, org_name: String) -> DbResult<DbOrganization> {
        use entity::organization::Column;

        let resp = Organization::find()
            .filter(Column::OrgName.eq(org_name.clone()))
            .one(&self.db)
            .await?;

        let org = match resp {
            Some(org) => org,
            None => return Err(DatabaseError::NotFound { error: NotFoundError::Organization { org: org_name }}),
        };

        Ok(DbOrganization::from(org))
    }

    async fn list_orgs(&self, page_number: usize, limit: usize) -> DbResult<Vec<DbOrganization>> {
        use entity::organization::Column;

        let resp = Organization::find()
            .order_by_asc(Column::OrgId)
            .paginate(&self.db, limit)
            .fetch_page(page_number)
            .await?;

        Ok(resp.iter().map(DbOrganization::from).collect())
    }
}

impl From<&entity::organization::Model> for DbOrganization {
    fn from(org: &entity::organization::Model) -> Self {
        Self {
            org_id: org.org_id,
            org_name: org.org_name.clone(),
        }
    }
}

#[async_trait]
trait RepoQueries {
    async fn create_repo(&self, org: &DbOrganization, repo_name: String) -> DbResult<DbRepo>;
    async fn update_repo_settings(
        &self,
        repo: DbRepo,
        update_settings: UpdateRepoSetting,
    ) -> DbResult<DbRepo>;
    async fn find_repo(&self, org: &DbOrganization, repo_name: String) -> DbResult<DbRepo>;
    async fn get_repo_by_id(&self, org: &DbOrganization, repo_id: i32) -> DbResult<DbRepo>;
    async fn list_repo(
        &self,
        org: &DbOrganization,
        page_number: usize,
        limit: usize,
    ) -> DbResult<Vec<DbRepo>>;
    async fn delete_repo(&self, repo: DbRepo) -> DbResult<()>;
}

#[async_trait]
impl RepoQueries for PostresDatabase {
    async fn create_repo(&self, org: &DbOrganization, repo_name: String) -> DbResult<DbRepo> {
        use entity::repository::{ActiveModel, Column};

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
            version_strategy: Set(VersionScheme::Semver.to_string()),
            ..Default::default()
        };

        let res: InsertResult<ActiveModel> = Repository::insert(model).exec(&self.db).await?;
        self.get_repo_by_id(&org, res.last_insert_id).await
    }

    async fn update_repo_settings(
        &self,
        db_repo: DbRepo,
        update_settings: UpdateRepoSetting,
    ) -> DbResult<DbRepo> {
        use entity::repository::ActiveModel;

        let resp = Repository::find_by_id(db_repo.repo_id)
            .one(&self.db)
            .await?;
        let mut repo: ActiveModel = match resp {
            Some(repo) => repo.into(),
            None => {
                return Err(DatabaseError::NotFound { error: NotFoundError::Repo {
                    org: db_repo.org.org_name.clone(),
                    repo: db_repo.repo_name
                }});
            }
        };

        if let Some(version_schema) = update_settings.version_scheme {
            repo.version_strategy = Set(version_schema.to_string());
        }

        repo.update(&self.db).await?;

        self.get_repo_by_id(&db_repo.org, db_repo.repo_id).await
    }

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

    async fn find_repo(&self, org: &DbOrganization, repo_name: String) -> DbResult<DbRepo> {
        use entity::repository::{Column};
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

    async fn list_repo(
        &self,
        org: &DbOrganization,
        page_number: usize,
        limit: usize,
    ) -> DbResult<Vec<DbRepo>> {
        use entity::repository::Column;

        let condition = Condition::all().add(Column::OrgId.eq(org.org_id));

        let resp = Repository::find()
            .filter(condition)
            .order_by_asc(Column::OrgId)
            .paginate(&self.db, limit)
            .fetch_page(page_number)
            .await?;

        Ok(resp.iter().map(|it| DbRepo::from(org, it)).collect())
    }

    async fn delete_repo(&self, repo: DbRepo) -> DbResult<()> {
        unimplemented!();
    }
}

#[cfg(test)]
mod integ_test {
    use super::*;

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
        assert_eq!(repo.version_schema, VersionScheme::Semver);

        let repo = db
            .update_repo_settings(
                repo,
                UpdateRepoSetting {
                    version_scheme: Some(VersionScheme::Serial),
                },
            )
            .await
            .unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org, new_org);
        assert_eq!(repo.version_schema, VersionScheme::Serial);

        let repo = db.get_repo_by_id(&new_org, repo.repo_id).await.unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org, new_org);
        assert_eq!(repo.version_schema, VersionScheme::Serial);

        let repo = db.find_repo(&new_org, "bar".to_owned()).await.unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org, new_org);
        assert_eq!(repo.version_schema, VersionScheme::Serial);

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

        let found_repos = db.list_repo(&new_org, 0, 50).await.unwrap();
        assert_eq!(found_repos.len(), 2);
        assert_eq!(found_repos[0].repo_name, "bar");
        assert_eq!(found_repos[1].repo_name, "flig");

        assert_eq!(db.list_repo(&new_org, 1, 50).await.unwrap().len(), 0);
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

        let found_repos = db.list_repo(&org, 0, 50).await.unwrap();
        assert_eq!(found_repos.len(), 50);
        
        for i in 0..50 {
            assert_eq!(found_repos[i].repo_name, format!("repo-{}", i));
        }

        let found_repos = db.list_repo(&org, 1, 50).await.unwrap();
        assert_eq!(found_repos.len(), 50);
        
        for i in 0..50 {
            assert_eq!(found_repos[i].repo_name, format!("repo-{}", i+50));
        }

        let found_repos = db.list_orgs(2, 50).await.unwrap();
        assert_eq!(found_repos.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_orgs() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
        };

        let new_org = db.create_org("foo".to_owned()).await.unwrap();
        assert_eq!(new_org.org_name, "foo");

        let found_org = db.find_org("foo".to_owned()).await.unwrap();
        assert_eq!(found_org.org_name, "foo");

        match db.find_org("food".to_owned()).await {
            Err(DatabaseError::NotFound { error: NotFoundError::Organization { org }}) => assert_eq!(org, "food".to_owned()),
            failed => unreachable!("Should not have gotten {:?}", failed),
        }

        match db.create_org("foo".to_owned()).await {
            Err(DatabaseError::AlreadyExists { error: AlreadyExistsError::Organization { org } }) => {
                assert_eq!(org, "foo");
            }
            failed => unreachable!("Should not have gotten {:?}", failed),
        }

        let new_org = db.create_org("bar".to_owned()).await.unwrap();
        assert_eq!(new_org.org_name, "bar");

        let listed_orgs = db.list_orgs(0, 50).await.unwrap();
        assert_eq!(listed_orgs.len(), 2);
        assert_eq!(listed_orgs[0].org_name, "foo");
        assert_eq!(listed_orgs[1].org_name, "bar");

        // Get from page that doesn't exist
        assert_eq!(db.list_orgs(1, 50).await.unwrap().len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_org_pagination() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
        };

        for i in 0..100 {
            db.create_org(format!("org-{}", i)).await.unwrap();
        }

        let found_orgs = db.list_orgs(0, 50).await.unwrap();
        assert_eq!(found_orgs.len(), 50);
        
        for i in 0..50 {
            assert_eq!(found_orgs[i].org_name, format!("org-{}", i));
        }

        let found_orgs = db.list_orgs(1, 50).await.unwrap();
        assert_eq!(found_orgs.len(), 50);
        
        for i in 0..50 {
            assert_eq!(found_orgs[i].org_name, format!("org-{}", i+50));
        }

        let found_orgs = db.list_orgs(2, 50).await.unwrap();
        assert_eq!(found_orgs.len(), 0);
    }

    async fn setup_schema() -> DbResult<DatabaseConnection> {
        use super::entity::prelude::*;
        use sea_orm::schema::Schema;

        let db = Database::connect("sqlite::memory:").await?;

        db.execute(
            db.get_database_backend()
                .build(&Schema::create_table_from_entity(Organization)),
        )
        .await?;

        db.execute(
            db.get_database_backend()
                .build(&Schema::create_table_from_entity(Repository)),
        )
        .await?;

        Ok(db)
    }
}
