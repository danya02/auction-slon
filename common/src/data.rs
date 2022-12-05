use enum_primitive::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::crypto;
pub mod diesel_sqlite;

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

#[cfg(feature = "diesel_sqlite")]
use diesel::AsExpression;
#[cfg(feature = "diesel_sqlite")]
use diesel::{sql_types::*, FromSqlRow};

enum_from_primitive! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[cfg_attr(
        feature = "diesel_sqlite",
        derive(AsExpression, FromSqlRow),
        repr(i32),
        diesel(sql_type = Integer)
    )]
    pub enum UserRole {
        Buyer = 0,
        Seller = 1,
    }
}
