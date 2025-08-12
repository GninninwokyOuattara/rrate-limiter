use axum::{Router, routing::get};

use rrl_core::Rule;
use tokio_postgres::NoTls;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres password=postgres dbname=rrate-limiter",
        NoTls,
    )
    .await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    tracing::debug!("Connected to Postgres");

    let result = client
        .query_one("SELECT * FROM rules WHERE route = $1", &[&"api/v1/users"])
        .await
        .unwrap();
    println!("ROWS: {:?}", result);
    // println!("row data {:?}", result.get::<&usize, String>(&0_usize));
    // let id: Uuid = result.get("id");
    // println!("id: {}", id);
    let rule: Rule = result.try_into()?;
    println!("row data {:?}", rule);

    // let algorithm: LimiterTrackingType = result.get("tracking_type");

    // println!("Algorithm :: {:?}", algorithm);

    let app = Router::new().route("/", get(async move || "Hello, World!"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
