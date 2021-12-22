pub mod models;

use models::*;
use thiserror::Error;
use tracing::error;

use crate::database::prelude::*;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("{id} not found")]
    NotFound { id: String },
    #[error(transparent)]
    DatabaseError {
        #[from]
        source: DatabaseError,
    },
}

// impl From<sqlx::Error> for BackendError {
//     fn from(e: sqlx::Error) -> Self {
//         error!("Unable to exec SQL: {:?}", e);
//         BackendError::BackendError { source: e.into() }
//     }
// }

pub struct DefaultBackend {
    database: PostresDatabase,
}

impl DefaultBackend {
    pub async fn new(db_connection_string: String) -> Result<Self, BackendError> {
        Ok(Self {
            database: PostresDatabase::new(db_connection_string).await?,
        })
    }
    pub async fn create_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, BackendError> {
        let new_org = self.database.create_org(org_name).await?;
        Ok(new_org.into())
    }

    pub async fn get_organizations(&self, pagination: PaginationOptions) -> Result<DataStoreOrganizationList, BackendError> {
        let found_orgs = self.database.list_orgs(pagination).await?;
        Ok(found_orgs.into())
    }

    pub async fn get_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, BackendError> {
        let new_org = self.database.find_org(org_name).await?;
        Ok(new_org.into())
    }

    pub async fn create_repo(
        &self,
        org_name: &str,
        repo_name: &str,
        repo_url: &Option<String>,
    ) -> Result<DataStoreRepository, BackendError> {
        let org = self.database.find_org(org_name).await?;
        let repo = self.database.create_repo(&org, repo_name).await?;
        
        if let Some(repo_url) = repo_url {
            self.database.update_repo_metadata(&repo, UpdateRepoMetadata {repo_url: Some(repo_url.to_string())}).await?;
        }

        Ok(repo.into())
    }

    pub async fn get_repos(&self, org_name: &str) -> Result<DataStoreRepositoryList, BackendError> {
        unimplemented!();
    }

    pub async fn get_repo(
        &self,
        org_name: &str,
        repo_name: &str,
    ) -> Result<DataStoreRepository, BackendError> {
        unimplemented!();
    }
}
