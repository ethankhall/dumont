// Generated with `sea-orm-cli generate entity -s public -o src/database/entity`
mod entity;
mod models;

mod common_tests;
mod org_queries;
mod repo_queries;

use sea_orm::{Database, ConnectOptions, DatabaseConnection};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotFoundError {
    #[error("{org} not found")]
    Organization { org: String },
    #[error("{org}/{repo} not found")]
    Repo { org: String, repo: String },
    #[error("Repo with id {repo_id} not found")]
    RepoById { repo_id: i32 },
}

#[derive(Error, Debug)]
pub enum AlreadyExistsError {
    #[error("{org} exists")]
    Organization { org: String },
    #[error("{org}/{repo} exists")]
    Repo { org: String, repo: String },
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
}

pub struct PostresDatabase {
    db: DatabaseConnection,
}

impl PostresDatabase {
    pub async fn new<S: Into<String>>(connection_url: S) -> prelude::DbResult<Self> {
        let mut opts: ConnectOptions = ConnectOptions::new(connection_url.into());
            opts.sqlx_logging(cfg!(debug_assertions));
        let db: DatabaseConnection = Database::connect(opts).await?;

        Ok(Self { db })
    }
}

pub mod prelude {
    pub use super::entity::prelude::*;
    pub use thiserror::Error;
    pub type DbResult<T> = Result<T, DatabaseError>;
    pub use super::models::*;
    pub use super::org_queries::OrganizationQueries;
    pub use super::repo_queries::RepoQueries;
    pub use super::{AlreadyExistsError, DatabaseError, NotFoundError, PostresDatabase};
}
