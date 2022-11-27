use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn hmac(key: &str, val: &str) -> Vec<u8> {
    let mut hmac_generator = HmacSha256::new_from_slice(key.as_bytes()).unwrap();
    hmac_generator.update(val.as_bytes());
    let output = hmac_generator.finalize().into_bytes();
    output[..].to_vec()
}
