use serde::{Deserialize, Serialize};

pub type GetOrganization = CreateOrganization;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOrganization {
    pub organization: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRepository {
    pub repository: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRepository {
    pub organization: String,
    pub repository: String,
    pub url: Option<String>,
}
