use crate::database::prelude::{DbOrganization, DbRepo};

#[derive(Debug, Clone)]
pub struct PaginationOptions {
    pub page_number: u64,
    pub page_size: u64,
}

impl PaginationOptions {
    pub fn new(page_number: u64, page_size: u64) -> Self {
        Self {
            page_number,
            page_size,
        }
    }

    pub fn has_more(&self, total: u64) -> bool {
        (1 + self.page_number) * self.page_size < total
    }
}

#[test]
fn validate_has_more() {
    assert_eq!(PaginationOptions::new(0, 50).has_more(100), true);
    assert_eq!(PaginationOptions::new(0, 50).has_more(10), false);
    assert_eq!(PaginationOptions::new(10, 50).has_more(551), true);
    assert_eq!(PaginationOptions::new(10, 50).has_more(400), false);
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
            id: source.org_id,
            name: source.org_name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataStoreOrganizationList {
    pub orgs: Vec<DataStoreOrganization>,
    pub total_count: u64,
    pub has_more: bool,
}

impl DataStoreOrganizationList {
    pub fn from(
        source: Vec<crate::database::prelude::DbOrganizationModel>,
        total_count: u64,
        has_more: bool,
    ) -> Self {
        let orgs: Vec<DataStoreOrganization> = source.iter().map(|it| it.into()).collect();

        Self {
            orgs,
            total_count,
            has_more,
        }
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
    pub total_count: u64,
    pub has_more: bool,
}

impl DataStoreRepositoryList {
    pub fn from(
        source: Vec<crate::database::prelude::DbRepoModel>,
        total_count: u64,
        has_more: bool,
    ) -> Self {
        let repos: Vec<DataStoreRepository> = source.iter().map(|it| it.into()).collect();

        Self {
            repos,
            total_count,
            has_more,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataStoreVersionList {
    pub versions: Vec<DataStoreRevision>,
    pub total_count: u64,
    pub has_more: bool,
}

impl DataStoreVersionList {
    pub fn from(
        source: Vec<crate::database::prelude::DbRevisionModel>,
        total_count: u64,
        has_more: bool,
    ) -> DataStoreVersionList {
        let versions: Vec<DataStoreRevision> = source.iter().map(|it| it.into()).collect();

        Self {
            versions,
            total_count,
            has_more,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataStoreRevision {
    pub version: String,
    pub labels: crate::models::GenericLabels,
}

impl From<crate::database::prelude::DbRevisionModel> for DataStoreRevision {
    fn from(source: crate::database::prelude::DbRevisionModel) -> Self {
        (&source).into()
    }
}

impl From<&crate::database::prelude::DbRevisionModel> for DataStoreRevision {
    fn from(source: &crate::database::prelude::DbRevisionModel) -> Self {
        Self {
            version: source.revision_name.clone(),
            labels: source.labels.clone(),
        }
    }
}
