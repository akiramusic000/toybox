use std::{fs, path::PathBuf};

use anyhow::{Result, bail};
use clap::Args;
use toybox_rs::Client;
use uuid::Uuid;

use crate::{
    Config,
    unpacked_repr::{
        GameConfig, ObjectConfig, ObjectInstanceConfig, ObjectUnpacked, RoomConfig, RoomUnpacked,
        SpriteUnpacked, ToyboxConfig,
    },
};

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

    let dot_toybox = game_path.join(".toybox");
    let toybox_config_path = dot_toybox.join("toybox.json");
    let game_config = game_path.join("config.json");
    let objects = game_path.join("objects");
    let rooms = game_path.join("rooms");
    let sprites = game_path.join("sprites");

    fs::create_dir_all(&dot_toybox)?;
    fs::create_dir_all(&objects)?;
    fs::create_dir_all(&rooms)?;
    fs::create_dir_all(&sprites)?;

    let starting_room_index = game
        .rooms
        .binary_search_by_key(&game.starting_room_id, |room| room.id)
        .expect("Starting room ID not found in rooms!");

    let config = GameConfig {
        name: game.name,
        description: game.description,
        starting_room: game.rooms[starting_room_index].name.clone(),
        published: game.published,
    };

    let config_json = serde_json::to_string_pretty(&config)?;
    fs::write(game_config, config_json)?;

    let mut toybox_config = ToyboxConfig {
        id: game.id,
        owner_id: game.owner_id,
        objects: vec![],
        rooms: vec![],
        sprites: vec![],
    };

    for room in game.rooms {
        let room_config_path = rooms.join(&room.name).with_extension("json");

        let background_sprite = if let Some(background_sprite_id) = room.background_sprite_id {
            let background_sprite = game
                .sprites
                .binary_search_by_key(&background_sprite_id, |sprite| sprite.id)
                .unwrap_or_else(|_| {
                    panic!("Failed to find sprite ID {background_sprite_id} in sprite list!")
                });
            Some(game.sprites[background_sprite].name.clone())
        } else {
            None
        };

        let room_config = RoomConfig {
            background_sprite,
            objects: room
                .objects
                .into_iter()
                .map(|object_instance| {
                    let object = game
                        .objects
                        .binary_search_by_key(&object_instance.game_object_id, |object| object.id)
                        .unwrap_or_else(|_| {
                            panic!(
                                "Failed to find object ID {} in object list!",
                                object_instance.game_object_id
                            )
                        });
                    ObjectInstanceConfig {
                        object: game.objects[object].name.clone(),
                        x: object_instance.x,
                        y: object_instance.y,
                    }
                })
                .collect(),
        };
        let room_config_json = serde_json::to_string_pretty(&room_config)?;
        fs::write(room_config_path, room_config_json)?;

        toybox_config.rooms.push(RoomUnpacked {
            name: room.name,
            id: room.id,
        });
    }

    for object in game.objects {
        let object_config_path = objects.join(&object.name).with_extension("json");
        let object_script_path = objects.join(&object.name).with_extension("lua");

        let sprite = if let Some(sprite_id) = object.sprite_id {
            let sprite = game
                .sprites
                .binary_search_by_key(&sprite_id, |sprite| sprite.id)
                .unwrap_or_else(|_| {
                    panic!("Failed to find sprite ID {} in sprite list!", sprite_id)
                });
            Some(game.sprites[sprite].name.clone())
        } else {
            None
        };
        let object_config = ObjectConfig { sprite };
        let object_config_json = serde_json::to_string_pretty(&object_config)?;

        fs::write(object_config_path, object_config_json)?;
        fs::write(object_script_path, object.script)?;

        toybox_config.objects.push(ObjectUnpacked {
            name: object.name,
            id: object.id,
        });
    }

    for sprite in game.sprites {
        let sprite_path = sprites.join(&sprite.name).with_extension("png");
        let sprite_data = client.fetch_file(&sprite.image_path).await?;
        fs::write(&sprite_path, &sprite_data)?;
        let md5 = format!("{:x}", md5::compute(sprite_data));

        toybox_config.sprites.push(SpriteUnpacked {
            name: sprite.name,
            id: sprite.id,
            image_path: sprite.image_path,
            md5,
        });
    }

    let toybox_config_json = serde_json::to_string_pretty(&toybox_config)?;
    fs::write(toybox_config_path, toybox_config_json)?;

    Ok(())
}
