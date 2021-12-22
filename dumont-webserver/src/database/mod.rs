// Generated with `sea-orm-cli generate entity -s public -o src/database/entity`
mod entity;
mod models;

mod org_queries;
mod repo_queries;
mod common_tests;

use sea_orm::{Database, DatabaseConnection};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotFoundError {
    #[error("{org} not found")]
    Organization { org: String },
    #[error("{org}/{repo} not found")]
    Repo { org: String, repo: String },
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
        let db: DatabaseConnection = Database::connect(&connection_url.into()).await?;

        Ok(Self { db })
    }
}

pub mod prelude {
    pub use super::entity::prelude::*;
    pub use thiserror::Error;
    pub type DbResult<T> = Result<T, DatabaseError>;
    pub use super::{AlreadyExistsError, NotFoundError, DatabaseError, PostresDatabase};
    pub use super::org_queries::OrganizationQueries;
    pub use super::repo_queries::RepoQueries;
    pub use super::models::*;
}