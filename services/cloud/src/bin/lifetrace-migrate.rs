use lifetrace_cloud::Config;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    config
        .validate()
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;

    let database_url = config
        .database_url
        .as_deref()
        .ok_or("DATABASE_URL is required for migrations")?;

    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(config.database_max_connections.max(1))
        .connect(database_url)
        .await?;

    let (database_name, database_user): (String, String) =
        sqlx::query_as("SELECT current_database(), current_user")
            .fetch_one(&pool)
            .await?;
    println!(
        "Running LifeTrace migrations on database '{database_name}' as user '{database_user}'"
    );

    sqlx::migrate!().run(&pool).await?;

    let cloud_users_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public.cloud_users') IS NOT NULL")
            .fetch_one(&pool)
            .await?;

    if !cloud_users_exists {
        return Err("migration verification failed: public.cloud_users was not created".into());
    }

    println!("LifeTrace PostgreSQL migrations completed and verified successfully");
    Ok(())
}
