pub mod models;

use async_trait::async_trait;
use models::*;
use thiserror::Error;
use tracing::error;

use crate::database::{DatabaseError, PostresDatabase};

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
        unimplemented!();
    }

    pub async fn get_organizations(&self) -> Result<DataStoreOrganizationList, BackendError> {
        unimplemented!();
    }

    pub async fn get_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, BackendError> {
        unimplemented!();
    }

    pub async fn create_repo(
        &self,
        org_name: &str,
        repo_name: &str,
        repo_url: &Option<String>,
        version: VersionScheme,
    ) -> Result<DataStoreRepository, BackendError> {
        unimplemented!();
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
