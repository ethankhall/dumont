pub mod models;

use async_trait::async_trait;
pub use memory::MemDataStore;
use models::*;
use thiserror::Error;
use tracing::error;

mod memory;
#[cfg(feature = "postgres")]
mod postgres;

#[derive(Error, Debug)]
pub enum DataStoreError {
    #[error("{id} not found")]
    NotFound { id: String },
    #[error(transparent)]
    BackendError {
        #[from]
        source: anyhow::Error,
    },
}

// impl From<sqlx::Error> for DataStoreError {
//     fn from(e: sqlx::Error) -> Self {
//         error!("Unable to exec SQL: {:?}", e);
//         DataStoreError::BackendError { source: e.into() }
//     }
// }

#[async_trait]
pub trait DataStore: Sync + Send {
    async fn create_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, DataStoreError>;

    async fn get_organizations(&self) -> Result<DataStoreOrganizationList, DataStoreError>;

    async fn get_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, DataStoreError>;

    async fn create_repo(
        &self,
        org_name: &str,
        repo_name: &str,
        repo_url: &Option<String>,
    ) -> Result<DataStoreRepository, DataStoreError>;

    async fn get_repos(&self, org_name: &str) -> Result<DataStoreRepositoryList, DataStoreError>;

    async fn get_repo(
        &self,
        org_name: &str,
        repo_name: &str,
    ) -> Result<DataStoreRepository, DataStoreError>;
}
