use std::{ffi::OsStr, fs, path::PathBuf};

use anyhow::Result;
use clap::Args;
use toybox_rs::{Client, Game, Object, ObjectInstance, Room, Sprite};
use uuid::Uuid;

use crate::{
    Config,
    unpacked_repr::{
        GameConfig, ObjectConfig, ObjectUnpacked, RoomConfig, SpriteUnpacked, ToyboxConfig,
    },
};

#[derive(Args)]
pub struct Upload {
    game_path: PathBuf,
}

pub async fn run_upload(_: &mut Config, client: &mut Client, upload: Upload) -> Result<()> {
    let Upload { game_path } = upload;

    let dot_toybox = game_path.join(".toybox");
    let toybox_config_path = dot_toybox.join("toybox.json");
    let game_config_path = game_path.join("config.json");
    let objects = game_path.join("objects");
    let rooms = game_path.join("rooms");
    let sprites = game_path.join("sprites");

    let toybox_config_json = fs::read_to_string(&toybox_config_path)?;
    let mut toybox_config = serde_json::from_str::<ToyboxConfig>(&toybox_config_json)?;

    let game_config_json = fs::read_to_string(&game_config_path)?;
    let game_config = serde_json::from_str::<GameConfig>(&game_config_json)?;

    let mut game = Game {
        id: toybox_config.id,
        owner_id: toybox_config.owner_id,
        name: game_config.name.clone(),
        description: game_config.description.clone(),
        sprites: vec![],
        objects: vec![],
        rooms: vec![],
        starting_room_id: Uuid::nil(),
        published: game_config.published,
        plays: None,
        likes: None,
        liked: None,
    };

    for entry in fs::read_dir(&sprites)? {
        let entry = entry?;
        let path = entry.path();

        let mime_type = if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("png") => "image/png",
                Some("jpg" | "jpeg") => "image/jpeg",
                _ => {
                    eprintln!("Unsupported sprite extension {}! skipping.", ext.display());
                    continue;
                }
            }
        } else {
            continue;
        };

        let image_data = fs::read(&path)?;
        let digest = md5::compute(&image_data);
        let md5 = format!("{digest:x}");
        let sprite_name = path.file_stem().unwrap().to_string_lossy().to_string();

        let sprite = if let Ok(sprite) = toybox_config.lookup_sprite_mut(&sprite_name) {
            if sprite.md5 != md5 {
                sprite.image_path = client.upload_sprite(image_data, mime_type).await?;
            }

            sprite
        } else {
            let image_path = client.upload_sprite(image_data, mime_type).await?;

            &*toybox_config.sprites.push_mut(SpriteUnpacked {
                name: sprite_name,
                id: Uuid::new_v4(),
                image_path,
                md5,
            })
        };

        game.sprites.push(Sprite {
            id: sprite.id,
            name: sprite.name.clone(),
            image_path: sprite.image_path.clone(),
        });
    }

    for entry in fs::read_dir(&objects)? {
        let entry = entry?;
        let json_path = entry.path();
        let object_name = json_path.file_stem().unwrap().to_string_lossy().to_string();
        if json_path.extension() != Some(OsStr::new("json")) {
            continue;
        }
        let script_path = json_path.with_extension("lua");

        let object_json = fs::read_to_string(&json_path)?;
        let script = fs::read_to_string(&script_path)?;

        let object_config = serde_json::from_str::<ObjectConfig>(&object_json)?;

        let object = if let Ok(object) = toybox_config.lookup_object(&object_name) {
            object
        } else {
            toybox_config.objects.push(ObjectUnpacked {
                name: object_name,
                id: Uuid::new_v4(),
            });
            toybox_config.objects.last().unwrap()
        };

        let sprite_id = if let Some(sprite_name) = object_config.sprite {
            let sprite = toybox_config.lookup_sprite(&sprite_name)?;
            Some(sprite.id)
        } else {
            None
        };

        game.objects.push(Object {
            id: object.id,
            name: object.name.clone(),
            sprite_id,
            script,
        });
    }

    for entry in fs::read_dir(&rooms)? {
        let entry = entry?;
        let json_path = entry.path();
        let room_name = json_path.file_stem().unwrap().to_string_lossy().to_string();
        let room_json = fs::read_to_string(&json_path)?;
        let room_config = serde_json::from_str::<RoomConfig>(&room_json)?;

        let mut room = Room {
            id: toybox_config.lookup_room(&room_name)?.id,
            name: room_name,
            background_sprite_id: if let Some(background_sprite) = room_config.background_sprite {
                Some(toybox_config.lookup_sprite(&background_sprite)?.id)
            } else {
                None
            },
            background_color: None,
            objects: vec![],
        };

        for obj in room_config.objects {
            let object = toybox_config.lookup_object(&obj.object)?;
            let instance = ObjectInstance {
                id: Uuid::new_v4(),
                game_object_id: object.id,
                x: obj.x,
                y: obj.y,
            };

            room.objects.push(instance);
        }

        game.rooms.push(room);
    }

    game.starting_room_id = toybox_config.lookup_room(&game_config.starting_room)?.id;

    let toybox_config_json = serde_json::to_string_pretty(&toybox_config)?;
    fs::write(&toybox_config_path, &toybox_config_json)?;

    let game_config_json = serde_json::to_string_pretty(&game_config)?;
    fs::write(&game_config_path, &game_config_json)?;

    client.upload_game(&game).await?;

    Ok(())
}
