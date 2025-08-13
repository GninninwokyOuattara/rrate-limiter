use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_macros::debug_handler;
use rrl_core::{Rule, tokio_postgres::Client, uuid::Uuid};

use crate::{
    errors::ServiceError,
    models::{Pagination, PostedRule},
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
                &pagination.page_size,
                &(pagination.page_size * (pagination.page - 1)),
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
    println!("path id : {:#?}", rule_id);

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
) -> Result<impl IntoResponse, ()> {
    Ok(())
}

#[debug_handler]
pub async fn delete_rule() -> Result<impl IntoResponse, ()> {
    Ok(())
}

#[debug_handler]
pub async fn patch_rule() -> Result<impl IntoResponse, ()> {
    Ok(())
}
