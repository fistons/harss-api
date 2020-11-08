/// # A bunch of stuff
use serde::{Deserialize, Serialize};


/// Source of articles, over da web
#[derive(Debug, Serialize, Deserialize, Clone)]
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

