use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct GameConfig {
    pub name: String,
    pub description: String,
    #[serde(rename = "startingRoom")]
    pub starting_room: String,
    pub published: bool,
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
    pub id: Uuid,
    #[serde(rename = "ownerId")]
    pub owner_id: Uuid,
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
