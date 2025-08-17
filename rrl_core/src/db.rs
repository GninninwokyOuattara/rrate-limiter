// Return a route informations from the database.

use crate::tokio_postgres::Client;
use std::{error::Error, sync::Arc};

use crate::Rule;

pub async fn get_rule(route: &str, client: Arc<Client>) -> Result<Rule, Box<dyn Error>> {
    let result = client
        .query_one(
            r#"
            select * from rules where route = $1 limit 1;
            "#,
            &[&route],
        )
        .await?;

    Ok(result.try_into()?)
}
