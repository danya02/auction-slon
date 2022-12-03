use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::{prelude::*, AsExpression};
use diesel::{sql_types::*, FromSqlRow};

#[derive(Debug, Clone, Copy, PartialEq, AsExpression, FromSqlRow)]
#[repr(i32)]
#[diesel(sql_type = Integer)]
pub enum UserRole {
    Buyer = 0,
    Seller = 1,
}

impl UserRole {
    pub fn value(&self) -> i32 {
        match self {
            UserRole::Buyer => 0,
            UserRole::Seller => 1,
        }
    }
}

impl<DB> ToSql<Integer, DB> for UserRole
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match self {
            UserRole::Buyer => 0.to_sql(out),
            UserRole::Seller => 1.to_sql(out),
        }
    }
}

impl<DB> FromSql<Integer, DB> for UserRole
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: diesel::backend::RawValue<'_, DB>) -> deserialize::Result<Self> {
        // Note: this is kinda code duplication
        match i32::from_sql(bytes)? {
            0 => Ok(UserRole::Buyer),
            1 => Ok(UserRole::Seller),
            x => Err(format!("Unrecognized variant {} of enum UserRole", x).into()),
        }
    }
}

#[derive(Debug, PartialEq, Queryable)]
pub struct User {
    pub id: Option<i32>,
    pub name: String,
    pub passcode: String,
    pub role: UserRole,
}

use super::schema::users;
#[derive(Debug, Insertable)]
#[diesel(table_name = users)]
/// Used for diesel's insertions
pub struct UserInsert {
    pub name: String,
    pub passcode: String,
    pub role: UserRole,
}
