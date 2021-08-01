use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataStoreOrganization {
    #[serde(skip_serializing)]
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataStoreOrganizationList {
    pub orgs: Vec<DataStoreOrganization>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataStoreRepository {
    #[serde(skip_serializing)]
    pub id: i64,
    pub organization: DataStoreOrganization,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataStoreRepositoryList {
    pub repos: Vec<DataStoreRepository>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataStoreRepositoryMetadata {
    #[serde(skip_serializing)]
    pub id: i64,
    pub repo: DataStoreRepository,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataStoreRepositoryTag {
    #[serde(skip_serializing)]
    pub id: i64,
    pub repo: DataStoreRepository,
    pub tag: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataStoreRepositoryRevision {
    #[serde(skip_serializing)]
    pub id: i64,
    pub repo: DataStoreRepository,
    pub revision_name: String,
    pub revision_id: String,
    pub revision_state: String,
}
