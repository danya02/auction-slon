use enum_primitive::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::crypto;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginData {
    pub passcode_hmac: crypto::HmacOutput,
}

impl Default for LoginData {
    fn default() -> Self {
        Self {
            passcode_hmac: [0; 32],
        }
    }
}

impl Into<JsValue> for LoginData {
    fn into(self) -> wasm_bindgen::JsValue {
        JsValue::from_str(&serde_json::to_string(&self).unwrap_or_default())
    }
}

impl From<JsValue> for LoginData {
    fn from(val: wasm_bindgen::JsValue) -> Self {
        serde_json::from_str(&val.as_string().unwrap_or_default()).unwrap_or_default()
    }
}

#[cfg(feature = "sqlite")]
use diesel::AsExpression;
#[cfg(feature = "sqlite")]
use diesel::{sql_types::*, FromSqlRow};

enum_from_primitive! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[cfg_attr(
        feature = "sqlite",
        derive(AsExpression, FromSqlRow),
        repr(i32),
        diesel(sql_type = Integer)
    )]
    pub enum UserRole {
        Buyer = 0,
        Seller = 1,
    }
}

#[cfg(feature = "sqlite")]
pub mod diesel_sqlite {
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
    #[cfg(feature = "sqlite")]
    pub const HELLO: &str =
        "You can read this only if the feature diesel_sqlite is enabled for this crate";
}
