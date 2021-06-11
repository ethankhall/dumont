mod db_models;

use async_trait::async_trait;
use db_models::{OrganizationDbModel, RepoDbModel};
use dumont_backend_base::{BackendDataStore, DataStoreError};
use dumont_models::{
    models::{Organization, Repository},
    operations::{CreateOrganization, CreateRepository, GetRepository},
};
use sqlx::{sqlite::SqlitePool, SqliteConnection};
use tracing::{info, trace};

pub struct SqlLiteDataStore {
    connection: SqlitePool,
}

impl SqlLiteDataStore {
    pub async fn new(url: &str) -> Self {
        info!("Connecting to {}", url);
        let connection = SqlitePool::connect(url).await.unwrap();

        sqlx::migrate!("./migrations")
            .run(&connection)
            .await
            .unwrap();

        Self { connection }
    }
}

impl SqlLiteDataStore {
    async fn _get_org_by_name(
        &self,
        connection: &mut SqliteConnection,
        org_name: &str,
    ) -> Result<OrganizationDbModel, DataStoreError> {
        sqlx::query_as!(
            OrganizationDbModel,
            "SELECT org_id, org_name FROM orginization WHERE org_name = ?1",
            org_name
        )
        .fetch_one(connection)
        .await
        .map_err(|e| e.into())
    }

    async fn _get_org_by_id(
        connection: &mut SqliteConnection,
        id: i64,
    ) -> Result<OrganizationDbModel, DataStoreError> {
        sqlx::query_as!(
            OrganizationDbModel,
            "SELECT org_id, org_name FROM orginization WHERE org_id = ?1",
            id
        )
        .fetch_one(connection)
        .await
        .map_err(|e| e.into())
    }

    async fn _get_repo_by_id(
        connection: &mut SqliteConnection,
        id: i64,
    ) -> Result<RepoDbModel, DataStoreError> {
        sqlx::query_as!(
            RepoDbModel,
            "SELECT org_id, repo_id, repo_name, url from repository where repo_id = ?1",
            id
        )
        .fetch_one(connection)
        .await
        .map_err(|e| e.into())
    }
}

#[async_trait]
impl BackendDataStore for SqlLiteDataStore {
    async fn create_organization(
        &self,
        entity: &CreateOrganization,
    ) -> Result<Organization, DataStoreError> {
        let mut connection = self.connection.acquire().await.unwrap();

        let result = sqlx::query_as!(
            OrganizationDbModel,
            "INSERT INTO orginization (org_name) VALUES (?1)",
            entity.organization
        )
        .execute(&mut connection)
        .await?;

        trace!("New entity created. ID {}", result.last_insert_rowid());

        let org =
            SqlLiteDataStore::_get_org_by_id(&mut connection, result.last_insert_rowid()).await?;
        Ok(OrganizationDbModel::into_org(&org))
    }

    async fn get_organizations(&self) -> Result<Vec<Organization>, DataStoreError> {
        let mut connection = self.connection.acquire().await.unwrap();

        let orgs = sqlx::query_as!(
            OrganizationDbModel,
            "SELECT org_id, org_name FROM `orginization`"
        )
        .fetch_all(&mut connection)
        .await?;

        trace!("Found {} orgs", orgs.len());
        Ok(orgs.iter().map(OrganizationDbModel::into_org).collect())
    }

    async fn create_repo(&self, entity: &CreateRepository) -> Result<Repository, DataStoreError> {
        let mut connection = self.connection.acquire().await.unwrap();
        let org = self
            ._get_org_by_name(&mut connection, &entity.organization)
            .await?;

        let repo = sqlx::query_as!(
            OrganizationDbModel,
            "INSERT INTO repository (org_id, repo_name, url) VALUES (?1, ?2, ?3)",
            org.org_id,
            entity.repository,
            entity.url
        )
        .execute(&mut connection)
        .await?;

        trace!("New entity created. ID {}", repo.last_insert_rowid());
        let repo =
            SqlLiteDataStore::_get_repo_by_id(&mut connection, repo.last_insert_rowid()).await?;
        Ok(RepoDbModel::into_repo(&org, &repo))
    }

    async fn get_repo(&self, entity: &GetRepository) -> Result<Option<Repository>, DataStoreError> {
        let mut connection = self.connection.acquire().await.unwrap();

        let resp = sqlx::query!(
            r#"SELECT r.repo_id, r.repo_name, r.url, o.org_id, o.org_name 
                FROM repository r 
                JOIN orginization o ON r.org_id = o.org_id 
                WHERE o.org_name = ?1 AND r.repo_name = ?2"#,
            entity.organization,
            entity.repository
        )
        .fetch_one(&mut connection)
        .await?;

        Ok(Some(Repository {
            name: resp.repo_name,
            id: resp.repo_id,
            url: resp.url,
            organization: Organization {
                id: resp.org_id,
                name: resp.org_name,
            },
        }))
    }
}
