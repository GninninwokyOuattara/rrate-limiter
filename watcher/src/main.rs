use std::sync::Arc;

use redis::{Script, aio::ConnectionManager};
use rrl_core::{
    Rule,
    chrono::{self, DateTime},
    db::get_all_rules_updated_at_and_after_date,
    tokio_postgres::NoTls,
    tracing, tracing_subscriber,
};
use serde_json::json;
use tokio::time::{self, Duration};
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

    tracing::debug!("Reading environment variables...");

    let host = std::env::var("RL_POSTGRES_HOST").unwrap_or("localhost".to_string());
    let port = std::env::var("RL_POSTGRES_PORT").unwrap_or("5432".to_string());
    let user = std::env::var("RL_POSTGRES_USER").unwrap_or("postgres".to_string());
    let password = std::env::var("RL_POSTGRES_PASSWORD").unwrap_or("postgres".to_string());
    let redis_host = std::env::var("RL_REDIS_HOST").unwrap_or("localhost".to_string());
    let redis_port = std::env::var("RL_REDIS_PORT").unwrap_or("6379".to_string());

    tracing::debug!("Connecting to postgres...");

    let (pg_client, connection) = rrl_core::tokio_postgres::connect(
        format!("host={host} port={port} user={user} password={password} dbname=rrate-limiter")
            .as_str(),
        NoTls,
    )
    .await?;
    let pg_client = Arc::new(pg_client);
    tracing::debug!("Connected to postgres. Spawning pg connection manager task...");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    tracing::debug!("connecting to redis...");
    let client = redis::Client::open(format!("redis://{}:{}", redis_host, redis_port))?;
    let mut redis_client = ConnectionManager::new(client).await?;

    tracing::debug!("Managed connection to redis established.");

    let mut interval = time::interval(Duration::from_secs(60));

    let mut cursor = DateTime::default(); // Starting timer way back so we get everything on first launch.

    loop {
        interval.tick().await;
        tracing::debug!("Checking for new at or after updates {:?}...", cursor);
        let maybe_rules = get_all_rules_updated_at_and_after_date(pg_client.clone(), cursor).await;
        if maybe_rules.is_err() {
            tracing::error!("An error occured :: {}", maybe_rules.unwrap_err());
            continue;
        }

        let rules = maybe_rules.unwrap();
        if !rules.is_empty() {
            tracing::info!("Found {} rules to update", rules.len());
            tracing::debug!("Rules :: {:#?}", rules);

            let maybe_cursor =
                rules.last().unwrap().date_modification + chrono::Duration::microseconds(1);
            let generated_script = make_redis_script(rules);
            tracing::debug!("Script :: {:#?}", generated_script);

            let result: Result<(), redis::RedisError> =
                generated_script.invoke_async(&mut redis_client).await;

            if result.is_err() {
                tracing::error!("An error occured :: {}", result.unwrap_err());
                continue;
            }

            cursor = maybe_cursor;
            tracing::info!("Updating cursor to: {}", cursor);
        } else {
            tracing::debug!("No new rules found.");
        }

        tracing::debug!("Sleeping for 60 seconds...");
    }
}

fn make_redis_script(rules: Vec<Rule>) -> Script {
    // Create the root if it does not exists.
    let check_initialization = r"
        local success, objlen_result = pcall(redis.call, 'JSON.OBJLEN', 'rules', '$')
        if success == false then
            redis.call('JSON.SET', 'rules', '$', '{}')
        end

    ";

    // Build the rules, redis call after redis call
    let mut script_rules: Vec<String> = vec![];
    script_rules.push(check_initialization.to_string());

    rules.into_iter().for_each(|rule| {
        let id = rule.id.clone().to_string();
        let rule_json = json!(
            {
                "id": rule.id,
                "route": rule.route,
                "algorithm": rule.algorithm.to_string(),
                "tracking_type": rule.tracking_type.to_string(),
                "limit": rule.limit,
                "expiration": rule.expiration,
                "custom_tracking_key": rule.custom_tracking_key.unwrap_or("".to_string()),
                "status": rule.status.to_string(),
                "date_creation": rule.date_creation,
                "date_modification": rule.date_modification
            }
        );

        let script = format!(r#"redis.call('JSON.SET', 'rules', '$.{id}' , '{rule_json}')"#);
        script_rules.push(script);
    });

    // Finilize the script by publishing the update
    let publish = "redis.call('PUBLISH', 'rl_update', 'update')".to_string();
    let script_rules = script_rules.join("\n");
    Script::new(&format!("{}\n{}", script_rules, publish))
}
