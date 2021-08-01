use super::models::{
    DataStoreOrganization, DataStoreOrganizationList, DataStoreRepository, DataStoreRepositoryList,
};
use super::{DataStore, DataStoreError};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

pub struct MemDataStore {
    org_id: AtomicU16,
    repo_id: AtomicU16,
    orgs: Arc<Mutex<BTreeMap<String, DataStoreOrganization>>>,
    repos: Arc<Mutex<BTreeMap<String, ReferencingRepository>>>,
}

#[derive(Debug, Clone)]
pub struct ReferencingRepository {
    pub id: i64,
    pub organization: String,
    pub name: String,
    pub url: Option<String>,
}

impl ReferencingRepository {
    fn into_repo(&self, org: &DataStoreOrganization) -> DataStoreRepository {
        DataStoreRepository {
            id: self.id,
            organization: org.clone(),
            name: self.name.clone(),
            url: self.url.clone(),
        }
    }
}

impl Default for MemDataStore {
    fn default() -> Self {
        Self {
            org_id: AtomicU16::new(0),
            repo_id: AtomicU16::new(0),
            orgs: Arc::new(Mutex::new(BTreeMap::new())),
            repos: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

#[async_trait]
impl DataStore for MemDataStore {
    async fn create_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, DataStoreError> {
        let id = self.org_id.fetch_add(1, Ordering::SeqCst);
        let org_name = org_name.to_string();
        let org = DataStoreOrganization {
            id: id.into(),
            name: org_name.clone(),
        };

        self.orgs.lock().unwrap().insert(org_name, org.clone());

        Ok(org)
    }

    async fn get_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, DataStoreError> {
        match self.orgs.lock().unwrap().get(org_name).map(|x| x.clone()) {
            Some(org) => Ok(org.clone()),
            None => Err(DataStoreError::NotFound {
                id: org_name.to_string(),
            }),
        }
    }

    async fn get_organizations(&self) -> Result<DataStoreOrganizationList, DataStoreError> {
        let orgs = self.orgs.lock().unwrap().values().cloned().collect();
        Ok(DataStoreOrganizationList { orgs })
    }

    async fn create_repo(
        &self,
        org_name: &str,
        repo_name: &str,
        repo_url: &Option<String>,
    ) -> Result<DataStoreRepository, DataStoreError> {
        let org = match self.orgs.lock().unwrap().get(org_name) {
            Some(org) => org.clone(),
            None => {
                return Err(DataStoreError::NotFound {
                    id: org_name.to_string(),
                })
            }
        };

        let id = self.repo_id.fetch_add(1, Ordering::SeqCst);
        let repo_name = repo_name.to_string();
        let ref_repo = ReferencingRepository {
            id: id.into(),
            organization: org.name.clone(),
            name: repo_name.clone(),
            url: repo_url.clone(),
        };

        let key = format!("{}:{}", org.name.clone(), repo_name.clone());

        self.repos.lock().unwrap().insert(key, ref_repo.clone());

        Ok(ref_repo.into_repo(&org))
    }

    async fn get_repos(&self, org_name: &str) -> Result<DataStoreRepositoryList, DataStoreError> {
        let org = match self.orgs.lock().unwrap().get(org_name).map(|x| x.clone()) {
            Some(org) => org.clone(),
            None => {
                return Err(DataStoreError::NotFound {
                    id: org_name.to_string(),
                })
            }
        };

        let mut repos: Vec<DataStoreRepository> = Vec::new();

        for repo in self.repos.lock().unwrap().values() {
            if repo.organization == org.name {
                repos.push(repo.into_repo(&org));
            }
        }

        Ok(DataStoreRepositoryList { repos })
    }

    async fn get_repo(
        &self,
        org_name: &str,
        repo_name: &str,
    ) -> Result<Option<DataStoreRepository>, DataStoreError> {
        let key = format!("{}:{}", org_name, repo_name);

        let repos = self.repos.lock().unwrap();
        let repo = match repos.get(&key) {
            Some(repo) => repo,
            None => return Err(DataStoreError::NotFound { id: key }),
        };

        let orgs = self.orgs.lock().unwrap();
        let org = match orgs.get(org_name).map(|x| x.clone()) {
            Some(org) => org.clone(),
            None => {
                return Err(DataStoreError::NotFound {
                    id: org_name.to_string(),
                })
            }
        };

        Ok(Some(repo.into_repo(&org)))
    }
}
