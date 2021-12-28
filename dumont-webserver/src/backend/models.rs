use crate::database::prelude::{DbOrganization, DbRepo};

#[derive(Debug, Clone)]
pub struct PaginationOptions {
    pub page_number: usize,
    pub page_size: usize,
}

impl PaginationOptions {
    pub fn new(page_number: usize, page_size: usize) -> Self {
        Self {
            page_number,
            page_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataStoreOrganization {
    pub id: i32,
    pub name: String,
}

impl From<crate::database::prelude::DbOrganizationModel> for DataStoreOrganization {
    fn from(source: crate::database::prelude::DbOrganizationModel) -> Self {
        (&source).into()
    }
}

impl From<&crate::database::prelude::DbOrganizationModel> for DataStoreOrganization {
    fn from(source: &crate::database::prelude::DbOrganizationModel) -> Self {
        Self {
            id: source.org_id.clone(),
            name: source.org_name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataStoreOrganizationList {
    pub orgs: Vec<DataStoreOrganization>,
}

impl From<Vec<crate::database::prelude::DbOrganizationModel>> for DataStoreOrganizationList {
    fn from(source: Vec<crate::database::prelude::DbOrganizationModel>) -> Self {
        let orgs: Vec<DataStoreOrganization> = source.iter().map(|it| it.into()).collect();

        Self { orgs }
    }
}

#[derive(Debug, Clone)]
pub struct DataStoreRepository {
    pub org_name: String,
    pub repo_name: String,
    pub labels: crate::models::GenericLabels,
}

impl From<crate::database::prelude::DbRepoModel> for DataStoreRepository {
    fn from(source: crate::database::prelude::DbRepoModel) -> Self {
        (&source).into()
    }
}

impl From<&crate::database::prelude::DbRepoModel> for DataStoreRepository {
    fn from(source: &crate::database::prelude::DbRepoModel) -> Self {
        Self {
            org_name: source.get_org_name(),
            repo_name: source.get_repo_name(),
            labels: source.labels.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataStoreRepositoryList {
    pub repos: Vec<DataStoreRepository>,
}

impl From<Vec<crate::database::prelude::DbRepoModel>> for DataStoreRepositoryList {
    fn from(source: Vec<crate::database::prelude::DbRepoModel>) -> Self {
        let repos: Vec<DataStoreRepository> = source.iter().map(|it| it.into()).collect();

        Self { repos }
    }
}

#[derive(Debug)]
pub struct DataStoreRevision {
    pub scm_id: String,
    pub version: String,
    pub labels: crate::models::GenericLabels,
}

impl From<crate::database::prelude::DbRevisionModel> for DataStoreRevision {
    fn from(source: crate::database::prelude::DbRevisionModel) -> Self {
        Self {
            scm_id: source.scm_id.clone(),
            version: source.revision_name.clone(),
            labels: source.labels,
        }
    }
}
