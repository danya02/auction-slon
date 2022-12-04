use diesel::prelude::*;
use enum_primitive::*;

pub use common::data::UserRole;

#[derive(Debug, PartialEq, Queryable)]
pub struct User {
    pub id: Option<i32>,
    pub name: String,
    pub passcode: String,
    pub role: UserRole,
}
