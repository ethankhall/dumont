use crate::database::{
    entity::{self, prelude::*},
    revision_queries::{models::RevisionParam, RevisionQueries},
    DbResult, PostgresDatabase,
};
use async_trait::async_trait;
use sea_orm::{entity::*, query::*};
use std::collections::BTreeMap;
use tracing::info;
use tracing_attributes::instrument;

/**
 * RevisionLabelQueries is a collection of api calls against the database focused
 * on revisions's and their labels. This was split out of the RevisionQueries trait because
 * the trait was getting a little unwieldy to manage.
 */
#[async_trait]
pub trait RevisionLabelQueries {
    async fn set_revision_labels(
        &self,
        revision_param: &RevisionParam<'_>,
        labels: &BTreeMap<String, String>,
    ) -> DbResult<()>;

    async fn sql_set_revision_labels(
        &self,
        revision_id: i32,
        labels: &BTreeMap<String, String>,
    ) -> DbResult<()>;

    async fn get_revision_labels(
        &self,
        revision_param: &RevisionParam<'_>,
    ) -> DbResult<RevisionLabels>;

    async fn sql_get_revision_labels(
        &self,
        repo: &entity::repository_revision::Model,
    ) -> DbResult<Vec<entity::repository_revision_label::Model>>;

    async fn sql_get_revision_labels_by_revision_id(
        &self,
        revision_id: i64,
    ) -> DbResult<Vec<entity::repository_revision_label::Model>>;
}

pub mod models {
    use crate::database::entity::{self};
    use std::collections::BTreeMap;

    pub type RevisionLabels = crate::models::GenericLabels;

    impl From<&Vec<entity::repository_revision_label::Model>> for RevisionLabels {
        fn from(source: &Vec<entity::repository_revision_label::Model>) -> Self {
            let mut labels: BTreeMap<String, String> = Default::default();
            for value in source.iter() {
                labels.insert(value.label_name.to_string(), value.label_value.to_string());
            }

            labels.into()
        }
    }

    impl From<Vec<entity::repository_revision_label::Model>> for RevisionLabels {
        fn from(source: Vec<entity::repository_revision_label::Model>) -> Self {
            (&source).into()
        }
    }
}

use models::*;

#[async_trait]
impl RevisionLabelQueries for PostgresDatabase {
    #[instrument(skip(self))]
    async fn sql_get_revision_labels(
        &self,
        repo: &entity::repository_revision::Model,
    ) -> DbResult<Vec<entity::repository_revision_label::Model>> {
        let condition = Condition::all()
            .add(entity::repository_revision::Column::RevisionId.eq(repo.revision_id));

        Ok(RepositoryRevisionLabel::find()
            .join(
                JoinType::Join,
                entity::repository_revision_label::Relation::RepositoryRevision.def(),
            )
            .filter(condition)
            .all(&self.db)
            .await?)
    }

    #[instrument(skip(self))]
    async fn get_revision_labels(
        &self,
        revision_param: &RevisionParam<'_>,
    ) -> DbResult<RevisionLabels> {
        let revision = self.sql_get_revision(revision_param).await?;
        let labels = self.sql_get_revision_labels(&revision).await?;
        Ok(labels.into())
    }

    #[instrument(skip(self))]
    async fn sql_set_revision_labels(
        &self,
        revision_id: i32,
        labels: &BTreeMap<String, String>,
    ) -> DbResult<()> {
        let mut new_labels = Vec::default();

        for (key, value) in labels {
            new_labels.push(entity::repository_revision_label::ActiveModel {
                revision_id: Set(revision_id),
                label_name: Set(key.to_string()),
                label_value: Set(value.to_string()),
                created_at: Set(self.date_time_provider.now()),
                ..Default::default()
            })
        }

        let new_label_count = new_labels.len();

        let txn = self.db.begin().await?;

        let del = RepositoryRevisionLabel::delete_many()
            .filter(entity::repository_revision_label::Column::RevisionId.eq(revision_id))
            .exec(&txn)
            .await?;
        if !new_labels.is_empty() {
            RepositoryRevisionLabel::insert_many(new_labels)
                .exec(&txn)
                .await?;
        }

        txn.commit().await?;

        info!(
            "Deleted {} rows, Inserted {} rows",
            del.rows_affected, new_label_count
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn set_revision_labels(
        &self,
        revision_param: &RevisionParam<'_>,
        labels: &BTreeMap<String, String>,
    ) -> DbResult<()> {
        let revision = self.sql_get_revision(revision_param).await?;

        self.sql_set_revision_labels(revision.revision_id, labels)
            .await?;
        Ok(())
    }

    #[instrument(skip(self, revision_id))]
    async fn sql_get_revision_labels_by_revision_id(
        &self,
        revision_id: i64,
    ) -> DbResult<Vec<entity::repository_revision_label::Model>> {
        let labels = RepositoryRevisionLabel::find()
            .filter(entity::repository_revision_label::Column::RevisionId.eq(revision_id))
            .all(&self.db)
            .await?;
        Ok(labels)
    }
}

#[cfg(test)]
mod integ_test {
    use super::*;
    use crate::database::{revision_queries::models::CreateRevisionParam, DateTimeProvider};
    use crate::test_utils::*;
    use serial_test::serial;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_creating_revision_labels() {
        // let _logging = logging_setup();
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        create_org_and_repos(&db, "foo", vec!["bar"]).await.unwrap();

        let revision = db
            .create_revision(
                &RevisionParam::new("foo", "bar", "1.2.3"),
                &CreateRevisionParam {
                    artifact_url: None,
                    labels: vec![("key", "value"), ("foo", "bar")].into(),
                },
            )
            .await
            .unwrap();

        assert_eq!(revision.revision_name, "1.2.3");
        assert_eq!(revision.artifact_url, None);
        assert_eq!(revision.labels.len(), 2);
        assert_eq!(revision.labels.get("key").unwrap(), "value");
        assert_eq!(revision.labels.get("foo").unwrap(), "bar");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial]
    async fn test_updating_revision_labels() {
        // let _logging = logging_setup();
        let db = PostgresDatabase {
            db: setup_schema().await.unwrap(),
            date_time_provider: DateTimeProvider::RealDateTime,
        };

        create_org_and_repos(&db, "foo", vec!["bar"]).await.unwrap();

        let param = RevisionParam::new("foo", "bar", "1.2.3");

        let revision = db
            .create_revision(
                &param,
                &CreateRevisionParam {
                    artifact_url: None,
                    labels: vec![("key", "value"), ("foo", "bar")].into(),
                },
            )
            .await
            .unwrap();

        assert_eq!(revision.revision_name, "1.2.3");
        assert_eq!(revision.artifact_url, None);
        assert_eq!(revision.labels.len(), 2);
        assert_eq!(revision.labels.get("key").unwrap(), "value");
        assert_eq!(revision.labels.get("foo").unwrap(), "bar");

        db.set_revision_labels(
            &param,
            &BTreeMap::from_iter(vec![("fig".to_owned(), "value".to_owned())]),
        )
        .await
        .unwrap();

        let revision = db.get_revision(&param).await.unwrap();
        assert_eq!(revision.revision_name, "1.2.3");
        assert_eq!(revision.artifact_url, None);
        assert_eq!(revision.labels.len(), 1);
        assert_eq!(revision.labels.get("fig").unwrap(), "value");
    }
}
