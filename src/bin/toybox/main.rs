use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use directories::ProjectDirs;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use toybox_rs::Authenticated;
use toybox_rs::Client;
use uuid::Uuid;

use crate::download::Download;
use crate::download::run_download;
use crate::fork::Fork;
use crate::fork::run_fork;
use crate::login::Login;
use crate::login::run_login;
use crate::logout::run_logout;
use crate::new::New;
use crate::new::run_new;
use crate::upload::run_upload;

mod download;
mod fork;
mod login;
mod logout;
mod new;
mod unpacked_repr;
mod upload;

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    subcommand: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Log in to ToyBox using a username
    Login(Login),
    /// Log out of ToyBox
    Logout,
    /// Download a game from ToyBox
    Download(Download),
    /// Upload a game to ToyBox
    Upload,
    /// Fork a game on ToyBox by reuploading it.
    Fork(Fork),
    /// Create a new game without uploading it.
    New(New),
}

#[derive(Default, Serialize, Deserialize)]
struct Config {
    username: Option<String>,
    user_id: Option<Uuid>,
}

static CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let dirs = ProjectDirs::from("com", "chloe", "toybox").unwrap();
    let config_dir = dirs.config_dir().to_path_buf();
    fs::create_dir_all(&config_dir).expect("Failed to create config dir!");
    config_dir
});

fn serialize_config(config: &Config) -> Result<()> {
    let config_path = CONFIG_DIR.join("config.json");

    Ok(fs::write(
        config_path,
        serde_json::to_string_pretty(config)?,
    )?)
}
fn deserialize_config() -> Result<Config> {
    let config_path = CONFIG_DIR.join("config.json");

    if !config_path.exists() {
        let config = Config::default();
        serialize_config(&config)?;
        return Ok(config);
    }

    let json = fs::read_to_string(config_path)?;
    Ok(serde_json::from_str::<Config>(&json)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut client = Client::new();

    let mut config = deserialize_config()?;

    if let (Some(user_id), Some(username)) = (&config.user_id, &config.username) {
        let entry = Entry::new("toybox", username)?;
        if let Ok(token) = entry.get_password() {
            client.load_session(Authenticated {
                token,
                id: *user_id,
            });
            if !client.is_logged_in().await? {
                client.auth = None;
                config.user_id = None;
                config.username = None;
                eprintln!("Session expired!");
            }
        }
    }

    match cli.subcommand {
        Commands::Login(login) => run_login(&mut config, &mut client, login).await?,
        Commands::Logout => run_logout(&mut config, &mut client).await?,
        Commands::Download(unpack) => run_download(&mut config, &mut client, unpack).await?,
        Commands::Upload => run_upload(&mut config, &mut client).await?,
        Commands::Fork(fork) => run_fork(&mut config, &mut client, fork).await?,
        Commands::New(new) => run_new(&mut config, &mut client, new).await?,
    }

    serialize_config(&config)?;

    Ok(())
}
