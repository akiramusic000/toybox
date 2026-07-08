use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Args;
use toybox_rs::Client;
use uuid::Uuid;

use crate::{Config, unpacked_repr::unpack};

#[derive(Args)]
pub struct Download {
    game: String,
    game_path: PathBuf,
}

pub async fn run_download(_: &mut Config, client: &mut Client, download: Download) -> Result<()> {
    let Download { game, game_path } = download;
    let url = game.strip_prefix("https://").unwrap_or(&game);
    let page = url.strip_prefix("toybox.zublek.net/").unwrap_or(url);
    let uuid_str = &page
        .strip_prefix("editor/")
        .unwrap_or(page)
        .strip_prefix("play/")
        .unwrap_or(page)[0..36];
    let Ok(uuid) = Uuid::parse_str(uuid_str) else {
        bail!("Cannot parse game '{game}'!");
    };

    let game = client.fetch_game(uuid).await?;

    unpack(game_path, &game, client).await?;

    Ok(())
}
