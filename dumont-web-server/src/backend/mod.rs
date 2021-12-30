pub mod models;
use std::collections::BTreeMap;

use crate::models::GenericLabels;
use models::*;
use thiserror::Error;
use tracing::{error, info};
use tracing_attributes::instrument;

use crate::database::prelude::*;
use crate::policy::{PolicyError, RealizedPolicyContainer};

#[derive(Error, Debug)]
pub enum BackendError {
    #[error(transparent)]
    DatabaseError {
        #[from]
        source: DatabaseError,
    },
    #[error("Requested action was not allowed because: {reason}")]
    ConstraintViolation { reason: ConstraintViolation },
    #[error(transparent)]
    PolicyViolation {
        #[from]
        error: PolicyError,
    },
}

#[derive(Error, Debug)]
pub enum ConstraintViolation {
    #[error("Version string '{version}' was more than the 30 character limit")]
    VersionToLong { version: String },
}

pub struct DefaultBackend {
    pub database: PostgresDatabase,
    pub policy_container: RealizedPolicyContainer,
}

impl DefaultBackend {
    pub async fn new(
        db_connection_string: String,
        policy_container: RealizedPolicyContainer,
    ) -> Result<Self, BackendError> {
        info!(
            "Policies Configured\n{}",
            toml::to_string_pretty(&policy_container)
                .unwrap_or_else(|_| "Policy failed to render".to_owned())
        );
        Ok(Self {
            database: PostgresDatabase::new(db_connection_string).await?,
            policy_container,
        })
    }

    #[instrument(skip(self))]
    pub async fn create_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, BackendError> {
        let new_org = self.database.create_org(org_name).await?;
        Ok(new_org.into())
    }

    #[instrument(skip(self))]
    pub async fn delete_organization(&self, org_name: &str) -> Result<bool, BackendError> {
        Ok(self.database.delete_org(org_name).await?)
    }

    #[instrument(skip(self, pagination))]
    pub async fn list_organizations(
        &self,
        pagination: PaginationOptions,
    ) -> Result<DataStoreOrganizationList, BackendError> {
        let found_orgs = self.database.list_orgs(pagination).await?;
        Ok(found_orgs.into())
    }

    #[instrument(skip(self))]
    pub async fn get_organization(
        &self,
        org_name: &str,
    ) -> Result<DataStoreOrganization, BackendError> {
        let new_org = self.database.find_org(org_name).await?;
        Ok(new_org.into())
    }

    #[instrument(skip(self, provided_labels))]
    pub async fn create_repo(
        &self,
        org_name: &str,
        repo_name: &str,
        provided_labels: BTreeMap<String, String>,
    ) -> Result<DataStoreRepository, BackendError> {
        let mut labels = provided_labels.clone();
        self.policy_container
            .execute_repo_policies(org_name, repo_name, &mut labels)?;

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

    #[instrument(skip(self, pagination))]
    pub async fn list_repos(
        &self,
        org_name: &str,
        pagination: PaginationOptions,
    ) -> Result<DataStoreRepositoryList, BackendError> {
        let repos = self.database.list_repo(org_name, pagination).await?;
        Ok(repos.into())
    }

    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
    pub async fn delete_repo(&self, org_name: &str, repo_name: &str) -> Result<bool, BackendError> {
        Ok(self
            .database
            .delete_repo(&RepoParam::new(org_name, repo_name))
            .await?)
    }

    #[instrument(skip(self, provided_labels))]
    pub async fn update_repo(
        &self,
        org_name: &str,
        repo_name: &str,
        provided_labels: BTreeMap<String, String>,
    ) -> Result<DataStoreRepository, BackendError> {
        let mut labels = provided_labels.clone();
        self.policy_container
            .execute_repo_policies(org_name, repo_name, &mut labels)?;

        self.database
            .set_repo_labels(&RepoParam::new(org_name, repo_name), labels)
            .await?;
        let repo = self
            .database
            .get_repo(&RepoParam::new(org_name, repo_name))
            .await?;
        Ok(repo.into())
    }

    #[instrument(skip(self, provided_labels))]
    pub async fn create_version(
        &self,
        org_name: &str,
        repo_name: &str,
        version_name: &str,
        provided_labels: BTreeMap<String, String>,
    ) -> Result<DataStoreRevision, BackendError> {
        if version_name.len() > 30 {
            return Err(BackendError::ConstraintViolation {
                reason: ConstraintViolation::VersionToLong {
                    version: version_name.to_owned(),
                },
            });
        }

        let mut labels = provided_labels.clone();
        self.policy_container
            .execute_version_policies(org_name, repo_name, &mut labels)?;

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

    #[instrument(skip(self, provided_labels))]
    pub async fn update_version(
        &self,
        org_name: &str,
        repo_name: &str,
        version_name: &str,
        provided_labels: GenericLabels,
    ) -> Result<DataStoreRevision, BackendError> {
        let mut labels = provided_labels.labels.clone();
        self.policy_container
            .execute_version_policies(org_name, repo_name, &mut labels)?;

        let param = RevisionParam::new(org_name, repo_name, version_name);
        self.database.set_revision_labels(&param, &labels).await?;

        let revision = self.database.get_revision(&param).await?;
        Ok(revision.into())
    }

    #[instrument(skip(self))]
    pub async fn delete_version(
        &self,
        org_name: &str,
        repo_name: &str,
        version_name: &str,
    ) -> Result<bool, BackendError> {
        let param = RevisionParam::new(org_name, repo_name, version_name);
        Ok(self.database.delete_revision(&param).await?)
    }

    #[instrument(skip(self, pagination))]
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

    #[instrument(skip(self))]
    pub async fn get_version(
        &self,
        org_name: &str,
        repo_name: &str,
        version_name: &str,
    ) -> Result<DataStoreRevision, BackendError> {
        let param = RevisionParam::new(org_name, repo_name, version_name);
        let revision = self.database.get_revision(&param).await?;
        Ok(revision.into())
    }
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::policy::*;
    use crate::test_utils::*;
    use serial_test::serial;

    fn make_policy() -> RealizedPolicyContainer {
        RealizedPolicyContainer {
            policies: vec![RealizedPolicy::test_new_different_labels(
                "example/repo-1",
                vec![RequiredLabel::new("owner", vec!["bob"], None)],
                vec![RequiredLabel::new("git_sha", vec![], None)],
            )],
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn policy_enforcement_repo_label_create() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        let backend = DefaultBackend {
            database: db,
            policy_container: make_policy(),
        };

        backend.create_organization("example").await.unwrap();

        assert_eq!(backend
            .create_repo(
                "example",
                "repo-1",
                BTreeMap::from_iter(vec![("owner".to_owned(), "alice".to_owned())])
            )
            .await
            .unwrap_err().to_string(), "Policy `test` required that label `owner` be one of a set values, however `alice` was not in that set.");

        assert!(backend
            .create_repo(
                "example",
                "repo-1",
                BTreeMap::from_iter(vec![("owner".to_owned(), "bob".to_owned())])
            )
            .await
            .is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn policy_enforcement_repo_label_update() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        let backend = DefaultBackend {
            database: db,
            policy_container: make_policy(),
        };

        backend.create_organization("example").await.unwrap();
        assert!(backend
            .create_repo(
                "example",
                "repo-1",
                BTreeMap::from_iter(vec![("owner".to_owned(), "bob".to_owned())])
            )
            .await
            .is_ok());

        assert_eq!(backend
                .update_repo(
                    "example",
                    "repo-1",
                    BTreeMap::from_iter(vec![("owner".to_owned(), "alice".to_owned())])
                )
                .await
                .unwrap_err().to_string(), "Policy `test` required that label `owner` be one of a set values, however `alice` was not in that set.");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn policy_enforcement_version_label_create() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        let backend = DefaultBackend {
            database: db,
            policy_container: make_policy(),
        };

        backend.create_organization("example").await.unwrap();
        assert!(backend
            .create_repo(
                "example",
                "repo-1",
                BTreeMap::from_iter(vec![("owner".to_owned(), "bob".to_owned())])
            )
            .await
            .is_ok());

        assert_eq!(backend
                .create_version(
                    "example",
                    "repo-1",
                    "1.2.3",
                    BTreeMap::from_iter(vec![("git".to_owned(), "123".to_owned())])
                )
                .await
                .unwrap_err().to_string(), "Policy `test` required that label `git_sha` be set, however it was not and no default was specified.");

        assert!(backend
            .create_version(
                "example",
                "repo-1",
                "1.2.3",
                BTreeMap::from_iter(vec![("git_sha".to_owned(), "123".to_owned())])
            )
            .await
            .is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn policy_enforcement_version_label_update() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        let backend = DefaultBackend {
            database: db,
            policy_container: make_policy(),
        };

        backend.create_organization("example").await.unwrap();
        assert!(backend
            .create_repo(
                "example",
                "repo-1",
                BTreeMap::from_iter(vec![("owner".to_owned(), "bob".to_owned())])
            )
            .await
            .is_ok());

        assert!(backend
            .create_version(
                "example",
                "repo-1",
                "1.2.3",
                BTreeMap::from_iter(vec![("git_sha".to_owned(), "123".to_owned())])
            )
            .await
            .is_ok());

        assert_eq!(backend
                .create_version(
                    "example",
                    "repo-1",
                    "1.2.3",
                    BTreeMap::from_iter(vec![("git".to_owned(), "123".to_owned())])
                )
                .await
                .unwrap_err().to_string(), "Policy `test` required that label `git_sha` be set, however it was not and no default was specified.");
    }
}
