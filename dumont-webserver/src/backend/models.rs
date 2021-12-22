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

impl From<crate::database::prelude::DbOrganization> for DataStoreOrganization {
    fn from(source: crate::database::prelude::DbOrganization) -> Self {
        Self {
            id: source.org_id,
            name: source.org_name,
        }
    }
}

impl From<&crate::database::prelude::DbOrganization> for DataStoreOrganization {
    fn from(source: &crate::database::prelude::DbOrganization) -> Self {
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

impl From<Vec<crate::database::prelude::DbOrganization>> for DataStoreOrganizationList {
    fn from(source: Vec<crate::database::prelude::DbOrganization>) -> Self {
        let orgs: Vec<DataStoreOrganization> = source.iter().map(|it| it.into()).collect();

        Self { orgs }
    }
}

#[derive(Debug, Clone)]
pub struct DataStoreRepository {
    pub id: i32,
    pub organization: DataStoreOrganization,
    pub name: String,
}

impl From<crate::database::prelude::DbRepo> for DataStoreRepository {
    fn from(source: crate::database::prelude::DbRepo) -> Self {
        Self {
            id: source.repo_id,
            organization: source.org.into(),
            name: source.repo_name,
        }
    }
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
