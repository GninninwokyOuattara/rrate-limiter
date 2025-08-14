use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use rrl_core::{Rule, postgres_types::ToSql, tokio_postgres::Client, uuid::Uuid};

use crate::{
    errors::ServiceError,
    models::{Pagination, PatchedRule, PostedRule},
};

#[debug_handler]
pub async fn get_rules(
    pagination: Query<Pagination>,
    State(client): State<Arc<Client>>,
) -> Result<impl IntoResponse, ServiceError> {
    let result = client
        .query(
            r#"
            select * from rules 
            where route ~* $1
            order by route asc
            limit $2::int4
            offset $3::int4;
            "#,
            &[
                &format!("^{}", pagination.route.clone().unwrap_or_default()),
                &(pagination.page_size as i32),
                &(pagination.page_size as i32 * (pagination.page as i32 - 1)),
            ],
        )
        .await?;
    // TODO: Consider cursor based pagination
    let rules = result
        .into_iter()
        .map(|row| row.try_into())
        .collect::<Result<Vec<Rule>, Box<dyn std::error::Error>>>()?;

    Ok(Json(rules))
}

#[debug_handler]
pub async fn get_rule_by_id(
    Path(rule_id): Path<Uuid>,
    State(client): State<Arc<Client>>,
) -> Result<impl IntoResponse, ServiceError> {
    let result = client
        .query(
            r#"
        select * from rules 
        where id = $1
        limit 1;
        "#,
            &[&rule_id],
        )
        .await?;

    if result.is_empty() {
        return Err(ServiceError::RuleNotFound(rule_id));
    }

    let rule: Rule = result.get(0).unwrap().to_owned().try_into()?;
    Ok(Json(rule))
}

#[debug_handler]
pub async fn post_rule(
    State(client): State<Arc<Client>>,
    Json(rule): Json<PostedRule>,
) -> Result<impl IntoResponse, ServiceError> {
    let custom_key = if let Some(key) = rule.custom_tracking_key
        && !key.is_empty()
    {
        Some(key)
    } else {
        None
    };

    let _result = client
        .query(
            r#"
            insert into rules (route, "limit", expiration, algorithm, tracking_type, custom_tracking_key, status, ttl)
            values ($1, $2, $3, $4, $5, $6, $7, $8 );
            "#,
            &[&rule.route, &(rule.limit as i32), &(rule.expiration as i32), &rule.algorithm, &rule.tracking_type, &custom_key, &rule.status, &(rule.ttl as i32)],
        )
        .await?;

    Ok(())
}

#[debug_handler]
pub async fn delete_rule(
    Path(rule_id): Path<Uuid>,
    State(client): State<Arc<Client>>,
) -> Result<impl IntoResponse, ServiceError> {
    let _result = client
        .query(
            r#"
            delete from rules where id = $1;
            "#,
            &[&rule_id],
        )
        .await?;
    Ok(())
}

#[debug_handler]
pub async fn patch_rule(
    Path(rule_id): Path<Uuid>,
    State(client): State<Arc<Client>>,
    Json(rule): Json<PatchedRule>,
) -> Result<impl IntoResponse, ServiceError> {
    let route_field;
    let limit_field;
    let expiration_field;
    let algorithm_field;
    let tracking_type_field;
    let custom_tracking_key_field;
    let status_field;
    let ttl_field;

    let mut fields: Vec<String> = Vec::new();
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![];

    params.push(&rule_id);
    let mut i = 2;

    if let Some(route) = rule.route {
        route_field = route;
        fields.push(format!("route = ${}", i));
        params.push(&route_field);
        i += 1;
    }

    if let Some(limit) = rule.limit {
        fields.push(format!("limit = ${}", i));
        limit_field = limit as i32;
        params.push(&limit_field);
        i += 1;
    }

    if let Some(expiration) = rule.expiration {
        fields.push(format!("expiration = ${}", i));
        expiration_field = expiration as i32;
        params.push(&expiration_field);
        i += 1;
    }

    if let Some(algorithm) = rule.algorithm {
        fields.push(format!("algorithm = ${}", i));
        algorithm_field = algorithm;
        params.push(&algorithm_field);
        i += 1;
    }

    if let Some(tracking_type) = rule.tracking_type {
        fields.push(format!("tracking_type = ${}", i));
        tracking_type_field = tracking_type;
        params.push(&tracking_type_field);
        i += 1;
    }

    if let Some(custom_tracking_key) = rule.custom_tracking_key {
        fields.push(format!("custom_tracking_key = ${}", i));
        custom_tracking_key_field = custom_tracking_key;
        params.push(&custom_tracking_key_field);
        i += 1;
    }

    if let Some(status) = rule.status {
        fields.push(format!("status = ${}", i));
        status_field = status;
        params.push(&status_field);
        i += 1;
    }

    if let Some(ttl) = rule.ttl {
        fields.push(format!("ttl = ${}", i));
        ttl_field = ttl as i32;
        params.push(&ttl_field);
        // i += 1;
    }
    fields.push(format!("date_modification = now()"));

    let query_string = format!(
        "update rules set {} where id = $1 returning *;",
        fields.join(", ")
    );

    let result = client.query(&query_string, &params).await?;

    if result.len() == 0 {
        return Err(ServiceError::RuleNotFound(rule_id));
    }

    let rule: Rule = result
        .get(0)
        .ok_or_else(|| ServiceError::RuleNotFound(rule_id))?
        .to_owned()
        .try_into()?;

    Ok(Json(rule))
}
