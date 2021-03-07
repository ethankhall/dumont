#![deny(clippy::all)]
// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

use std::collections::BTreeMap;

use clap::Clap;
use dotenv::dotenv;
use log::{error, trace};
use rusoto_s3::{S3Client, S3};

use dumont::{
    cli::{AwsData, LoggingOpts},
    configure_logging,
    errors::Error as DError,
};

#[derive(Clap, Debug)]
#[clap(author, about, version)]
pub struct Opts {
    #[clap(flatten)]
    pub logging_opts: LoggingOpts,

    #[clap(flatten)]
    pub aws_data: AwsData,

    #[clap(name = "BUCKET", required = true, min_values = 1)]
    pub buckets: Vec<String>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let opt = Opts::parse();
    configure_logging(&opt.logging_opts);

    let exit_code = match do_work(opt).await {
        Ok(code) => code,
        Err(e) => {
            error!("Unable to serach Object Storage. Error: {:?}", e);
            1
        }
    };

    std::process::exit(exit_code)
}

async fn do_work(opt: Opts) -> Result<i32, DError> {
    let s3_client = S3Client::new(opt.aws_data.to_region());
    let mut bucket_to_files: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for bucket in opt.buckets {
        trace!("Bucket: {}", &bucket);
        let files = get_files_in_bucket(&s3_client, &bucket).await?;
        bucket_to_files.insert(bucket, files);
    }

    trace!("Found Files: {:?}", bucket_to_files);

    Ok(0)
}

async fn get_files_in_bucket(client: &S3Client, bucket: &str) -> Result<Vec<String>, DError> {
    use rusoto_s3::ListObjectsRequest;

    let mut marker: Option<String> = None;
    let mut files = Vec::new();

    loop {
        let request = ListObjectsRequest {
            bucket: bucket.to_string(),
            marker,
            delimiter: None,
            encoding_type: None,
            expected_bucket_owner: None,
            max_keys: None,
            prefix: None,
            request_payer: None,
        };

        let results = client
            .list_objects(request)
            .await
            .map_err(|e| DError::with_chain(e, "Unable to fetch data from Object Storage."))?;

        marker = results.next_marker;
        if let Some(contents) = results.contents {
            for object in contents {
                if let Some(path) = object.key {
                    files.push(path);
                }
            }
        }

        if marker == None {
            break;
        }
    }

    Ok(files)
}
