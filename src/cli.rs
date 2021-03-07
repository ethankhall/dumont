use clap::{ArgGroup, Clap};
use log::LevelFilter;

use rusoto_core::Region;
use std::str::FromStr;

#[derive(Clap, Debug)]
#[clap(group = ArgGroup::new("logging"))]
pub struct LoggingOpts {
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences), global(true), group = "logging")]
    pub verbose: u64,

    /// Enable warn logging
    #[clap(short, long, global(true), group = "logging")]
    pub warn: bool,

    /// Disable everything but error logging
    #[clap(short, long, global(true), group = "logging")]
    pub error: bool,
}

impl LoggingOpts {
    pub fn to_level_filter(&self) -> LevelFilter {
        if self.error {
            LevelFilter::Error
        } else if self.warn {
            LevelFilter::Warn
        } else if self.verbose == 0 {
            LevelFilter::Info
        } else if self.verbose == 1 {
            LevelFilter::Debug
        } else {
            LevelFilter::Trace
        }
    }

    pub fn to_dep_level_filter(&self) -> LevelFilter {
        match self.verbose {
            0 | 1 => LevelFilter::Off,
            2 => LevelFilter::Info,
            _ => LevelFilter::Debug
        }
    }
}

#[derive(Clap, Debug)]
pub struct AwsData {
    #[clap(long, global(true), env = "AWS_DEFAULT_REGION")]
    pub region: Option<String>,

    #[clap(long, global(true), env = "AWS_DEFAULT_ENDPOINT", requires("region"))]
    pub endpoint: Option<String>,
}

impl AwsData {
    pub fn to_region(self) -> Region {
        if let Some(endpoint) = self.endpoint {
            return Region::Custom {
                name: self.region.unwrap(),
                endpoint: endpoint.clone(),
            };
        }
        self.region
            .and_then(|x| Region::from_str(&x).ok())
            .unwrap_or_else(Region::default)
    }
}
