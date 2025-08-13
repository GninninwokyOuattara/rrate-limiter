use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Pagination {
    pub page: i32,
    pub page_size: i32,
    pub route: Option<String>,
}
