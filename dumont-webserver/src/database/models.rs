pub trait DbOrganization {
    fn get_org_id(&self) -> i32;
    fn get_org_name(&self) -> String;
}

pub trait DbRepo {
    fn get_repo_id(&self) -> i32;
    fn get_repo_name(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbOrganizationModel {
    pub org_id: i32,
    pub org_name: String,
}

impl DbOrganization for DbOrganizationModel {
    fn get_org_id(&self) -> i32 {
        self.org_id
    }
    fn get_org_name(&self) -> String {
        self.org_name.clone()
    }
}
impl From<&super::entity::organization::Model> for DbOrganizationModel {
    fn from(org: &super::entity::organization::Model) -> Self {
        Self {
            org_id: org.org_id,
            org_name: org.org_name.clone(),
        }
    }
}

impl From<super::entity::organization::Model> for DbOrganizationModel {
    fn from(org: super::entity::organization::Model) -> Self {
        (&org).into()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DbRepoModel {
    pub org_id: i32,
    pub org_name: String,
    pub repo_id: i32,
    pub repo_name: String,
    pub metadata: RepoMetadata,
}

impl DbOrganization for DbRepoModel {
    fn get_org_id(&self) -> i32 {
        self.org_id
    }
    fn get_org_name(&self) -> String {
        self.org_name.clone()
    }
}

impl DbRepo for DbRepoModel {
    fn get_repo_id(&self) -> i32 {
        self.repo_id
    }
    
    fn get_repo_name(&self) -> String {
        self.repo_name.clone()
    }
}

impl DbRepoModel {
    pub fn from(org: &super::entity::organization::Model, repo: &super::entity::repository::Model, metadata: &super::entity::repository_metadata::Model) -> Self {
        Self {
            org_id: org.org_id,
            org_name: org.org_name.clone(),
            repo_id: repo.repo_id,
            repo_name: repo.repo_name.clone(),
            metadata: metadata.into(),
        }
    }

    pub fn from_optional_meta(org: &super::entity::organization::Model, repo: &super::entity::repository::Model, metadata: &Option<super::entity::repository_metadata::Model>) -> Self {
        let metadata = match metadata {
            Some(meta) => meta.into(),
            None => Default::default()
        };
        Self {
            org_id: org.org_id,
            org_name: org.org_name.clone(),
            repo_id: repo.repo_id,
            repo_name: repo.repo_name.clone(),
            metadata
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RepoMetadata {
    pub repo_url: Option<String>,
}

impl From<&super::entity::repository_metadata::Model> for RepoMetadata {
    fn from(source: &super::entity::repository_metadata::Model) -> Self {
        Self {
            repo_url: source.repo_url.clone()
        }
    }
}

impl From<super::entity::repository_metadata::Model> for RepoMetadata {
    fn from(source: super::entity::repository_metadata::Model) -> Self {
        (&source).into()
    }
}

impl Default for RepoMetadata {
    fn default() -> Self {
        Self { 
            repo_url: None
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UpdateRepoMetadata {
    pub repo_url: Option<String>,
}
