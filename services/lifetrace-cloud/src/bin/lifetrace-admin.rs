use std::env;
use std::io::{self, Write};

use lifetrace_cloud::{AppState, Config};

fn usage() -> &'static str {
    "Usage:\n  lifetrace-admin bootstrap-user --email <email> [--display-name <name>] [--allow-additional]\n  lifetrace-admin create-invite [--email <email>] [--expires-seconds <seconds>]\n\nPasswords are read from the terminal and never accepted as command-line arguments."
}

fn value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        eprintln!("{}", usage());
        std::process::exit(2);
    };

    let config = Config::from_env();
    config
        .validate()
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let state = AppState::new(config);
    state.initialize().await?;

    match command {
        "bootstrap-user" => {
            let email = value(&args, "--email").ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "--email is required")
            })?;
            let display_name = value(&args, "--display-name");
            print!("Password: ");
            io::stdout().flush()?;
            let password = rpassword::read_password()?;
            print!("Confirm password: ");
            io::stdout().flush()?;
            let confirmation = rpassword::read_password()?;
            if password != confirmation {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "password confirmation does not match",
                )
                .into());
            }
            let user_id = state
                .auth_service
                .bootstrap_user(
                    &email,
                    display_name.as_deref(),
                    &password,
                    args.iter().any(|arg| arg == "--allow-additional"),
                )
                .await?;
            println!("Created LifeTrace account {user_id}");
        }
        "create-invite" => {
            let expires = value(&args, "--expires-seconds")
                .map(|value| value.parse::<u64>())
                .transpose()?
                .unwrap_or(86_400);
            let token = state
                .auth_service
                .create_invite(value(&args, "--email").as_deref(), expires, None)
                .await?;
            println!("{token}");
        }
        _ => {
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    }
    Ok(())
}
