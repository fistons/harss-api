use serde::{Deserialize, Serialize};

/// # Application configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ApplicationConfiguration {
    /// Allow user creation without authentication
    pub allow_account_creation: bool,
}
