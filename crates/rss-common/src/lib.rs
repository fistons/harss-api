extern crate core;

pub mod model;
pub mod observability;
pub mod services;

pub use entity::sea_orm_active_enums::UserRole;
pub use entity::users::Model as UserModel;
