use crate::backend::models::PaginationOptions;
use crate::database::{
    entity::{self, prelude::*},
    AlreadyExistsError, DatabaseError, DbResult, NotFoundError, PostgresDatabase,
};
use async_trait::async_trait;
use sea_orm::{entity::*, query::*};
use tracing_attributes::instrument;

pub trait DbOrganization {
    fn get_org_id(&self) -> i32;
    fn get_org_name(&self) -> String;
}

pub mod models {
    use crate::database::entity;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DbOrganizationModel {
        pub org_id: i32,
        pub org_name: String,
    }

    impl super::DbOrganization for DbOrganizationModel {
        fn get_org_id(&self) -> i32 {
            self.org_id
        }
        fn get_org_name(&self) -> String {
            self.org_name.clone()
        }
    }

    impl From<&entity::organization::Model> for DbOrganizationModel {
        fn from(org: &entity::organization::Model) -> Self {
            Self {
                org_id: org.org_id,
                org_name: org.org_name.clone(),
            }
        }
    }

    impl From<entity::organization::Model> for DbOrganizationModel {
        fn from(org: entity::organization::Model) -> Self {
            (&org).into()
        }
    }
}

pub use models::*;

/**
 * OrganizationQueries is a collection of api calls against the database focused
 * on the "organization".
 *
 * In general this is a CRUD API, with access to some lower level API's for other
 * traits to use to use the ORM.
 */
#[async_trait]
pub trait OrganizationQueries {
    async fn create_org(&self, org_name: &str) -> DbResult<DbOrganizationModel>;
    async fn sql_get_raw_org(
        &self,
        org_name: &str,
    ) -> DbResult<Option<entity::organization::Model>>;
    async fn sql_get_org(&self, org_name: &str) -> DbResult<entity::organization::Model>;
    async fn find_org(&self, org_name: &str) -> DbResult<DbOrganizationModel>;
    async fn list_orgs(&self, pagination: PaginationOptions) -> DbResult<Vec<DbOrganizationModel>>;
    async fn delete_org(&self, org_name: &str) -> DbResult<bool>;
}

#[async_trait]
impl OrganizationQueries for PostgresDatabase {
    #[instrument(skip(self))]
    async fn create_org(&self, org_name: &str) -> DbResult<DbOrganizationModel> {
        use entity::organization::ActiveModel;

        let org_name = org_name.to_string();

        if let Some(_org) = self.sql_get_raw_org(&org_name).await? {
            return Err(DatabaseError::AlreadyExists {
                error: AlreadyExistsError::Organization {
                    org: org_name.clone(),
                },
            });
        }

        let model = ActiveModel {
            org_name: Set(org_name),
            ..Default::default()
        };

        let res: InsertResult<ActiveModel> = Organization::insert(model).exec(&self.db).await?;
        let model = Organization::find_by_id(res.last_insert_id)
            .one(&self.db)
            .await?
            .unwrap();
        Ok(DbOrganizationModel::from(model))
    }

    #[instrument(skip(self))]
    async fn delete_org(&self, org_name: &str) -> DbResult<bool> {
        use entity::organization::Column;
        let org_name = org_name.to_string();

        let resp = Organization::delete_many()
            .filter(Column::OrgName.eq(org_name.clone()))
            .exec(&self.db)
            .await?;

        if resp.rows_affected == 0 {
            return Err(DatabaseError::NotFound {
                error: NotFoundError::Organization { org: org_name },
            });
        }

        Ok(true)
    }

    #[instrument(skip(self))]
    async fn find_org(&self, org_name: &str) -> DbResult<DbOrganizationModel> {
        let org_name = org_name.to_string();

        let org = self.sql_get_org(&org_name).await?;

        Ok(DbOrganizationModel::from(org))
    }

    #[instrument(skip(self))]
    async fn list_orgs(&self, pagination: PaginationOptions) -> DbResult<Vec<DbOrganizationModel>> {
        use entity::organization::Column;

        let resp = Organization::find()
            .order_by_asc(Column::OrgId)
            .paginate(&self.db, pagination.page_size)
            .fetch_page(pagination.page_number)
            .await?;

        Ok(resp.iter().map(DbOrganizationModel::from).collect())
    }

    #[instrument(skip(self))]
    async fn sql_get_raw_org(
        &self,
        org_name: &str,
    ) -> DbResult<Option<entity::organization::Model>> {
        use entity::organization::Column;

        let resp = Organization::find()
            .filter(Column::OrgName.eq(org_name.clone()))
            .one(&self.db)
            .await?;

        Ok(resp)
    }

    #[instrument(skip(self))]
    async fn sql_get_org(&self, org_name: &str) -> DbResult<entity::organization::Model> {
        match self.sql_get_raw_org(org_name).await? {
            Some(org) => Ok(org),
            None => {
                return Err(DatabaseError::NotFound {
                    error: NotFoundError::Organization {
                        org: org_name.to_owned(),
                    },
                })
            }
        }
    }
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::database::DateTimeProvider;
    use crate::test_utils::*;
    use serial_test::serial;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_orgs() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        let new_org = db.create_org("foo").await.unwrap();
        assert_eq!(new_org.org_name, "foo");

        let found_org = db.find_org("foo").await.unwrap();
        assert_eq!(found_org.org_name, "foo");

        match db.find_org("food").await {
            Err(DatabaseError::NotFound {
                error: NotFoundError::Organization { org },
            }) => assert_eq!(org, "food".to_owned()),
            failed => unreachable!("Should not have gotten {:?}", failed),
        }

        match db.create_org("foo").await {
            Err(DatabaseError::AlreadyExists {
                error: AlreadyExistsError::Organization { org },
            }) => {
                assert_eq!(org, "foo");
            }
            failed => unreachable!("Should not have gotten {:?}", failed),
        }

        let new_org = db.create_org("bar").await.unwrap();
        assert_eq!(new_org.org_name, "bar");

        let listed_orgs = db.list_orgs(PaginationOptions::new(0, 50)).await.unwrap();
        assert_eq!(listed_orgs.len(), 2);
        assert_eq!(listed_orgs[0].org_name, "foo");
        assert_eq!(listed_orgs[1].org_name, "bar");

        // Get from page that doesn't exist
        assert_eq!(
            db.list_orgs(PaginationOptions::new(1, 50))
                .await
                .unwrap()
                .len(),
            0
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_org_pagination() {
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        for i in 0..100 {
            db.create_org(&format!("org-{}", i)).await.unwrap();
        }

        let found_orgs = db.list_orgs(PaginationOptions::new(0, 50)).await.unwrap();
        assert_eq!(found_orgs.len(), 50);

        for i in 0..50 {
            assert_eq!(found_orgs[i].org_name, format!("org-{}", i));
        }

        let found_orgs = db.list_orgs(PaginationOptions::new(1, 50)).await.unwrap();
        assert_eq!(found_orgs.len(), 50);

        for i in 0..50 {
            assert_eq!(found_orgs[i].org_name, format!("org-{}", i + 50));
        }

        let found_orgs = db.list_orgs(PaginationOptions::new(2, 50)).await.unwrap();
        assert_eq!(found_orgs.len(), 0);
    }
}
