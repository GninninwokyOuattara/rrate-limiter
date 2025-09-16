use std::path::{Path, PathBuf};

use anyhow::Context;
use redis::Script;
use rrl_core::{
    LimiterTrackingType, RateLimiterAlgorithms, Rule, get_rules_route_and_id, tracing,
    tracing_subscriber,
};
use serde::Deserialize;
use serde_json::json;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Deserialize, Debug)]
struct Configuration {
    pub route: String,
    pub algorithm: RateLimiterAlgorithms,
    pub limit: i32,
    pub expiration: i32,
    pub tracking_type: LimiterTrackingType,
    pub custom_tracking_key: Option<String>,
    pub active: Option<bool>,
}

fn make_redis_script(rules: Vec<Rule>) -> Script {
    // Empty or initialize the rules hash
    let check_initialization = r"
        redis.call('JSON.SET', 'rules', '$', '{}')

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
                "active": rule.active
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start_time = std::time::Instant::now();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::debug!("Reading environment variables...");
    let redis_host = std::env::var("RL_REDIS_HOST").unwrap_or("localhost".to_string());
    let redis_port = std::env::var("RL_REDIS_PORT").unwrap_or("6379".to_string());
    let config_file =
        PathBuf::from(std::env::var("RL_CONFIG_FILE_PATH").unwrap_or("./config.yaml".to_string()));

    if !config_file.exists() {
        panic!("Configuration file does not exist.");
    }

    let file_extension = config_file.extension().unwrap_or_default();
    if file_extension != "yaml" && file_extension != "yml" {
        panic!("Configuration file must be a yaml file.");
    }

    tracing::info!("Reading configuration file...");
    let content =
        std::fs::read_to_string(AsRef::<Path>::as_ref(&config_file)).with_context(|| {
            format!(
                "Failed to read configuration file: {}",
                config_file.display()
            )
        })?;

    tracing::info!("connecting to redis...");
    let client = redis::Client::open(format!("redis://{}:{}", redis_host, redis_port))?;
    let mut con = client.get_connection()?;

    tracing::info!("Getting previous rules (route, id) pairs from redis...");
    let rules_to_ids = get_rules_route_and_id(&mut con).map_err(anyhow::Error::from_boxed)?;
    tracing::debug!("Previous rules :: {:#?}", rules_to_ids);

    tracing::info!("Parsing rules...");
    let rules: Vec<Rule> = serde_yaml::from_str::<Vec<Configuration>>(&content)
        .with_context(|| format!("Invalid configuration file."))?
        .into_iter()
        .map(|c| {
            if let Some(id) = rules_to_ids.get(&c.route) {
                tracing::debug!(
                    "- Route {} already exists with id {}. Id will be reused",
                    c.route,
                    id
                );
                return Rule {
                    id: id.clone(),
                    route: c.route,
                    algorithm: c.algorithm,
                    tracking_type: c.tracking_type,
                    limit: c.limit,
                    expiration: c.expiration,
                    custom_tracking_key: c.custom_tracking_key,
                    active: c.active,
                };
            }

            let rule = Rule::new(
                c.route,
                c.algorithm,
                c.limit,
                c.expiration,
                c.tracking_type,
                c.custom_tracking_key,
                c.active,
            );
            tracing::debug!("+ Route {} will be added with id {}", &rule.route, &rule.id);
            rule
        })
        .collect();

    tracing::info!("Processed {} rules.", rules.len());
    tracing::info!("Creating redis script...");
    let generated_script = make_redis_script(rules);
    tracing::debug!("Script :: {:#?}", generated_script);

    tracing::info!("Publishing script to redis store...");
    let _: () = generated_script.invoke(&mut con)?;

    let duration = start_time.elapsed();
    tracing::info!(
        "Configuration loaded succesfully in {}ms.",
        duration.as_millis()
    );
    Ok(())
}
