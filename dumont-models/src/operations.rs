use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOrganization {
    pub organization: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRepository {
    pub organization: String,
    pub repository: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRepository {
    pub organization: String,
    pub repository: String,
}
