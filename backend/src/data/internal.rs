use rand::prelude::*;

pub struct ServerState {
    pub server_secret: [u8; 32],
}

impl ServerState {
    pub fn new() -> Self {
        let mut secret = [0; 32];
        thread_rng().fill_bytes(&mut secret);
        Self {
            server_secret: secret,
        }
    }
}
