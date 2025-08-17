// Return a route informations from the database.

use chrono::Utc;

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

// Get the latest update date for the rules, it will be the cursor
// for the pagination.
pub async fn get_last_update_time(
    client: Arc<Client>,
) -> Result<Option<chrono::DateTime<Utc>>, Box<dyn Error>> {
    let result = client
        .query(
            r#"
            select max(date_modification) from rules;
            "#,
            &[],
        )
        .await?;
    if result.is_empty() {
        Ok(None)
    } else {
        let time: chrono::DateTime<Utc> = result.first().unwrap().get(0);
        Ok(Some(time))
    }
}

//  get all rules updated at or after the cursor (date_modification)
pub async fn get_all_rules_updated_at_and_after_date(
    client: Arc<Client>,
    date: chrono::DateTime<Utc>,
) -> Result<Vec<Rule>, Box<dyn Error>> {
    let result = client
        .query(
            r#"
        select * from rules 
        where date_modification >= $1
        order by route asc;
        "#,
            &[&date],
        )
        .await?;

    let rules = result
        .into_iter()
        .map(|row| row.try_into())
        .collect::<Result<Vec<Rule>, Box<dyn std::error::Error>>>()?;

    Ok(rules)
}
