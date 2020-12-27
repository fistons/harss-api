/// # A bunch of stuff
use serde::{Deserialize, Serialize};
use crate::diesel::Queryable;

/// Source of articles, over da web
#[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
pub struct Flux {
    pub id: u32,
    pub url: String,
    pub title: String,
}

/// # Basic **article** 
pub struct Article {
    pub url: String,
    pub title: String,
    pub content: String,
}

