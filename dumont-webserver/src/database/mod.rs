// Generated with `sea-orm-cli generate entity -s public -o src/database/entity`
mod entity;

#[cfg(test)]
mod common_tests;
mod org_queries;
mod repo_label_queries;
mod repo_queries;
mod reversion_label_queries;
mod reversion_queries;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use thiserror::Error;

pub type DbResult<T> = Result<T, DatabaseError>;

#[derive(Error, Debug)]
pub enum NotFoundError {
    #[error("Org {org} not found")]
    Organization { org: String },
    #[error("Repo {org}/{repo} not found")]
    Repo { org: String, repo: String },
    #[error("Revision {org}/{repo}/{revision} not found")]
    Revision {
        org: String,
        repo: String,
        revision: String,
    },
    #[error("Repo with id {repo_id} not found")]
    RepoById { repo_id: i32 },
}

#[derive(Error, Debug)]
pub enum AlreadyExistsError {
    #[error("Org {org} exists")]
    Organization { org: String },
    #[error("Repo {org}/{repo} exists")]
    Repo { org: String, repo: String },
    #[error("Revision {org}/{repo}/{revision} exists")]
    Revision {
        org: String,
        repo: String,
        revision: String,
    },
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error(transparent)]
    NotFound { error: NotFoundError },
    #[error(transparent)]
    AlreadyExists { error: AlreadyExistsError },
    #[error(transparent)]
    BackendError {
        #[from]
        source: anyhow::Error,
    },
    #[error(transparent)]
    SeaOrmError {
        #[from]
        source: sea_orm::DbErr,
    },
    #[error(transparent)]
    SqlxError {
        #[from]
        source: sqlx::Error,
    },
    #[error(transparent)]
    MigrateError {
        #[from]
        source: sqlx::migrate::MigrateError,
    },
}

pub mod models {
    use std::collections::BTreeMap;
    use std::ops::Deref;

    #[derive(Debug, PartialEq, Eq)]
    pub struct GenericLabels {
        pub labels: BTreeMap<String, String>,
    }

    impl From<BTreeMap<String, String>> for GenericLabels {
        fn from(source: BTreeMap<String, String>) -> Self {
            Self { labels: source }
        }
    }

    impl From<Vec<(&str, &str)>> for GenericLabels {
        fn from(source: Vec<(&str, &str)>) -> Self {
            let mut labels: BTreeMap<String, String> = Default::default();
            for (key, value) in source {
                labels.insert(key.to_owned(), value.to_owned());
            }

            labels.into()
        }
    }

    impl Deref for GenericLabels {
        type Target = BTreeMap<String, String>;
        fn deref(&self) -> &Self::Target {
            &self.labels
        }
    }

    impl Default for GenericLabels {
        fn default() -> Self {
            Self {
                labels: Default::default(),
            }
        }
    }
}

pub enum DateTimeProvider {
    RealDateTime,
}

impl DateTimeProvider {
    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

pub struct PostresDatabase {
    db: DatabaseConnection,
    date_time_provider: DateTimeProvider,
}

impl PostresDatabase {
    pub async fn new<S: Into<String>>(connection_url: S) -> prelude::DbResult<Self> {
        let mut opts: ConnectOptions = ConnectOptions::new(connection_url.into());
        opts.sqlx_logging(cfg!(debug_assertions) || cfg!(test_assertions));
        let db: DatabaseConnection = Database::connect(opts).await?;

        Ok(Self {
            db,
            date_time_provider: DateTimeProvider::RealDateTime,
        })
    }
}

pub mod prelude {
    pub use super::entity::prelude::*;
    pub use super::org_queries::{models::*, DbOrganization, OrganizationQueries};
    pub use super::repo_label_queries::{models::*, RepoLabelQueries};
    pub use super::repo_queries::{models::*, DbRepo, RepoQueries};
    pub use super::reversion_label_queries::{models::*, RevisionLabelQueries};
    pub use super::DbResult;
    pub use super::{AlreadyExistsError, DatabaseError, NotFoundError, PostresDatabase};
    pub use thiserror::Error;
}
