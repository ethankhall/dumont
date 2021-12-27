use crate::database::{
    entity::{self, prelude::*},
    repo_queries::{RepoParam, RepoQueries},
    DbResult, PostresDatabase,
};
use async_trait::async_trait;
use sea_orm::{entity::*, query::*};
use std::collections::BTreeMap;
use tracing::info;
use tracing_attributes::instrument;

pub mod models {
    use crate::database::entity;
    use std::collections::BTreeMap;

    pub type RepoLabels = crate::database::models::GenericLabels;

    impl From<&Vec<entity::repository_label::Model>> for RepoLabels {
        fn from(source: &Vec<entity::repository_label::Model>) -> Self {
            let mut labels: BTreeMap<String, String> = Default::default();
            for value in source.iter() {
                labels.insert(value.label_name.to_string(), value.label_value.to_string());
            }

            Self { labels }
        }
    }

    impl From<Vec<entity::repository_label::Model>> for RepoLabels {
        fn from(source: Vec<entity::repository_label::Model>) -> Self {
            (&source).into()
        }
    }
}

pub use models::*;

#[async_trait]
pub trait RepoLabelQueries {
    async fn sql_set_repo_labels(
        &self,
        revision_id: i32,
        labels: &BTreeMap<String, String>,
    ) -> DbResult<()>;

    async fn set_repo_labels(
        &self,
        repo_param: &RepoParam<'_>,
        labels: BTreeMap<String, String>,
    ) -> DbResult<()>;

    async fn get_repo_labels(&self, repo_param: &RepoParam<'_>) -> DbResult<RepoLabels>;

    async fn sql_get_repo_labels(
        &self,
        repo: &entity::repository::Model,
    ) -> DbResult<Vec<entity::repository_label::Model>>;
}

#[async_trait]
impl RepoLabelQueries for PostresDatabase {
    #[instrument(level = "debug", skip(self))]
    async fn get_repo_labels(&self, repo_param: &RepoParam<'_>) -> DbResult<RepoLabels> {
        let repo = self
            .sql_get_repo(&repo_param.org_name, &repo_param.repo_name)
            .await?;

        let labels = self.sql_get_repo_labels(&repo).await?;
        Ok(labels.into())
    }

    #[instrument(level = "debug", skip(self))]
    async fn set_repo_labels(
        &self,
        repo_param: &RepoParam<'_>,
        labels: BTreeMap<String, String>,
    ) -> DbResult<()> {
        let repo = self
            .sql_get_repo(&repo_param.org_name, &repo_param.repo_name)
            .await?;

        self.sql_set_repo_labels(repo.repo_id, &labels).await
    }

    #[instrument(level = "debug", skip(self))]
    async fn sql_set_repo_labels(
        &self,
        repo_id: i32,
        labels: &BTreeMap<String, String>,
    ) -> DbResult<()> {
        let mut new_labels = Vec::default();

        for (key, value) in labels {
            new_labels.push(entity::repository_label::ActiveModel {
                repo_id: Set(repo_id),
                label_name: Set(key.to_string()),
                label_value: Set(value.to_string()),
                created_at: Set(self.date_time_provider.now().naive_utc()),
                ..Default::default()
            })
        }

        let new_label_count = new_labels.len();

        let txn = self.db.begin().await?;

        let del = RepositoryLabel::delete_many()
            .filter(entity::repository_label::Column::RepoId.eq(repo_id))
            .exec(&txn)
            .await?;
        if !new_labels.is_empty() {
            RepositoryLabel::insert_many(new_labels).exec(&txn).await?;
        }

        txn.commit().await?;

        info!(
            "Deleted {} rows, Inserted {} rows",
            del.rows_affected, new_label_count
        );
        Ok(())
    }

    #[instrument(level = "debug", skip(repo, self))]
    async fn sql_get_repo_labels(
        &self,
        repo: &entity::repository::Model,
    ) -> DbResult<Vec<entity::repository_label::Model>> {
        Ok(repo.find_related(RepositoryLabel).all(&self.db).await?)
    }
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::database::{
        common_tests::*, org_queries::OrganizationQueries, repo_queries::models::CreateRepoParam,
        DateTimeProvider,
    };
    use serial_test::serial;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn update_labels() {
        let db = PostresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        db.create_org("foo").await.unwrap();
        create_repo_with_params(
            &db,
            "foo",
            "bar",
            CreateRepoParam {
                labels: vec![("owner", "bobby tables")].into(),
            },
        )
        .await
        .unwrap();

        let metadata = db
            .get_repo_labels(&RepoParam::new("foo", "bar"))
            .await
            .unwrap();
        assert_eq!(metadata.labels.len(), 1);
        assert_eq!(metadata.labels.get("owner").unwrap(), "bobby tables");

        let mut labels = BTreeMap::new();
        labels.insert("scm_url".to_owned(), "https://google.com".to_owned());

        db.set_repo_labels(&RepoParam::new("foo", "bar"), labels)
            .await
            .unwrap();

        let labels = db
            .get_repo_labels(&RepoParam::new("foo", "bar"))
            .await
            .unwrap();
        assert_eq!(
            labels.labels.get("scm_url"),
            Some(&"https://google.com".to_owned())
        );
    }
}
