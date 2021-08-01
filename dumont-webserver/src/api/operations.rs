use serde::{Deserialize, Serialize};

pub type GetOrganization = CreateOrganization;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOrganization {
    pub org: String,
}

impl From<&crate::backend::models::DataStoreOrganization> for GetOrganization {
    fn from(model: &crate::backend::models::DataStoreOrganization) -> Self {
        Self {
            org: model.name.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRepository {
    pub repo: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRepository {
    pub org: String,
    pub repo: String,
    pub url: Option<String>,
}

impl From<&crate::backend::models::DataStoreRepository> for GetRepository {
    fn from(model: &crate::backend::models::DataStoreRepository) -> Self {
        Self {
            org: model.organization.name.clone(),
            repo: model.name.clone(),
            url: model.url.clone(),
        }
    }
}
