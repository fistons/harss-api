use std::sync::Arc;

use diesel::prelude::*;

use crate::errors::ApiError;
use crate::model::{Category, NewCategory};
use crate::schema::categories::dsl::*;
use crate::DbPool;

#[derive(Clone)]
pub struct CategoryService {
    pool: Arc<DbPool>,
}

impl CategoryService {
    pub fn new(pool: DbPool) -> CategoryService {
        CategoryService {
            pool: Arc::new(pool),
        }
    }

    pub fn create_category(&self, new_cat: NewCategory, u_id: i32) -> Result<Category, ApiError> {
        let connection = self.pool.get().unwrap();
        let new_cat = NewCategory {
            user_id: u_id,
            ..new_cat
        };

        let cat_id: i32 = diesel::insert_into(categories)
            .values(new_cat)
            .returning(id)
            .get_result(&connection)?;

        Ok(categories
            .filter(id.eq(cat_id))
            .first::<Category>(&connection)?)
    }

    pub fn list_categories_of_user(&self, u_id: i32) -> Result<Vec<Category>, ApiError> {
        Ok(categories
            .filter(user_id.eq(u_id))
            .load(&self.pool.get().unwrap())?)
    }

    pub fn delete_category(&self, cat_id: i32, u_id: i32) -> Result<usize, ApiError> {
        Ok(
            diesel::delete(categories.filter(id.eq(cat_id).and(user_id.eq(u_id))))
                .execute(&self.pool.get().unwrap())?,
        )
    }
}
