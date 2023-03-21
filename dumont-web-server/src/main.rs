use crate::backend::DefaultBackend;
use clap::{Args, Parser, Subcommand};
use futures_util::join;
use std::sync::Arc;

mod logging;
mod api;
mod backend;
mod database;
mod policy;
#[cfg(test)]
pub mod test_utils;

pub type Backend = Arc<DefaultBackend>;

pub mod models {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    use std::ops::Deref;

    #[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct GenericLabels {
        #[serde(default)]
        pub labels: BTreeMap<String, String>,
    }

    impl From<BTreeMap<String, String>> for GenericLabels {
        fn from(source: BTreeMap<String, String>) -> Self {
            Self { labels: source }
        }
    }

    impl From<Vec<(&str, &str)>> for GenericLabels {
        fn from(source: Vec<(&str, &str)>) -> Self {
            let mut labels: BTreeMap<String, String> = Default::default();
            for (key, value) in source {
                labels.insert(key.to_owned(), value.to_owned());
            }

            labels.into()
        }
    }

    impl Deref for GenericLabels {
        type Target = BTreeMap<String, String>;
        fn deref(&self) -> &Self::Target {
            &self.labels
        }
    }
}

#[derive(Parser, Debug)]
#[clap(author, about, version)]
pub struct Opts {
    #[clap(subcommand)]
    pub sub_command: MainOperation,

    #[clap(flatten)]
    pub logging_opts: logging::LoggingOpts,

    #[clap(flatten)]
    pub runtime_args: logging::RuntimeArgs,
}

#[derive(Subcommand, Debug)]
pub enum MainOperation {
    /// Run the web server
    #[clap(name = "web-server")]
    RunWebServer(RunWebServerArgs),

    /// Run the DB Migration
    #[clap(name = "db-migrate")]
    DatabaseMigration(RunDatabaseMigrationsArgs),
}

#[derive(Args, Debug)]
pub struct RunWebServerArgs {
    /// Database Connection String
    #[clap(long = "database-url", env = "DATABASE_URL")]
    db_connection_string: String,

    /// File that represents the policies that need to be applied to
    /// incoming edits.
    #[clap(long = "policy")]
    policy_document: Option<String>,

    /// Address to expose the main API on
    #[clap(long = "server-address", env = "SERVER_ADDRESS", default_value("127.0.0.1:3030"))]
    server_address: String,

    /// Address to expose the main API on
    #[clap(long = "admin-address", env = "ADMIN_ADDRESS", default_value("127.0.0.1:3031"))]
    admin_address: String,
}

#[derive(Args, Debug)]
pub struct RunDatabaseMigrationsArgs {
    /// Database Connection String
    #[clap(long = "database-url", env = "DATABASE_URL")]
    db_connection_string: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    human_panic::setup_panic!();
    dotenv::dotenv().ok();

    let opt = Opts::parse();

    logging::configure_logging(&opt.logging_opts, &opt.runtime_args);

    match opt.sub_command {
        MainOperation::RunWebServer(args) => run_webserver(args).await,
        MainOperation::DatabaseMigration(args) => run_db_migration(args).await,
    }
}

async fn run_db_migration(args: RunDatabaseMigrationsArgs) -> Result<(), anyhow::Error> {
    use sqlx::postgres::PgPoolOptions;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&args.db_connection_string)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(())
}

async fn run_webserver(args: RunWebServerArgs) -> Result<(), anyhow::Error> {
    use crate::policy::RealizedPolicyContainer;
    use warp::Filter;
    use std::net::SocketAddr;

    let policy_container: RealizedPolicyContainer = match args.policy_document {
        Some(path) => {
            let file_string = std::fs::read_to_string(path)?;
            let policy_container: crate::policy::PolicyDefinitionContainer =
                toml::from_str(&file_string)?;
            RealizedPolicyContainer::try_from(policy_container)?
        }
        None => Default::default(),
    };

    let backend =
        Arc::new(backend::DefaultBackend::new(args.db_connection_string, policy_container).await?);

    let filters = api::create_filters(backend).await;

    let api_addr: SocketAddr = args.server_address.parse()?;
    let api_server = warp::serve(filters).run(api_addr);

    let admin_server = warp::path("metrics")
        .map(api::metrics::metrics_endpoint)
        .or(warp::path("status").map(|| "OK"))
        .with(warp::trace::request());

    let admin_addr: SocketAddr = args.admin_address.parse()?;
    let admin_server = warp::serve(admin_server).run(admin_addr);

    let (_main, _admin) = join!(api_server, admin_server);

    Ok(())
}
