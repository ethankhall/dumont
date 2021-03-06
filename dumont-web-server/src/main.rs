use crate::backend::DefaultBackend;
use clap::{ArgGroup, Args, Parser, Subcommand};
use futures_util::join;
use std::sync::Arc;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::format::{Format, JsonFields, PrettyFields},
    layer::SubscriberExt,
    Registry,
};

use opentelemetry::{
    global,
    sdk::{
        propagation::TraceContextPropagator,
        trace::{self, IdGenerator, Sampler},
        Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;

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
    pub logging_opts: LoggingOpts,

    #[clap(flatten)]
    pub runtime_args: RuntimeArgs,
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

#[derive(Args, Debug)]
pub struct RuntimeArgs {
    /// The URL to publish metrics to.
    #[clap(
        long = "open-telem-collector",
        env = "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        default_value("http://localhost:4317")
    )]
    otel_collector: String,
}

#[derive(Parser, Debug)]
#[clap(group = ArgGroup::new("logging"))]
pub struct LoggingOpts {
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences), global(true), group = "logging")]
    pub debug: u64,

    /// Enable warn logging
    #[clap(short, long, global(true), group = "logging")]
    pub warn: bool,

    /// Disable everything but error logging
    #[clap(short, long, global(true), group = "logging")]
    pub error: bool,
}

impl From<LoggingOpts> for LevelFilter {
    fn from(opts: LoggingOpts) -> Self {
        if opts.error {
            LevelFilter::ERROR
        } else if opts.warn {
            LevelFilter::WARN
        } else if opts.debug == 0 {
            LevelFilter::INFO
        } else if opts.debug == 1 {
            LevelFilter::DEBUG
        } else {
            LevelFilter::TRACE
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    human_panic::setup_panic!();
    dotenv::dotenv().ok();

    let opt = Opts::parse();

    global::set_text_map_propagator(TraceContextPropagator::new());
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(opt.runtime_args.otel_collector),
        )
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(IdGenerator::default())
                .with_resource(Resource::new(vec![KeyValue::new("service.name", "dumont")])),
        )
        .install_batch(opentelemetry::runtime::Tokio)
        .unwrap();

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let is_terminal = atty::is(atty::Stream::Stdout) && cfg!(debug_assertions);
    let pretty_logger = if is_terminal {
        Some(
            tracing_subscriber::fmt::layer()
                .event_format(Format::default().pretty())
                .fmt_fields(PrettyFields::new()),
        )
    } else {
        None
    };

    let json_logger = if !is_terminal {
        Some(
            tracing_subscriber::fmt::layer()
                .event_format(Format::default().json().flatten_event(true))
                .fmt_fields(JsonFields::new()),
        )
    } else {
        None
    };

    let subscriber = Registry::default()
        .with(LevelFilter::from(opt.logging_opts))
        .with(otel_layer)
        .with(json_logger)
        .with(pretty_logger);

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let _init = tracing_log::LogTracer::init().expect("logging to work correctly");

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
