use std::fs;

use anyhow::{Result, bail};
use toybox_rs::Client;

use crate::{
    Config,
    unpacked_repr::{ToyboxConfig, find_valid_game_path, pack},
};

pub async fn run_upload(_: &mut Config, client: &mut Client) -> Result<()> {
    let Some(game_path) = find_valid_game_path(".")? else {
        bail!("Cannot find valid game in current directory!");
    };

    let game = pack(&game_path, client).await?;
    let game = client.upload_game(&game).await?;

    let dot_toybox = game_path.join(".toybox");
    let toybox_config_path = dot_toybox.join("toybox.json");
    let toybox_config_json = fs::read_to_string(&toybox_config_path)?;
    let mut toybox_config = serde_json::from_str::<ToyboxConfig>(&toybox_config_json)?;

    toybox_config.id = game.id;
    toybox_config.owner_id = game.owner_id;

    let toybox_config_json = serde_json::to_string_pretty(&toybox_config)?;
    fs::write(&toybox_config_path, &toybox_config_json)?;

    Ok(())
}
