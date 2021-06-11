use async_trait::async_trait;
use dumont_models::{
    models::{Organization, Repository},
    operations::{CreateOrganization, CreateRepository, GetRepository},
};
use thiserror::Error;
use tracing::error;

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

impl From<sqlx::Error> for DataStoreError {
    fn from(e: sqlx::Error) -> Self {
        error!("Unable to exec SQL: {:?}", e);
        DataStoreError::BackendError { source: e.into() }
    }
}

#[async_trait]
pub trait BackendDataStore: Sync + Send {
    async fn create_organization(
        &self,
        entity: &CreateOrganization,
    ) -> Result<Organization, DataStoreError>;

    async fn get_organizations(&self) -> Result<Vec<Organization>, DataStoreError>;

    async fn create_repo(&self, entity: &CreateRepository) -> Result<Repository, DataStoreError>;

    async fn get_repo(&self, entity: &GetRepository) -> Result<Option<Repository>, DataStoreError>;
}
