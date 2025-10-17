use std::path::PathBuf;
use std::time::Duration;

use crate::{configurations_loader::load_configuration, server::run};
use clap::{Parser, Subcommand};
use opentelemetry::global;
use opentelemetry_appender_tracing::layer;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::PeriodicReader;
use opentelemetry_sdk::{Resource, metrics::SdkMeterProvider};
use tracing_subscriber::Layer;

use opentelemetry_otlp::Protocol;
use opentelemetry_otlp::WithExportConfig;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod configurations_loader;
mod errors;
mod handler;
mod rate_limiter;
mod rules;
mod server;
mod server_state;
mod utils;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "A simple and efficient rate limiter that supports five well-known algorithms: Fixed Window, Sliding Window Log, Sliding Window Counter, Leaky Bucket, and Token Bucket. 
It easy to setup, configure and is design to be easily scallable."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run as a rate limiter instance.
    Run,
    /// Load configuration file into the redis instance used by the rate limiters.
    Load {
        /// Path to the configuration file to be loaded
        #[arg(short, long)]
        file: PathBuf,
    },
}

fn init_oltp_metrics_provider() -> SdkMeterProvider {
    let otlp_host = std::env::var("RL_OTLP_HOST").unwrap_or("http://localhost:4318".to_string());
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(format!("{}/v1/metrics", otlp_host))
        .build()
        .expect("Failed to create metric exporter");

    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_millis(5000))
        .build();

    SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(
            Resource::builder()
                .with_service_name("rrate-limiter")
                .build(),
        )
        .build()
}

fn init_otlp_logs_provider() -> SdkLoggerProvider {
    let otlp_host = std::env::var("RL_OTLP_HOST").unwrap_or("http://localhost:4318".to_string());
    // let exporter = opentelemetry_stdout::LogExporter::default();
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(format!("{}/v1/logs", otlp_host))
        .build()
        .expect("Failed to create metric exporter");

    SdkLoggerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name("rrate-limiter")
                .build(),
        )
        .with_batch_exporter(exporter)
        .build()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let meter_provider = init_oltp_metrics_provider();
    global::set_meter_provider(meter_provider.clone());

    let log_provider = init_otlp_logs_provider();

    let filter_otel = tracing_subscriber::EnvFilter::new("info")
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());
    let otel_layer = layer::OpenTelemetryTracingBridge::new(&log_provider).with_filter(filter_otel);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(otel_layer)
        .init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Run => run().await?,
        Commands::Load { file } => load_configuration(file).await?,
    }

    Ok(())
}
