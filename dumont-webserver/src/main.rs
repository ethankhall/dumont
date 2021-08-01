use crate::backend::DataStore;
use clap::Clap;
use std::sync::Arc;
use tracing::{Level};
use tracing_subscriber::{fmt::format::FmtSpan, FmtSubscriber};

mod api;
mod backend;

pub type Db = Arc<Box<dyn DataStore>>;

#[derive(Clap, Debug)]
#[clap(author, about, version)]
pub struct Opts {
    #[clap(subcommand)]
    pub sub_command: MainOperation,
}

#[derive(Clap, Debug)]
pub enum MainOperation {
    /// Run the web server
    #[clap(name = "serve")]
    RunWebServer(RunWebServerArgs),
}

#[derive(Clap, Debug)]
pub enum RunWebServerArgs {
    /// Create an in-memory database
    #[clap(name = "memory")]
    Memory(MemoryArgs),
}

#[derive(Clap, Debug)]
pub struct MemoryArgs {}

impl MemoryArgs {
    async fn as_backend(&self) -> Box<dyn DataStore> {
        use backend::MemDataStore;
        return Box::new(MemDataStore::default());
    }
}

impl RunWebServerArgs {
    async fn into_backend(self) -> Db {
        match &self {
            RunWebServerArgs::Memory(args) => Arc::new(args.as_backend().await),
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let opt = Opts::parse();

    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::INFO)
        // Record an event when each span closes. This can be used to time our
        // routes' durations!
        .with_span_events(FmtSpan::CLOSE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    match opt.sub_command {
        MainOperation::RunWebServer(args) => run_webserver(args).await,
    }
}

async fn run_webserver(args: RunWebServerArgs) {
    let db = args.into_backend().await;

    let filters = api::create_filters(db).await;

    warp::serve(filters).run(([127, 0, 0, 1], 3030)).await;
}
