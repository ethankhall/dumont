use std::collections::BTreeMap;

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
    pub labels: RepoLabels,
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
    pub fn from(
        org: &super::entity::organization::Model,
        repo: &super::entity::repository::Model,
        labels: &Vec<super::entity::repository_label::Model>,
    ) -> Self {
        Self {
            org_id: org.org_id,
            org_name: org.org_name.clone(),
            repo_id: repo.repo_id,
            repo_name: repo.repo_name.clone(),
            labels: labels.into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RepoLabels {
    pub labels: BTreeMap<String, String>,
}

impl From<&Vec<super::entity::repository_label::Model>> for RepoLabels {
    fn from(source: &Vec<super::entity::repository_label::Model>) -> Self {
        let mut labels: BTreeMap<String, String> = Default::default();
        for value in source.iter() {
            labels.insert(value.label_name.to_string(), value.label_value.to_string());
        }

        Self { labels }
    }
}

impl From<Vec<super::entity::repository_label::Model>> for RepoLabels {
    fn from(source: Vec<super::entity::repository_label::Model>) -> Self {
        (&source).into()
    }
}

impl Default for RepoLabels {
    fn default() -> Self {
        Self {
            labels: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UpdateRepoMetadata {
    pub labels: BTreeMap<String, String>,
}
