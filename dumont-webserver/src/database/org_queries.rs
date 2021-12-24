use super::prelude::*;
use crate::backend::models::PaginationOptions;
use crate::database::entity::{self, prelude::*};
use async_trait::async_trait;
use sea_orm::{entity::*, query::*};
use tracing_attributes::instrument;

#[async_trait]
pub trait OrganizationQueries {
    async fn create_org<T>(&self, org_name: T) -> DbResult<DbOrganizationModel>
    where
        T: ToString + Send;
    async fn sql_get_org(&self, org_name: &str) -> DbResult<entity::organization::Model>;
    async fn find_org<T>(&self, org_name: T) -> DbResult<DbOrganizationModel>
    where
        T: ToString + Send;
    async fn list_orgs(&self, pagination: PaginationOptions) -> DbResult<Vec<DbOrganizationModel>>;
    async fn delete_org<T>(&self, org_name: T) -> DbResult<bool>
    where
        T: ToString + Send;
}

#[async_trait]
impl OrganizationQueries for PostresDatabase {
    #[instrument(level = "debug", fields(org_name = %org_name.to_string()), skip(self, org_name))]
    async fn create_org<T>(&self, org_name: T) -> DbResult<DbOrganizationModel>
    where
        T: ToString + Send,
    {
        use entity::organization::{ActiveModel, Column};

        let org_name = org_name.to_string();

        let resp = Organization::find()
            .filter(Column::OrgName.eq(org_name.clone()))
            .count(&self.db)
            .await?;
        if resp != 0 {
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

    #[instrument(level = "debug", fields(org_name = %org_name.to_string()), skip(self, org_name))]
    async fn delete_org<T>(&self, org_name: T) -> DbResult<bool>
    where
        T: ToString + Send,
    {
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

    #[instrument(level = "debug", fields(org_name = %org_name.to_string()), skip(self, org_name))]
    async fn find_org<T>(&self, org_name: T) -> DbResult<DbOrganizationModel>
    where
        T: ToString + Send,
    {
        let org_name = org_name.to_string();

        let org = self.sql_get_org(&org_name).await?;

        Ok(DbOrganizationModel::from(org))
    }

    #[instrument(level = "debug", skip(self))]
    async fn list_orgs(&self, pagination: PaginationOptions) -> DbResult<Vec<DbOrganizationModel>> {
        use entity::organization::Column;

        let resp = Organization::find()
            .order_by_asc(Column::OrgId)
            .paginate(&self.db, pagination.page_size)
            .fetch_page(pagination.page_number)
            .await?;

        Ok(resp.iter().map(DbOrganizationModel::from).collect())
    }

    #[instrument(level = "debug", skip(self))]
    async fn sql_get_org(&self, org_name: &str) -> DbResult<entity::organization::Model> {
        use entity::organization::Column;

        let resp = Organization::find()
        .filter(Column::OrgName.eq(org_name.clone()))
        .one(&self.db)
        .await?;

        match resp {
            Some(org) => Ok(org),
            None => {
                return Err(DatabaseError::NotFound {
                    error: NotFoundError::Organization { org: org_name.to_owned() },
                })
            }
        }
    }
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::database::common_tests::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_orgs() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
        };

        let new_org = db.create_org("foo".to_owned()).await.unwrap();
        assert_eq!(new_org.org_name, "foo");

        let found_org = db.find_org("foo".to_owned()).await.unwrap();
        assert_eq!(found_org.org_name, "foo");

        match db.find_org("food".to_owned()).await {
            Err(DatabaseError::NotFound {
                error: NotFoundError::Organization { org },
            }) => assert_eq!(org, "food".to_owned()),
            failed => unreachable!("Should not have gotten {:?}", failed),
        }

        match db.create_org("foo".to_owned()).await {
            Err(DatabaseError::AlreadyExists {
                error: AlreadyExistsError::Organization { org },
            }) => {
                assert_eq!(org, "foo");
            }
            failed => unreachable!("Should not have gotten {:?}", failed),
        }

        let new_org = db.create_org("bar".to_owned()).await.unwrap();
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
    async fn test_org_pagination() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
        };

        for i in 0..100 {
            db.create_org(format!("org-{}", i)).await.unwrap();
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
