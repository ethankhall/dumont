pub mod models;
use std::collections::BTreeMap;

use models::*;
use thiserror::Error;
use tracing::error;

use crate::database::prelude::*;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("{id} not found")]
    NotFound { id: String },
    #[error(transparent)]
    DatabaseError {
        #[from]
        source: DatabaseError,
    },
}

pub struct DefaultBackend {
    database: PostresDatabase,
}

impl DefaultBackend {
    pub async fn new(db_connection_string: String) -> Result<Self, BackendError> {
        Ok(Self {
            database: PostresDatabase::new(db_connection_string).await?,
        })
    }

    pub async fn create_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, BackendError> {
        let new_org = self.database.create_org(org_name).await?;
        Ok(new_org.into())
    }

    pub async fn delete_organization(&self, org_name: &str) -> Result<bool, BackendError> {
        Ok(self.database.delete_org(org_name).await?)
    }

    pub async fn get_organizations(
        &self,
        pagination: PaginationOptions,
    ) -> Result<DataStoreOrganizationList, BackendError> {
        let found_orgs = self.database.list_orgs(pagination).await?;
        Ok(found_orgs.into())
    }

    pub async fn get_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, BackendError> {
        let new_org = self.database.find_org(org_name).await?;
        Ok(new_org.into())
    }

    pub async fn create_repo(
        &self,
        org_name: &str,
        repo_name: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<DataStoreRepository, BackendError> {
        self.database.create_repo(org_name, repo_name).await?;
        self.database
            .set_repo_labels(org_name, repo_name, labels)
            .await?;

        let repo = self.database.get_repo(&org_name, repo_name).await?;

        Ok(repo.into())
    }

    pub async fn get_repos(
        &self,
        org_name: &str,
        pagination: PaginationOptions,
    ) -> Result<DataStoreRepositoryList, BackendError> {
        let repos = self.database.list_repo(&org_name, pagination).await?;
        Ok(repos.into())
    }

    pub async fn get_repo(
        &self,
        org_name: &str,
        repo_name: &str,
    ) -> Result<DataStoreRepository, BackendError> {
        let repo = self.database.get_repo(&org_name, repo_name).await?;
        Ok(repo.into())
    }

    pub async fn delete_repo(&self, org_name: &str, repo_name: &str) -> Result<bool, BackendError> {
        Ok(self.database.delete_repo(org_name, repo_name).await?)
    }

    pub async fn update_repo(
        &self,
        org_name: &str,
        repo_name: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<DataStoreRepository, BackendError> {
        self.database
            .set_repo_labels(org_name, repo_name, labels)
            .await?;
        let repo = self.database.get_repo(&org_name, repo_name).await?;
        Ok(repo.into())
    }
}
