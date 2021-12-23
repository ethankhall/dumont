use serde::{Deserialize, Serialize};

pub type GetOrganization = CreateOrganization;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOrganization {
    pub org: String,
}

impl From<crate::backend::models::DataStoreOrganization> for GetOrganization {
    fn from(model: crate::backend::models::DataStoreOrganization) -> Self {
        (&model).into()
    }
}

impl From<&crate::backend::models::DataStoreOrganization> for GetOrganization {
    fn from(model: &crate::backend::models::DataStoreOrganization) -> Self {
        Self {
            org: model.name.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VersionScheme {
    Serial,
    Semver,
}

impl Default for VersionScheme {
    fn default() -> Self {
        Self::Semver
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRepository {
    pub repo: String,
    #[serde(default)]
    pub version_scheme: VersionScheme,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRepository {
    pub org: String,
    pub repo: String,
}

impl From<&crate::backend::models::DataStoreRepository> for GetRepository {
    fn from(model: &crate::backend::models::DataStoreRepository) -> Self {
        Self {
            org: model.organization.name.clone(),
            repo: model.name.clone(),
        }
    }
}

impl From<crate::backend::models::DataStoreRepository> for GetRepository {
    fn from(model: crate::backend::models::DataStoreRepository) -> Self {
        Self {
            org: model.organization.name.clone(),
            repo: model.name.clone(),
        }
    }
}
