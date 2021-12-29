pub mod models;
use std::collections::BTreeMap;

use crate::models::GenericLabels;
use models::*;
use thiserror::Error;
use tracing::error;

use crate::database::prelude::*;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error(transparent)]
    DatabaseError {
        #[from]
        source: DatabaseError,
    },
    #[error("Requested action was not allowed because: {reason}")]
    ConstraintViolation { reason: ConstraintViolation },
}

#[derive(Error, Debug)]
pub enum ConstraintViolation {
    #[error("Version string '{version}' was more than the 30 character limit")]
    VersionToLong { version: String },
}

pub struct DefaultBackend {
    pub database: PostgresDatabase,
}

impl DefaultBackend {
    pub async fn new(db_connection_string: String) -> Result<Self, BackendError> {
        Ok(Self {
            database: PostgresDatabase::new(db_connection_string).await?,
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
        let repo = self
            .database
            .create_repo(
                &RepoParam::new(org_name, repo_name),
                CreateRepoParam {
                    labels: labels.into(),
                },
            )
            .await?;

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
        let repo = self
            .database
            .get_repo(&RepoParam::new(org_name, repo_name))
            .await?;
        Ok(repo.into())
    }

    pub async fn delete_repo(&self, org_name: &str, repo_name: &str) -> Result<bool, BackendError> {
        Ok(self
            .database
            .delete_repo(&RepoParam::new(org_name, repo_name))
            .await?)
    }

    pub async fn update_repo(
        &self,
        org_name: &str,
        repo_name: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<DataStoreRepository, BackendError> {
        self.database
            .set_repo_labels(&RepoParam::new(org_name, repo_name), labels)
            .await?;
        let repo = self
            .database
            .get_repo(&RepoParam::new(org_name, repo_name))
            .await?;
        Ok(repo.into())
    }

    pub async fn create_version(
        &self,
        org_name: &str,
        repo_name: &str,
        version_name: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<DataStoreRevision, BackendError> {
        if version_name.len() > 30 {
            return Err(BackendError::ConstraintViolation {
                reason: ConstraintViolation::VersionToLong {
                    version: version_name.to_owned(),
                },
            });
        }
        let param = RevisionParam::new(org_name, repo_name, version_name);
        self.database
            .create_revision(
                &param,
                &CreateRevisionParam {
                    artifact_url: None,
                    labels: labels.into(),
                },
            )
            .await?;

        let revision = self.database.get_revision(&param).await?;
        Ok(revision.into())
    }

    pub async fn update_version(
        &self,
        org_name: &str,
        repo_name: &str,
        version_name: &str,
        labels: GenericLabels,
    ) -> Result<DataStoreRevision, BackendError> {
        let param = RevisionParam::new(org_name, repo_name, version_name);
        self.database.set_revision_labels(&param, labels).await?;

        let revision = self.database.get_revision(&param).await?;
        Ok(revision.into())
    }

    pub async fn delete_version(
        &self,
        org_name: &str,
        repo_name: &str,
        version_name: &str,
    ) -> Result<bool, BackendError> {
        let param = RevisionParam::new(org_name, repo_name, version_name);
        Ok(self.database.delete_revision(&param).await?)
    }

    pub async fn list_versions(
        &self,
        org_name: &str,
        repo_name: &str,
        pagination: PaginationOptions,
    ) -> Result<DataStoreVersionList, BackendError> {
        Ok(self
            .database
            .list_revisions(&RepoParam::new(org_name, repo_name), pagination)
            .await?
            .into())
    }
}
