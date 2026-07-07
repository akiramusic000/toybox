use std::io::Write;

use anyhow::Result;
use clap::Args;
use keyring::Entry;
use rpassword::read_password;
use toybox_rs::Client;

use crate::Config;

#[derive(Args)]
pub struct Login {
    username: String,
}

pub async fn run_login(config: &mut Config, client: &mut Client, login: Login) -> Result<()> {
    let Login { username } = login;

    if config.username.as_ref() == Some(&username) && client.auth.is_some() {
        eprintln!("Already logged in as {username}!");
        return Ok(());
    }

    print!("Password: ");
    std::io::stdout().flush()?;
    let password = read_password()?;

    let entry = Entry::new("toybox", &username)?;

    let auth = client.authenticate(username.clone(), password).await?;

    eprintln!("Successfully logged in as {username}!");

    entry.set_password(&auth.token)?;

    config.user_id = Some(auth.id);
    config.username = Some(username);

    Ok(())
}
