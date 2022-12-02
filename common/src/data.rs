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
        // serde_wasm_bindgen::from_value(val).unwrap_or_default()
        serde_json::from_str(&val.as_string().unwrap_or_default()).unwrap_or_default()
    }
}
