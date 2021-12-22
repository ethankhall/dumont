use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbOrganization {
    pub org_id: i32,
    pub org_name: String,
}

impl From<super::entity::organization::Model> for DbOrganization {
    fn from(org: super::entity::organization::Model) -> Self {
        Self {
            org_id: org.org_id,
            org_name: org.org_name,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DbRepo {
    pub org: DbOrganization,
    pub repo_id: i32,
    pub repo_name: String,
}

impl DbRepo {
    pub fn from(org: &DbOrganization, repo: &super::entity::repository::Model) -> Self {
        Self {
            org: org.clone(),
            repo_id: repo.repo_id,
            repo_name: repo.repo_name.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UpdateRepoMetadata {
    pub repo_url: Option<String>,
}
