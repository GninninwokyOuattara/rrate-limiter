use std::sync::Arc;

use rrl_core::tokio_postgres::Client;

use crate::errors::ServiceError;

pub async fn route_exists(route: &str, client: Arc<Client>) -> Result<bool, ServiceError> {
    let exists = client
        .query("select true from rules where route = $1 limit 1", &[&route])
        .await?;

    match exists.len() {
        0 => Ok(true),
        _ => Err(ServiceError::AlreadyExistingRoute(route.to_string())),
    }
}
