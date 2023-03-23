use crate::database::prelude::*;
use async_trait::async_trait;
pub use sea_orm::{entity::*, query::*, Database, DatabaseConnection, DbBackend, Schema};
use std::sync::Arc;

pub async fn make_backend() -> crate::Backend {
    let db = setup_schema().await.unwrap();
    let db_backend = BackendDatabase {
        db,
        date_time_provider: DateTimeProvider::RealDateTime,
    };

    Arc::new(crate::backend::DefaultBackend {
        database: db_backend,
        policy_container: Default::default(),
    })
}

pub async fn setup_schema() -> DbResult<DatabaseConnection> {
    use crate::database::prelude::*;
    let db = Database::connect("sqlite::memory:").await?;

    // Setup Schema helper
    let schema = Schema::new(DbBackend::Sqlite);

    // Derive from Entity
    db.execute(
        db.get_database_backend()
            .build(&schema.create_table_from_entity(Organization)),
    )
    .await?;

    db.execute(
        db.get_database_backend()
            .build(&schema.create_table_from_entity(Repository)),
    )
    .await?;

    db.execute(
        db.get_database_backend()
            .build(&schema.create_table_from_entity(RepositoryLabel)),
    )
    .await?;

    db.execute(
        db.get_database_backend()
            .build(&schema.create_table_from_entity(RepositoryRevision)),
    )
    .await?;

    db.execute(
        db.get_database_backend()
            .build(&schema.create_table_from_entity(RepositoryRevisionLabel)),
    )
    .await?;
    Ok(db)
}

#[allow(dead_code)]
pub fn logging_setup() {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::{
        fmt::format::{Format, PrettyFields},
        layer::SubscriberExt,
        Registry,
    };

    let logger = tracing_subscriber::fmt::layer()
        .event_format(Format::default().pretty())
        .fmt_fields(PrettyFields::new());

    let subscriber = Registry::default().with(LevelFilter::DEBUG).with(logger);

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing_log::LogTracer::init().expect("logging to work correctly")
}

pub trait SimpleString: ToString {}

#[async_trait]
pub trait TestBackend {
    async fn create_test_repo(&self, org: &str, repo: &str) -> DbResult<()>;
    async fn create_test_repo_with_params(
        &self,
        org: &str,
        repo: &str,
        create_param: CreateRepoParam,
    ) -> DbResult<()>;

    async fn create_test_version(&self, org: &str, repo: &str, version: &str) -> DbResult<()>;

    async fn create_test_org_and_repos(&self, org: &str, repos: Vec<&str>) -> DbResult<()>;
}

#[async_trait]
impl TestBackend for crate::Backend {
    async fn create_test_repo(&self, org: &str, repo: &str) -> DbResult<()> {
        self.database.create_test_repo(org, repo).await
    }

    async fn create_test_repo_with_params(
        &self,
        org: &str,
        repo: &str,
        create_param: CreateRepoParam,
    ) -> DbResult<()> {
        self.database
            .create_test_repo_with_params(org, repo, create_param)
            .await
    }

    async fn create_test_version(&self, org: &str, repo: &str, version: &str) -> DbResult<()> {
        self.database.create_test_version(org, repo, version).await
    }

    async fn create_test_org_and_repos(&self, org: &str, repos: Vec<&str>) -> DbResult<()> {
        self.database.create_test_org_and_repos(org, repos).await
    }
}

#[async_trait]
impl TestBackend for BackendDatabase {
    async fn create_test_repo(&self, org: &str, repo: &str) -> DbResult<()> {
        self.create_test_repo_with_params(org, repo, CreateRepoParam::default())
            .await
            .unwrap();

        Ok(())
    }

    async fn create_test_repo_with_params(
        &self,
        org: &str,
        repo: &str,
        create_param: CreateRepoParam,
    ) -> DbResult<()> {
        self.create_repo(&RepoParam::new(org, repo), create_param)
            .await
            .unwrap();

        Ok(())
    }

    async fn create_test_version(&self, org: &str, repo: &str, version: &str) -> DbResult<()> {
        self.create_revision(
            &RevisionParam::new(org, repo, version),
            &CreateRevisionParam {
                artifact_url: None,
                labels: vec![("version", version)].into(),
            },
        )
        .await
        .unwrap();

        Ok(())
    }

    async fn create_test_org_and_repos(&self, org: &str, repos: Vec<&str>) -> DbResult<()> {
        self.create_org(org).await.unwrap();
        for repo in repos {
            self.create_test_repo(org, repo).await.unwrap();
        }

        Ok(())
    }
}

pub fn assert_200_response(response: http::Response<bytes::Bytes>, expected_body: json::JsonValue) {
    use json::object;
    assert_response(
        response,
        http::StatusCode::OK,
        object! {
            "status": { "code": 200 },
            "data": expected_body
        },
    );
}

pub fn assert_200_list_response(
    response: http::Response<bytes::Bytes>,
    expected_body: json::JsonValue,
    total: usize,
    has_more: bool,
) {
    use json::object;
    assert_response(
        response,
        http::StatusCode::OK,
        object! {
            "status": { "code": 200 },
            "data": expected_body,
            "page": {
                "more": has_more,
                "total": total,
            }
        },
    );
}

pub fn assert_response(
    response: http::Response<bytes::Bytes>,
    status: http::StatusCode,
    expected_body: json::JsonValue,
) {
    let body = String::from_utf8(response.body().to_vec()).unwrap();
    println!("{:?}", body);
    let body = match json::parse(&body) {
        Err(e) => {
            println!("Unable to deserialize {:?}. Error: {:?}", body, e);
            unreachable!()
        }
        Ok(body) => body,
    };
    assert_eq!(json::stringify(body), json::stringify(expected_body));
    assert_eq!(response.status(), status);
}

pub fn assert_error_response(
    response: http::Response<bytes::Bytes>,
    status: http::StatusCode,
    message: &str,
) {
    use json::object;
    let body = String::from_utf8(response.body().to_vec()).unwrap();
    println!("{:?}", body);
    let body = match json::parse(&body) {
        Err(e) => {
            println!("Unable to deserialize {:?}. Error: {:?}", body, e);
            unreachable!()
        }
        Ok(body) => body,
    };
    assert_eq!(
        json::stringify(body),
        json::stringify(object! {
            "status": { "code": response.status().as_u16(), "error": [message] },
        })
    );
    assert_eq!(response.status(), status);
}
