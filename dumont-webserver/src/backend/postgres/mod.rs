mod db_models;

use super::{DataStore, DataStoreError};
use async_trait::async_trait;
use db_models::{OrganizationDbModel, RepoDbModel};
use dumont_models::{
    models::{Organization, Repository},
    operations::{CreateOrganization, CreateRepository, GetRepository},
};
use sqlx::{Pool, Sqlite};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use tracing::{info, trace};

pub struct SqlLiteDataStore {
    // tx: Arc<Mutex<sqlx::Transaction<'a, Sqlite>>>,
}

impl SqlLiteDataStore {
    async fn _get_org_by_name<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, Sqlite>,
        org_name: &str,
    ) -> Result<OrganizationDbModel, DataStoreError> {
        sqlx::query_as!(
            OrganizationDbModel,
            "SELECT org_id, org_name FROM orginization WHERE org_name = ?1",
            org_name
        )
        .fetch_one(tx)
        .await
        .map_err(|e| e.into())
    }

    async fn _get_org_by_id<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, Sqlite>,
        id: i64,
    ) -> Result<OrganizationDbModel, DataStoreError> {
        sqlx::query_as!(
            OrganizationDbModel,
            "SELECT org_id, org_name FROM orginization WHERE org_id = ?1",
            id
        )
        .fetch_one(tx)
        .await
        .map_err(|e| e.into())
    }

    async fn _get_repo_by_id<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, Sqlite>,
        id: i64,
    ) -> Result<RepoDbModel, DataStoreError> {
        sqlx::query_as!(
            RepoDbModel,
            "SELECT org_id, repo_id, repo_name, url from repository where repo_id = ?1",
            id
        )
        .fetch_one(tx)
        .await
        .map_err(|e| e.into())
    }
}

#[async_trait]
impl DataStore<Sqlite> for SqlLiteDataStore {
    async fn create_organization<'b>(
        &self,
        tx: &mut sqlx::Transaction<'b, Sqlite>,
        entity: &CreateOrganization,
    ) -> Result<Organization, DataStoreError> {
        let result = sqlx::query_as!(
            OrganizationDbModel,
            "INSERT INTO orginization (org_name) VALUES (?1)",
            entity.organization
        )
        .execute(tx)
        .await?;

        trace!("New entity created. ID {}", result.last_insert_rowid());

        let org = self._get_org_by_id(tx, result.last_insert_rowid()).await?;
        Ok(OrganizationDbModel::into_org(&org))
    }

    async fn get_organizations<'b>(
        &self,
        tx: &mut sqlx::Transaction<'b, Sqlite>,
    ) -> Result<Vec<Organization>, DataStoreError> {
        let orgs = sqlx::query_as!(
            OrganizationDbModel,
            "SELECT org_id, org_name FROM `orginization`"
        )
        .fetch_all(tx)
        .await?;

        trace!("Found {} orgs", orgs.len());
        Ok(orgs.iter().map(OrganizationDbModel::into_org).collect())
    }

    async fn create_repo<'b>(
        &self,
        tx: &sqlx::Transaction<'b, Sqlite>,
        entity: &CreateRepository,
    ) -> Result<Repository, DataStoreError> {
        let org = self._get_org_by_name(&mut tx, &entity.organization).await?;

        let repo = sqlx::query_as!(
            OrganizationDbModel,
            "INSERT INTO repository (org_id, repo_name, url) VALUES (?1, ?2, ?3)",
            org.org_id,
            entity.repository,
            entity.url
        )
        .execute(&mut tx)
        .await?;

        trace!("New entity created. ID {}", repo.last_insert_rowid());
        let repo = self
            ._get_repo_by_id(&mut tx, repo.last_insert_rowid())
            .await?;
        Ok(RepoDbModel::into_repo(&org, &repo))
    }

    async fn get_repo<'b>(
        &self,
        tx: &mut sqlx::Transaction<'b, Sqlite>,
        entity: &GetRepository,
    ) -> Result<Option<Repository>, DataStoreError> {
        let resp = sqlx::query!(
            r#"SELECT r.repo_id, r.repo_name, r.url, o.org_id, o.org_name 
                FROM repository r 
                JOIN orginization o ON r.org_id = o.org_id 
                WHERE o.org_name = ?1 AND r.repo_name = ?2"#,
            entity.organization,
            entity.repository
        )
        .fetch_one(tx)
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
