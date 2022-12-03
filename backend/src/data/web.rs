use super::internal::ServerState;
use common::crypto::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SessionCookie {
    pub nonce: [u8; 32],
    pub user_id: Option<i32>,
}

impl SessionCookie {
    pub fn new() -> Self {
        Self {
            nonce: Default::default(),
            user_id: None,
        }
    }

    pub fn serialize_with_hmac(&self, state: &ServerState) -> String {
        let json_string = serde_json::to_string(&self).expect("could not serialize cookie");
        let mut base64_string = base64::encode_config(json_string, base64::URL_SAFE);
        let hmac = hmac(&state.server_secret, &base64_string);
        let hmac_string = base64::encode_config(&hmac, base64::URL_SAFE);
        base64_string.push('.');
        base64_string.push_str(&hmac_string);
        base64_string
    }

    pub fn serialize_as_set_cookie(&self, state: &ServerState) -> String {
        let mut out = "session=\"".to_string();
        out.push_str(&self.serialize_with_hmac(state));
        out.push_str("\"");
        out
    }

    pub fn deserialize_with_hmac(data: &str, state: &ServerState) -> Option<Self> {
        let dot_index = data.find('.')?;
        log::debug!("Dot is at index {}", dot_index);
        let (text, user_hmac) = data.split_at(dot_index);
        let user_hmac = user_hmac.replace(".", "");
        log::debug!("Parts are {:?} and {:?}", text, user_hmac);
        let user_hmac_bytes = base64::decode_config(&user_hmac, base64::URL_SAFE).ok()?;
        log::debug!("Cookie's HMAC is: {:x?}", user_hmac_bytes);
        let expected_hmac = hmac(&state.server_secret, &text);
        log::debug!("Expected HMAC is: {:x?}", expected_hmac);
        if !compare_digest(&user_hmac_bytes, &expected_hmac) {
            log::warn!("User's cookie has wrong HMAC");
            return None;
        }

        let json_bytes = base64::decode_config(&text, base64::URL_SAFE).ok()?;
        log::debug!("Cookie's HMAC is: {:x?}", user_hmac_bytes);
        let cookie = serde_json::from_slice(&json_bytes).ok()?;

        Some(cookie)
    }

    pub fn deserialize_as_cookie(data: &str, state: &ServerState) -> Option<Self> {
        for cookie in data.split(";") {
            log::debug!("Found cookie: {:?}", &cookie);
            let cookie = cookie.trim().to_string();
            if cookie.starts_with("session=") {
                let cookie = cookie.strip_prefix("session=")?.to_string();
                let cookie = cookie.replace("\"", "");
                log::debug!("This is the session cookie: {:?}", cookie);
                return Self::deserialize_with_hmac(&cookie, state);
            }
        }
        None
    }
}
