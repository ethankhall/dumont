use async_trait::async_trait;

// Generated with `sea-orm-cli generate entity -s public -o src/database/entity`
mod entity;
mod models;

use thiserror::Error;
use sea_orm::{DatabaseConnection, Database, entity::*, query::*};
use entity::prelude::*;
use strum_macros::{Display};

use self::models::*;

type DbResult<T> = Result<T, DatabaseError>;

#[derive(Debug, PartialEq, Eq, Display)]
pub enum ErrorEntityName {
    Organization
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("{id} not found")]
    NotFound { id: String },
    #[error("Organization already exists with name {org}")]
    OrganizationAlreadyExists {
        org: String,
    },
    #[error("Repository already exists with name {org}/{repo}")]
    RepoAlreadyExists {
        org: String,
        repo: String
    },
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
    db: DatabaseConnection
}

impl PostresDatabase {
    pub async fn new<S: Into<String>>(connection_url: S) -> DbResult<Self> {
        let db: DatabaseConnection = Database::connect(&connection_url.into()).await?;

        Ok(Self { 
            db
        })
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

        let resp = Organization::find().filter(Column::OrgName.eq(org_name.clone())).count(&self.db).await?;
        if resp != 0 {
            return Err(DatabaseError::OrganizationAlreadyExists { org: org_name.clone()})
        }

        let model = ActiveModel {
                org_name: Set(org_name),
                ..Default::default()
        };

        let res: InsertResult<ActiveModel> = Organization::insert(model).exec(&self.db).await?;
        let model = Organization::find_by_id(res.last_insert_id).one(&self.db).await?.unwrap();
        Ok(DbOrganization::from(model))
    }

    async fn delete_org(&self, org: DbOrganization) -> DbResult<()> {
        use entity::organization::{ActiveModel};

        let resp = Organization::find_by_id(org.org_id).one(&self.db).await?;
        let org: ActiveModel = match resp {
            Some(org) => org.into(),
            None => return Err(DatabaseError::NotFound { id: org.org_name })
        };

        org.delete(&self.db).await?;

        Ok(())
    }

    async fn find_org(&self, org_name: String) -> DbResult<DbOrganization> {
        use entity::organization::{Column};

        let resp = Organization::find().filter(Column::OrgName.eq(org_name.clone())).one(&self.db).await?;

        let org = match resp {
            Some(org) => org,
            None => return Err(DatabaseError::NotFound { id: org_name })
        };

        Ok(DbOrganization::from(org))
    }

    async fn list_orgs(&self, page_number: usize, limit: usize) -> DbResult<Vec<DbOrganization>> {
        use entity::organization::{Column};

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
    async fn create_repo(&self, org_name: String, repo_name: String) -> DbResult<DbRepo>;
    async fn update_repo_settings(&self, repo: DbRepo, update_settings: UpdateRepoSetting) -> DbResult<DbRepo>;
    async fn find_repo(&self, org_name: String, repo_name: String) -> DbResult<DbRepo>;
    async fn get_repo_by_id(&self, org: &DbOrganization, repo_id: i32) -> DbResult<DbRepo>;
    async fn list_repo(&self, org_name: String, page_number: usize, limit: usize) -> DbResult<Vec<DbRepo>>;
    async fn delete_repo(&self, repo: DbRepo) -> DbResult<()>;
}

#[async_trait]
impl RepoQueries for PostresDatabase {
    async fn create_repo(&self, org_name: String, repo_name: String) -> DbResult<DbRepo> {
        use entity::repository::{ActiveModel, Column};
        let org = self.find_org(org_name.clone()).await?;

        let  condition = Condition::all().add(Column::RepoName.eq(repo_name.clone())).add(Column::OrgId.eq(org.org_id));
        let resp = Repository::find().filter(condition).count(&self.db).await?;
        if resp != 0 {
            return Err(DatabaseError::RepoAlreadyExists { org: org_name, repo: repo_name.clone()})
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

    async fn update_repo_settings(&self, db_repo: DbRepo, update_settings: UpdateRepoSetting) -> DbResult<DbRepo>{
        use entity::repository::{ActiveModel};

        let resp = Repository::find_by_id(db_repo.repo_id).one(&self.db).await?;
        let mut repo: ActiveModel = match resp {
            Some(repo) => repo.into(),
            None => return Err(DatabaseError::NotFound { id: db_repo.repo_name })
        };

        if let Some(version_schema) = update_settings.version_scheme {
            repo.version_strategy =  Set(version_schema.to_string());
        }

        repo.update(&self.db).await?;

        self.get_repo_by_id(&db_repo.org, db_repo.repo_id).await
    }

    async fn get_repo_by_id(&self, org: &DbOrganization, repo_id: i32) -> DbResult<DbRepo> {
        let found_repo = Repository::find_by_id(repo_id).one(&self.db).await?;
        let found_repo = match found_repo {
            Some(repo) => repo,
            None => return Err(DatabaseError::NotFound { id: repo_id.to_string() })
        };

        Ok(DbRepo::from(org.clone(), found_repo))
    }

    async fn find_repo(&self, org_name: String, repo_name: String) -> DbResult<DbRepo>{
        unimplemented!();
    }

    async fn list_repo(&self, org_name: String, page_number: usize, limit: usize) -> DbResult<Vec<DbRepo>>{
        unimplemented!();
    }

    async fn delete_repo(&self, repo: DbRepo) -> DbResult<()>{
        unimplemented!();
    }
}

#[cfg(test)]
mod integ_test {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_repos() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap()
        };

        let new_org = db.create_org("foo".to_owned()).await.unwrap();
        let repo = db.create_repo("foo".to_owned(), "bar".to_owned()).await.unwrap();

        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org, new_org);
        assert_eq!(repo.version_schema, VersionScheme::Semver);

        let repo = db.update_repo_settings(repo, UpdateRepoSetting { version_scheme: Some(VersionScheme::Serial )}).await.unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org, new_org);
        assert_eq!(repo.version_schema, VersionScheme::Serial);

        let repo = db.get_repo_by_id(&new_org, repo.repo_id).await.unwrap();
        assert_eq!(repo.repo_name, "bar");
        assert_eq!(repo.org, new_org);
        assert_eq!(repo.version_schema, VersionScheme::Serial);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_orgs() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap()
        };

        let new_org = db.create_org("foo".to_owned()).await.unwrap();
        assert_eq!(new_org.org_name, "foo");

        let found_org = db.find_org("foo".to_owned()).await.unwrap();
        assert_eq!(found_org.org_name, "foo");
        
        match db.find_org("food".to_owned()).await {
            Err(DatabaseError::NotFound{ id }) => assert_eq!(id, "food".to_owned()),
            failed => unreachable!("Should not have gotten {:?}", failed)
        }

        match db.create_org("foo".to_owned()).await {
            Err(DatabaseError::OrganizationAlreadyExists{ org }) => {
                assert_eq!(org, "foo");
            },
            failed => unreachable!("Should not have gotten {:?}", failed)
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

    async fn setup_schema() -> DbResult<DatabaseConnection> {
        use sea_orm::schema::Schema;
        use super::entity::prelude::*;

        let db = Database::connect("sqlite::memory:").await?;

        db.execute(db.get_database_backend().build(&Schema::create_table_from_entity(Organization)))
            .await?;

        db.execute(db.get_database_backend().build(&Schema::create_table_from_entity(Repository)))
            .await?;

        Ok(db)
    }
}