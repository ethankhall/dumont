use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, EnumString, Display, Eq, PartialEq)]
pub enum VersionScheme {
    #[strum(serialize = "serial")]
    Serial,
    #[strum(serialize = "semver")]
    Semver
}

#[derive(Debug, Clone)]
pub struct DataStoreOrganization {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DataStoreOrganizationList {
    pub orgs: Vec<DataStoreOrganization>,
}

#[derive(Debug, Clone)]
pub struct DataStoreRepository {
    pub id: i64,
    pub organization: DataStoreOrganization,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DataStoreRepositoryList {
    pub repos: Vec<DataStoreRepository>,
}

#[derive(Debug, Clone)]
pub struct DataStoreRepositoryMetadata {
    pub id: i64,
    pub repo: DataStoreRepository,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct DataStoreRepositoryTag {
    pub id: i64,
    pub repo: DataStoreRepository,
    pub tag: String,
}

#[derive(Debug)]
pub struct DataStoreRepositoryRevision {
    pub id: i64,
    pub repo: DataStoreRepository,
    pub revision_name: String,
    pub revision_id: String,
    pub revision_state: String,
}
