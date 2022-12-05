#![cfg(feature = "diesel_sqlite")]

use super::UserRole;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::*;
use diesel::sqlite::Sqlite;
use enum_primitive::*;

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
