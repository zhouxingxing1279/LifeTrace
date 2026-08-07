use lifetrace_cloud::{AppState, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    config
        .validate()
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    if config.database_url.is_none() {
        return Err("DATABASE_URL is required for migrations".into());
    }
    let state = AppState::new(config);
    state.initialize().await?;
    println!("LifeTrace PostgreSQL migrations completed successfully");
    Ok(())
}
