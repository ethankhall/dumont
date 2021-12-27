// Generated with `sea-orm-cli generate entity -s public -o src/database/entity`
mod entity;

mod common_tests;
mod org_queries;
mod repo_queries;
mod reversion_queries;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotFoundError {
    #[error("{org} not found")]
    Organization { org: String },
    #[error("{org}/{repo} not found")]
    Repo { org: String, repo: String },
    #[error("{org}/{repo}/{revision} not found")]
    Revision { org: String, repo: String, revision: String },
    #[error("Repo with id {repo_id} not found")]
    RepoById { repo_id: i32 },
}

#[derive(Error, Debug)]
pub enum AlreadyExistsError {
    #[error("{org} exists")]
    Organization { org: String },
    #[error("{org}/{repo} exists")]
    Repo { org: String, repo: String },
    #[error("{org}/{repo}/{revision} exists")]
    Revision { org: String, repo: String, revision: String },
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error(transparent)]
    NotFound { error: NotFoundError },
    #[error(transparent)]
    AlreadyExists { error: AlreadyExistsError },
    #[error("Error withing backend: {source}")]
    BackendError {
        #[from]
        source: anyhow::Error,
        backtrace: std::backtrace::Backtrace,
    },
    #[error("Error when accessing database: {source}")]
    SeaOrmError {
        #[from]
        source: sea_orm::DbErr,
        backtrace: std::backtrace::Backtrace,
    },
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
    date_time_provider: DateTimeProvider
}

impl PostresDatabase {
    pub async fn new<S: Into<String>>(connection_url: S) -> prelude::DbResult<Self> {
        let mut opts: ConnectOptions = ConnectOptions::new(connection_url.into());
        opts.sqlx_logging(cfg!(debug_assertions) || cfg!(test_assertions));
        let db: DatabaseConnection = Database::connect(opts).await?;

        Ok(Self { db, date_time_provider: DateTimeProvider::RealDateTime })
    }
}

pub mod prelude {
    pub use super::entity::prelude::*;
    pub use thiserror::Error;
    pub type DbResult<T> = Result<T, DatabaseError>;
    pub use super::org_queries::{DbOrganization, OrganizationQueries, models::*};
    pub use super::repo_queries::{DbRepo, RepoQueries, models::*};
    pub use super::{AlreadyExistsError, DatabaseError, NotFoundError, PostresDatabase};
}
