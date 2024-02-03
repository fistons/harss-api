//! Http model

pub use crate::common::model::UserRole;
use secrecy::Secret;
use serde::Deserialize;

/// Request to create a new user
#[derive(Debug, Deserialize, Clone)]
pub struct NewUserRequest {
    pub username: String,
    pub password: Secret<String>,
    pub email: Option<String>,
    pub role: UserRole,
}

/// Request to register to a new channel
#[derive(Debug, Deserialize, Clone)]
pub struct RegisterChannelRequest {
    pub url: String,
    pub name: Option<String>,
    pub notes: Option<String>,
}

/// Filter parameters on read/starred items' status
#[derive(Debug, Deserialize)]
pub struct ReadStarredParameters {
    pub read: Option<bool>,
    pub starred: Option<bool>,
}

/// Represent a list of IDs (could be item, channel, etc)
#[derive(Debug, Deserialize)]
pub struct IdListParameter {
    pub ids: Vec<i32>,
}

/// # Paging parameters
#[derive(Debug, Deserialize)]
pub struct PageParameters {
    /// * The `page` field should be superior or equals to 1
    page: Option<u64>,
    /// * The `size` field should be between 1 and 20
    size: Option<u64>,
}

impl PageParameters {
    /// Return the given page, or 1 if not provided.
    /// If the given page is 0, return 1.
    pub fn get_page(&self) -> u64 {
        self.page.unwrap_or(1).max(1)
    }

    /// Return the given size of the page, or 20 if not provided.
    /// The result is clamped between 1 and 200.
    pub fn get_size(&self) -> u64 {
        self.size.unwrap_or(20u64).clamp(1, 200)
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub current_password: Secret<String>,
    pub new_password: Secret<String>,
    pub confirm_password: Secret<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOtherPasswordRequest {
    pub new_password: Secret<String>,
    pub confirm_password: Secret<String>,
}

#[derive(Debug, Deserialize)]
pub struct ItemNotesRequest {
    pub notes: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub email: Secret<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordTokenRequest {
    pub token: Secret<String>,
    pub email: Secret<String>, // Needed?
    pub new_password: Secret<String>,
}
