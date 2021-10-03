use crate::backend::DefaultBackend;
use clap::{AppSettings, ArgGroup, Clap};
use std::sync::Arc;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::format::{Format, JsonFields},
    layer::SubscriberExt,
    Registry,
};
use tracing_timing::{Builder, Histogram};

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

pub type Db = Arc<DefaultBackend>;

#[derive(Clap, Debug)]
#[clap(author, about, version)]
pub struct Opts {
    #[clap(subcommand)]
    pub sub_command: MainOperation,

    #[clap(flatten)]
    pub logging_opts: LoggingOpts,

    #[clap(flatten)]
    pub runtime_args: RuntimeArgs,
}

#[derive(Clap, Debug)]
pub enum MainOperation {
    /// Run the web server
    #[clap(name = "serve")]
    RunWebServer(RunWebServerArgs),
}

#[derive(Clap, Debug)]
pub struct RunWebServerArgs {
    /// Database Connection String
    #[clap(long = "database-url", env = "DB_CONNECTION")]
    db_connection_string: String,
}

#[derive(Clap, Debug)]
pub struct RuntimeArgs {
    /// The URL to publish metrics to.
    #[clap(
        long = "open-telem-collector",
        env = "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        default_value("http://localhost:4317")
    )]
    otel_collector: String,
}

#[derive(Clap, Debug)]
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
    dotenv::dotenv().ok();

    let opt = Opts::parse();

    global::set_text_map_propagator(TraceContextPropagator::new());
    let timing = Builder::default()
        .layer_informed(|_s: &_, _e: &_| Histogram::new_with_max(1_000_000, 2).unwrap());
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(IdGenerator::default())
                .with_resource(Resource::new(vec![KeyValue::new("service.name", "dumont")])),
        )
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(opt.runtime_args.otel_collector),
        )
        .install_batch(opentelemetry::runtime::Tokio)
        .unwrap();

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let console_output = tracing_subscriber::fmt::layer()
        .event_format(Format::default().json().flatten_event(true))
        .fmt_fields(JsonFields::new());

    let subscriber = Registry::default()
        .with(LevelFilter::from(opt.logging_opts))
        .with(otel_layer)
        .with(timing)
        .with(console_output);

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let _init = tracing_log::LogTracer::init().expect("logging to work correctly");

    match opt.sub_command {
        MainOperation::RunWebServer(args) => run_webserver(args).await,
    }
}

async fn run_webserver(args: RunWebServerArgs) -> Result<(), anyhow::Error> {
    let db = Arc::new(backend::DefaultBackend::new(args.db_connection_string).await?);

    let filters = api::create_filters(db).await;

    warp::serve(filters).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}
