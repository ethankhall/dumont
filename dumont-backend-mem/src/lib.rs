use async_trait::async_trait;
use dumont_backend_base::{DataStore, DataStoreError};
use dumont_models::{
    models::{Organization, Repository},
    operations::{CreateOrganization, GetOrganization, CreateRepository, GetRepository},
};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU16, Ordering};
use std::collections::BTreeMap;


pub struct MemDataStore {
    org_id: AtomicU16,
    repo_id: AtomicU16,
    orgs: Arc<Mutex<BTreeMap<String, Organization>>>,
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
    fn into_repo(&self, org: &Organization) -> Repository {
        Repository{ 
            id: self.id,
            organization: org.clone(),
            name: self.name.clone(),
            url: self.url.clone()
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
        entity: &CreateOrganization,
    ) -> Result<Organization, DataStoreError> {

        let id = self.org_id.fetch_add(1, Ordering::SeqCst);
        let org_name = entity.organization.clone();
        let org = Organization {
            id: id.into(),
            name: org_name.clone()
        };

        self.orgs.lock().unwrap().insert(org_name, org.clone());
        
        Ok(org)
    }

    async fn get_organization(&self, entity: &GetOrganization) -> Result<Organization, DataStoreError> {
        match self.orgs.lock().unwrap().get(&entity.organization).map(|x| x.clone()) {
            Some(org) => Ok(org.clone()),
            None => Err(DataStoreError::NotFound { id: entity.organization.clone() })
        }
    }
    
    async fn get_organizations(&self) -> Result<Vec<Organization>, DataStoreError> {
        Ok(self.orgs.lock().unwrap().values().cloned().collect())
    }

    async fn create_repo(&self, entity: &CreateRepository) -> Result<Repository, DataStoreError> {

        let org = match self.orgs.lock().unwrap().get(&entity.organization) {
            Some(org) => org.clone(),
            None => { return Err(DataStoreError::NotFound { id: entity.organization.clone() })}
        };

        let id = self.repo_id.fetch_add(1, Ordering::SeqCst);
        let repo_name = entity.repository.clone();
        let ref_repo = ReferencingRepository {
            id: id.into(),
            organization: org.name.clone(),
            name: repo_name.clone(),
            url: entity.url.clone()
        };

        let key = format!("{}:{}", org.name.clone(), repo_name.clone());

        self.repos.lock().unwrap().insert(key, ref_repo.clone());

        Ok(ref_repo.into_repo(&org))
    }

    async fn get_repo(&self, entity: &GetRepository) -> Result<Option<Repository>, DataStoreError> {
        let key = format!("{}:{}", entity.organization.clone(), entity.repository.clone());

        let repos = self.repos.lock().unwrap();
        let repo = match repos.get(&key) {
            Some(repo) => repo,
            None => { return Err(DataStoreError::NotFound { id: key })}
        };

        let orgs = self.orgs.lock().unwrap();
        let org = match orgs.get(&entity.organization).map(|x| x.clone()) {
            Some(org) => org.clone(),
            None => {return Err(DataStoreError::NotFound { id: entity.organization.clone() }) }
        };

        Ok(Some(repo.into_repo(&org)))
    }
}
