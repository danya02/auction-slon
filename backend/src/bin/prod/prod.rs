use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let mut dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dotenv_path.push("src/bin/prod/.env");
    dotenvy::from_path(dotenv_path.clone()).expect(&format!(
        "Could not find .env file at {}",
        dotenv_path.to_string_lossy()
    ));
    pretty_env_logger::init();
    backend::run([127, 0, 0, 1], 3030).await;
}
