use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use rrl_core::{
    Rule,
    tokio_postgres::Client,
    uuid::{self, Uuid},
};
use serde::Deserialize;

use crate::models::Pagination;

#[debug_handler]
pub async fn get_rules(
    pagination: Query<Pagination>,
    State(client): State<Arc<Client>>,
) -> Result<impl IntoResponse, ()> {
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
        .await
        .unwrap();
    // TODO: Consider cursor based pagination
    let rules: Vec<Rule> = result
        .into_iter()
        .map(|row| row.try_into().unwrap())
        .collect();

    Ok((axum::http::StatusCode::OK, Json(rules)))
}

#[debug_handler]
pub async fn get_rule_by_id(
    Path(rule_id): Path<Uuid>,
    State(client): State<Arc<Client>>,
) -> Result<impl IntoResponse, ()> {
    println!("path id : {:#?}", rule_id);

    let result = client
        .query_one(
            r#"
        select * from rules 
        where id = $1;
        "#,
            &[&rule_id],
        )
        .await
        .unwrap();

    let rule: Rule = result.try_into().unwrap();
    println!("rule : {:#?}", rule);
    Ok(Json(rule))
}

#[debug_handler]
pub async fn post_rule() -> Result<impl IntoResponse, ()> {
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
