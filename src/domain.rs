/// # A bunch of stuff
use serde::{Deserialize, Serialize};
use crate::diesel::Queryable;
use crate::schema::*;


/// Source of articles, over da web
#[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
pub struct Flux {
    pub id: u32,
    pub name: String,
    pub url: String
}

