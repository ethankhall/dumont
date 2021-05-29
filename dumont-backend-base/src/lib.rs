use async_trait::async_trait;
use dumont_models::{models::Organization, operations::CreateOrganization};
use thiserror::Error;

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

#[async_trait]
pub trait BackendDataStore: Sync + Send {
    async fn create_organization(
        &self,
        entity: &CreateOrganization,
    ) -> Result<Organization, DataStoreError>;

    async fn get_organizations(&self) -> Result<Vec<Organization>, DataStoreError>;
}
