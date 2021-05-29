use async_trait::async_trait;
pub use dumont_backend_base::{BackendDataStore, DataStoreError};
use dumont_backend_sqlite::SqlLiteDataStore;
use std::ops::Deref;

use dumont_models::{models::Organization, operations::CreateOrganization};

pub enum DataStore {
    SqlLite(SqlLiteDataStore),
}

impl DataStore {
    pub async fn create_sqlite(database_url: &str) -> DataStore {
        DataStore::SqlLite(SqlLiteDataStore::new(database_url).await)
    }
}

impl Deref for DataStore {
    type Target = dyn BackendDataStore;

    fn deref(&self) -> &Self::Target {
        match self {
            DataStore::SqlLite(o) => o,
        }
    }
}

#[async_trait]
impl BackendDataStore for DataStore {
    async fn create_organization(
        &self,
        entity: &CreateOrganization,
    ) -> Result<Organization, DataStoreError> {
        self.deref().create_organization(&entity).await
    }

    async fn get_organizations(&self) -> Result<Vec<Organization>, DataStoreError> {
        self.deref().get_organizations().await
    }
}
