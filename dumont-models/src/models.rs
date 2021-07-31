use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Organization {
    #[serde(skip_serializing)]
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Repository {
    #[serde(skip_serializing)]
    pub id: i64,
    pub organization: Organization,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepositoryMetadata {
    #[serde(skip_serializing)]
    pub id: i64,
    pub repo: Repository,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepositoryTag {
    #[serde(skip_serializing)]
    pub id: i64,
    pub repo: Repository,
    pub tag: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryRevision {
    #[serde(skip_serializing)]
    pub id: i64,
    pub repo: Repository,
    pub revision_name: String,
    pub revision_id: String,
    pub revision_state: String,
}
