use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub type HmacOutput = [u8; 32];

pub fn hmac(key: &[u8], val: &str) -> HmacOutput {
    log::trace!("Key: {:x?}", key);
    log::trace!("Val: {:x?}", val.as_bytes());
    let mut hmac_generator = HmacSha256::new_from_slice(key).unwrap();
    hmac_generator.update(val.as_bytes());
    let hmac = hmac_generator.finalize().into_bytes();
    log::info!("Out: {:x?}", &hmac);
    hmac.try_into()
        .expect("Hmac<Sha256>'s output should be [u8; 32]")
}

/// Returns `a==b`, comparing the two with an algorithm to avoid timing attacks.
///
/// Returns `false` early if `a` and `b` have different lengths.
/// This will reveal to the attacker that the lengths are different, but not what they are.
pub fn compare_digest<T>(a: &[T], b: &[T]) -> bool
where
    T: PartialEq,
{
    let mut ok = true;
    if a.len() != b.len() {
        return false;
    }
    for (x, y) in a.iter().zip(b.iter()) {
        ok = ok && (x == y);
    }

    ok
}
