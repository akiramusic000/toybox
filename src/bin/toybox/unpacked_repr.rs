use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use toybox_rs::{Client, Game, Object, ObjectInstance, Room, Sprite};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct GameConfig {
    pub name: String,
    pub description: String,
    #[serde(rename = "startingRoom")]
    pub starting_room: String,
    pub published: bool,
}

impl GameConfig {
    pub fn load_from_game_path<P: AsRef<Path>>(game_path: P) -> Result<Self> {
        let game_path = game_path.as_ref();
        let config_path = game_path.join("config.json");
        let config_json = fs::read_to_string(&config_path)?;
        let config = serde_json::from_str::<GameConfig>(&config_json)?;
        Ok(config)
    }

    pub fn save_to_game_path<P: AsRef<Path>>(&self, game_path: P) -> Result<()> {
        let game_path = game_path.as_ref();
        let config_path = game_path.join("config.json");
        let config_json = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, config_json)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectConfig {
    pub sprite: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomConfig {
    #[serde(rename = "backgroundSprite")]
    pub background_sprite: Option<String>,
    pub objects: Vec<ObjectInstanceConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectInstanceConfig {
    pub object: String,
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToyboxConfig {
    pub id: Option<Uuid>,
    #[serde(rename = "ownerId")]
    pub owner_id: Option<Uuid>,
    pub objects: Vec<ObjectUnpacked>,
    pub rooms: Vec<RoomUnpacked>,
    pub sprites: Vec<SpriteUnpacked>,
}

impl ToyboxConfig {
    pub fn lookup_sprite<'a>(&'a self, name: &str) -> Result<&'a SpriteUnpacked> {
        self.sprites
            .iter()
            .find(|sprite| sprite.name.as_str() == name)
            .ok_or(anyhow!("Cannot find sprite {name} in sprite list!"))
    }

    pub fn lookup_sprite_mut<'a>(&'a mut self, name: &str) -> Result<&'a mut SpriteUnpacked> {
        self.sprites
            .iter_mut()
            .find(|sprite| sprite.name.as_str() == name)
            .ok_or(anyhow!("Cannot find sprite {name} in sprite list!"))
    }

    pub fn lookup_object<'a>(&'a self, name: &str) -> Result<&'a ObjectUnpacked> {
        self.objects
            .iter()
            .find(|obj| obj.name.as_str() == name)
            .ok_or(anyhow!("Cannot find object {name} in object list!"))
    }

    pub fn lookup_object_mut<'a>(&'a mut self, name: &str) -> Result<&'a mut ObjectUnpacked> {
        self.objects
            .iter_mut()
            .find(|obj| obj.name.as_str() == name)
            .ok_or(anyhow!("Cannot find object {name} in object list!"))
    }

    pub fn lookup_room<'a>(&'a self, name: &str) -> Result<&'a RoomUnpacked> {
        self.rooms
            .iter()
            .find(|room| room.name.as_str() == name)
            .ok_or(anyhow!("Cannot find room {name} in room list!"))
    }

    pub fn lookup_room_mut<'a>(&'a mut self, name: &str) -> Result<&'a mut RoomUnpacked> {
        self.rooms
            .iter_mut()
            .find(|room| room.name.as_str() == name)
            .ok_or(anyhow!("Cannot find room {name} in room list!"))
    }

    pub fn load_from_game_path<P: AsRef<Path>>(game_path: P) -> Result<Self> {
        let game_path = game_path.as_ref();
        let config_path = game_path.join(".toybox/toybox.json");
        let config_json = fs::read_to_string(&config_path)?;
        let config = serde_json::from_str::<ToyboxConfig>(&config_json)?;
        Ok(config)
    }

    pub fn save_to_game_path<P: AsRef<Path>>(&self, game_path: P) -> Result<()> {
        let game_path = game_path.as_ref();
        let config_path = game_path.join(".toybox/toybox.json");
        let config_json = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, config_json)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectUnpacked {
    pub name: String,
    pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomUnpacked {
    pub name: String,
    pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpriteUnpacked {
    pub name: String,
    pub id: Uuid,
    pub image_path: String,
    pub md5: String,
}

pub async fn pack<P: AsRef<Path>>(game_path: P, client: &mut Client) -> Result<Game> {
    let game_path = game_path.as_ref();

    let objects = game_path.join("objects");
    let rooms = game_path.join("rooms");
    let sprites = game_path.join("sprites");

    let mut toybox_config = ToyboxConfig::load_from_game_path(game_path)?;
    let game_config = GameConfig::load_from_game_path(game_path)?;

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

        let room_unpacked = if let Ok(room_unpacked) = toybox_config.lookup_room(&room_name) {
            room_unpacked
        } else {
            toybox_config.rooms.push(RoomUnpacked {
                name: room_name,
                id: Uuid::new_v4(),
            });
            toybox_config.rooms.last().unwrap()
        };

        let mut room = Room {
            id: room_unpacked.id,
            name: room_unpacked.name.clone(),
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

    toybox_config.save_to_game_path(game_path)?;

    Ok(game)
}

pub async fn unpack<P: AsRef<Path>>(game_path: P, game: &Game, client: &mut Client) -> Result<()> {
    let game_path = game_path.as_ref();

    let dot_toybox = game_path.join(".toybox");
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
        name: game.name.clone(),
        description: game.description.clone(),
        starting_room: game.rooms[starting_room_index].name.clone(),
        published: game.published,
    };

    config.save_to_game_path(game_path)?;

    let mut toybox_config = ToyboxConfig {
        id: game.id,
        owner_id: game.owner_id,
        objects: vec![],
        rooms: vec![],
        sprites: vec![],
    };

    for room in &game.rooms {
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
                .iter()
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
            name: room.name.clone(),
            id: room.id,
        });
    }

    for object in &game.objects {
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
        fs::write(object_script_path, object.script.clone())?;

        toybox_config.objects.push(ObjectUnpacked {
            name: object.name.clone(),
            id: object.id,
        });
    }

    for sprite in &game.sprites {
        let sprite_path = sprites.join(&sprite.name).with_extension("png");
        let sprite_data = client
            .fetch_file(Client::sprite_path_to_url(&sprite.image_path))
            .await?;
        fs::write(&sprite_path, &sprite_data)?;
        let md5 = format!("{:x}", md5::compute(sprite_data));

        toybox_config.sprites.push(SpriteUnpacked {
            name: sprite.name.clone(),
            id: sprite.id,
            image_path: sprite.image_path.clone(),
            md5,
        });
    }

    toybox_config.save_to_game_path(game_path)?;

    Ok(())
}

pub fn is_valid_game_path<P: AsRef<Path>>(game_path: P) -> bool {
    let game_path = game_path.as_ref();

    game_path.join(".toybox/toybox.json").exists() && game_path.join("config.json").exists()
}

pub fn find_valid_game_path<P: AsRef<Path>>(game_path: P) -> Result<Option<PathBuf>> {
    let mut game_path = game_path.as_ref();

    while !is_valid_game_path(game_path) {
        let Some(path) = game_path.parent() else {
            return Ok(None);
        };

        game_path = path
    }

    Ok(Some(game_path.to_path_buf()))
}
