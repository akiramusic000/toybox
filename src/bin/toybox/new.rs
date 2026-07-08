use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use toybox_rs::{Client, Game, Object, ObjectInstance, Room, Sprite};
use uuid::Uuid;

use crate::{Config, unpacked_repr::unpack};

const DEFAULT_SCRIPT: &str = include_str!("default_script.lua");

#[derive(Args)]
pub struct New {
    path: PathBuf,
}

pub async fn run_new(config: &mut Config, client: &mut Client, new: New) -> Result<()> {
    let New { path } = new;
    let sprite_id = Uuid::new_v4();
    let obj_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let object_instance_id = Uuid::new_v4();
    unpack(
        path,
        &Game {
            id: None,
            owner_id: config.user_id,
            name: String::from("New Game"),
            description: String::from("This game currently does not have a description."),
            sprites: vec![Sprite {
                id: sprite_id,
                name: "Sprite1".to_string(),
                image_path: "test/default.png".to_string(),
            }],
            objects: vec![Object {
                id: obj_id,
                name: "Object1".to_string(),
                sprite_id: Some(sprite_id),
                script: DEFAULT_SCRIPT.to_string(),
            }],
            rooms: vec![Room {
                id: room_id,
                name: "Room 1".to_string(),
                background_sprite_id: None,
                background_color: None,
                objects: vec![ObjectInstance {
                    id: object_instance_id,
                    game_object_id: obj_id,
                    x: 0,
                    y: 0,
                }],
            }],
            starting_room_id: room_id,
            published: false,
            plays: None,
            likes: None,
            liked: None,
        },
        client,
    )
    .await?;

    Ok(())
}
