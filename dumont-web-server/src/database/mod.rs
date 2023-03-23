// Generated with `sea-orm-cli generate entity -s public -o src/database/entity`
mod entity;

mod org_queries;
mod repo_label_queries;
mod repo_queries;
mod revision_label_queries;
mod revision_queries;

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

#[derive(Clone, Debug)]
pub enum DateTimeProvider {
    RealDateTime,
}

impl DateTimeProvider {
    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

#[derive(Debug)]
pub struct BackendDatabase {
    pub db: DatabaseConnection,
    pub date_time_provider: DateTimeProvider,
}

impl BackendDatabase {
    pub async fn new<S: Into<String>>(connection_url: S) -> prelude::DbResult<Self> {
        use std::time::Duration;

        let mut opts: ConnectOptions = ConnectOptions::new(connection_url.into());
        opts.max_connections(100)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(10));
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
    pub use super::revision_label_queries::{models::*, RevisionLabelQueries};
    pub use super::revision_queries::{models::*, RevisionQueries};
    pub use super::DbResult;
    pub use super::{
        AlreadyExistsError, BackendDatabase, DatabaseError, DateTimeProvider, NotFoundError,
    };
    pub use thiserror::Error;
}
