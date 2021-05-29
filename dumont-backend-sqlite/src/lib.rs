mod db_models;

use async_trait::async_trait;
use db_models::OrganizationDbModel;
use dumont_backend_base::{BackendDataStore, DataStoreError};
use dumont_models::{models::Organization, operations::CreateOrganization};
use sqlx::sqlite::SqlitePool;
use sqlx::SqliteConnection;
use tracing::{error, info, trace};

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
    async fn _create_org(
        &self,
        connection: &mut SqliteConnection,
        entity: &CreateOrganization,
    ) -> Result<i64, DataStoreError> {
        let sql = sqlx::query_as!(
            OrganizationDbModel,
            "INSERT INTO orginization (org_name) VALUES (?1)",
            entity.name
        )
        .execute(connection)
        .await;

        match sql {
            Ok(value) => Ok(value.last_insert_rowid()),
            Err(e) => {
                error!("Unable to exec SQL: {:?}", e);
                Err(DataStoreError::BackendError { source: e.into() })
            }
        }
    }

    async fn _get_all_orgs(
        &self,
        connection: &mut SqliteConnection,
    ) -> Result<Vec<OrganizationDbModel>, DataStoreError> {
        let result = sqlx::query_as!(
            OrganizationDbModel,
            "select org_id, org_name from `orginization`"
        )
        .fetch_all(connection)
        .await;

        match result {
            Ok(orgs) => {
                trace!("Found {} orgs", orgs.len());
                Ok(orgs)
            }
            Err(e) => {
                error!("Unable to exec SQL: {:?}", e);
                Err(DataStoreError::BackendError { source: e.into() })
            }
        }
    }
}

#[async_trait]
impl BackendDataStore for SqlLiteDataStore {
    async fn create_organization(
        &self,
        entity: &CreateOrganization,
    ) -> Result<Organization, DataStoreError> {
        let mut connection = self.connection.acquire().await.unwrap();
        self._create_org(&mut connection, &entity).await.map(|x| {
            trace!("New entity created. ID {}", x);
            Organization {
                id: x,
                name: entity.name.clone(),
            }
        })
    }

    async fn get_organizations(&self) -> Result<Vec<Organization>, DataStoreError> {
        let mut connection = self.connection.acquire().await.unwrap();
        self._get_all_orgs(&mut connection).await.map(|orgs| {
            orgs.iter()
                .map(|x| Organization {
                    id: x.org_id,
                    name: x.org_name.clone(),
                })
                .collect()
        })
    }
}
