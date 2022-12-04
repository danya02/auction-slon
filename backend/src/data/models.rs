use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sqlite::Sqlite;
use diesel::{prelude::*, AsExpression};
use diesel::{sql_types::*, FromSqlRow};
use enum_primitive::*;

enum_from_primitive! {
    #[derive(Debug, Clone, Copy, PartialEq, AsExpression, FromSqlRow)]
    #[repr(i32)]
    #[diesel(sql_type = Integer)]
    pub enum UserRole {
        Buyer = 0,
        Seller = 1,
    }
}

impl ToSql<Integer, Sqlite> for UserRole
where
    i32: ToSql<Integer, Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}

impl FromSql<Integer, Sqlite> for UserRole
where
    i32: FromSql<Integer, Sqlite>,
{
    fn from_sql(bytes: diesel::backend::RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        let val = i32::from_sql(bytes)?;
        UserRole::from_i32(val)
            .ok_or(format!("Unrecognized variant {} of enum UserRole", val).into())
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
