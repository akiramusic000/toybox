use anyhow::Result;
use keyring::Entry;
use toybox_rs::Client;

use crate::Config;

pub async fn run_logout(config: &mut Config, _: &mut Client) -> Result<()> {
    let Some(username) = &config.username else {
        println!("Already logged out!");
        return Ok(());
    };

    let entry = Entry::new("toybox", username)?;
    entry.delete_credential()?;

    eprintln!("Successfully logged out {username}!");

    config.user_id = None;
    config.username = None;

    Ok(())
}
