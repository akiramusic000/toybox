use anyhow::Result;
use clap::Args;
use toybox_rs::Client;
use uuid::Uuid;

use crate::Config;

#[derive(Args)]
pub struct Fork {
    game: Uuid,
}

pub async fn run_fork(_: &mut Config, client: &mut Client, fork: Fork) -> Result<()> {
    let mut game = client.fetch_game(fork.game).await?;
    game.id = None;
    game.owner_id = None;

    client.upload_game(&game).await?;

    Ok(())
}
