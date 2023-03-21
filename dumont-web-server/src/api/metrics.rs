use opentelemetry::sdk::export::metrics::aggregation;
use opentelemetry::sdk::metrics::{controllers, processors, selectors};
use opentelemetry_prometheus::PrometheusExporter;
use prometheus::{Encoder, TextEncoder};

use warp::http::header::CONTENT_TYPE;

fn init_meter() -> PrometheusExporter {
    let controller = controllers::basic(
        processors::factory(
            selectors::simple::histogram([1.0, 2.0, 5.0, 10.0, 20.0, 50.0]),
            aggregation::cumulative_temporality_selector(),
        )
        .with_memory(true),
    )
    .build();

    opentelemetry_prometheus::exporter(controller).init()
}

#[tracing::instrument]
pub fn metrics_endpoint() -> impl warp::Reply {
    let exporter = init_meter();
    let encoder = TextEncoder::new();
    let metric_families = exporter.registry().gather();
    let mut result = Vec::new();
    encoder.encode(&metric_families, &mut result).ok();

    Ok(warp::reply::with_header(
        result,
        CONTENT_TYPE,
        encoder.format_type(),
    ))
}
