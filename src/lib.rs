use anyhow::{Result, bail};
use reqwest::{Client as HttpClient, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use serde_with::{NoneAsEmptyString, serde_as};
use uuid::Uuid;

macro_rules! api_endpoint {
    ($($arg:tt)*) => {
        format!("https://api.toybox.zublek.net/api/v1{}", format!($($arg)*))
    };
}

trait RequestEx {
    fn load_auth(self, client: &Client) -> Self;
}

impl RequestEx for RequestBuilder {
    fn load_auth(self, client: &Client) -> Self {
        if let Some(auth) = client.auth.as_ref() {
            self.header("Cookie", &format!("session_token={}", auth.token))
        } else {
            self
        }
    }
}

pub trait AuthStatus {}

pub struct Unauthenticated;
impl AuthStatus for Unauthenticated {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Authenticated {
    pub token: String,
    pub id: Uuid,
}
impl AuthStatus for Authenticated {}

pub struct Client {
    pub auth: Option<Authenticated>,
    client: HttpClient,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Client {
        Client {
            auth: None,
            client: HttpClient::new(),
        }
    }

    pub fn new_authenticated(auth: Authenticated) -> Client {
        Client {
            auth: Some(auth),
            client: HttpClient::new(),
        }
    }

    pub async fn handle_error(&self, response: Response) -> Result<Response> {
        #[derive(Serialize, Deserialize)]
        struct Error {
            error: String,
        }

        if !response.status().is_success() {
            bail!("{}", response.json::<Error>().await?.error);
        }

        Ok(response)
    }

    pub async fn authenticate(
        &mut self,
        username: String,
        password: String,
    ) -> Result<&Authenticated> {
        #[derive(Serialize, Deserialize)]
        pub struct AuthRequest {
            username: String,
            password: String,
        }

        #[derive(Serialize, Deserialize)]
        pub struct AuthResponse {
            token: String,
            user_id: Uuid,
            expires_at: String,
        }

        let response = self
            .client
            .post(api_endpoint!("/auth/login"))
            .load_auth(self)
            .json(&AuthRequest {
                username: username.clone(),
                password,
            })
            .send()
            .await?;

        let response = self.handle_error(response).await?;

        let response = response.json::<AuthResponse>().await?;

        self.load_session(Authenticated {
            token: response.token,
            id: response.user_id,
        });

        Ok(self.auth.as_ref().unwrap())
    }

    pub fn load_session(&mut self, auth: Authenticated) {
        self.auth = Some(auth);
    }

    pub async fn is_logged_in(&mut self) -> Result<bool> {
        let response = self
            .client
            .get(api_endpoint!("/auth/me"))
            .load_auth(self)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    pub async fn fetch_game(&mut self, game_id: Uuid) -> Result<Game> {
        let response = self
            .client
            .get(api_endpoint!("/games/{game_id}"))
            .load_auth(self)
            .send()
            .await?;

        let response = self.handle_error(response).await?;

        let mut game = response.json::<Game>().await?;

        game.rooms.sort_by_key(|room| room.id);
        game.sprites.sort_by_key(|sprite| sprite.id);
        game.objects.sort_by_key(|object| object.id);

        Ok(game)
    }

    pub async fn upload_game(&mut self, game: &Game) -> Result<Game> {
        let request = if let Some(id) = game.id {
            self.client.put(api_endpoint!("/games/{id}"))
        } else {
            self.client.post(api_endpoint!("/games"))
        };

        let response = request.load_auth(self).json(game).send().await?;

        let response = self.handle_error(response).await?;
        let game = response.json::<Game>().await?;

        Ok(game)
    }

    pub async fn fetch_file(&mut self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            bail!("Cannot find file at URL {url}!");
        }
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    pub fn sprite_path_to_url(url: &str) -> &str {
        match url {
            "hero.png" => "https://toybox.zublek.com/assets/hero-CLDdwZDr.png",
            "test/ball.png" => {
                "https://toybox.zublek.net/sprites/259a6585-80c8-41a8-a557-d3d1b92c9632.png"
            }
            "test/bat.png" => {
                "https://toybox.zublek.net/sprites/66d6e464-d5ce-4d95-a40d-9b487d4091ac.png"
            }
            "test/bg.png" => {
                "https://assets.toybox.zublek.net/sprites/8317f030-c83a-4fc6-85a8-2e67d474e87d.png"
            }
            "test/bg2.png" => "https://toybox.zublek.com/assets/bg2-DJrhAMrp.png",
            "test/bg3.png" => "https://toybox.zublek.com/assets/bg3-z8e21UsU.png",
            "test/default.png" => {
                "https://assets.toybox.zublek.net/sprites/01660104-e53a-4ae5-84b2-d3af861a2799.png"
            }
            _ => url,
        }
    }

    pub async fn upload_sprite(&mut self, image_data: Vec<u8>, mime_type: &str) -> Result<String> {
        #[derive(Serialize, Deserialize)]
        struct UploadSpriteResponse {
            image_path: String,
        }

        let response = self
            .client
            .post(api_endpoint!("/assets/sprites"))
            .body(image_data)
            .header("Content-Type", mime_type)
            .send()
            .await?;
        let response = self.handle_error(response).await?;

        let response = response.json::<UploadSpriteResponse>().await?;
        Ok(response.image_path)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Game {
    pub id: Option<Uuid>,
    #[serde(rename = "ownerId")]
    pub owner_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub sprites: Vec<Sprite>,
    pub objects: Vec<Object>,
    pub rooms: Vec<Room>,
    #[serde(rename = "startingRoomId")]
    pub starting_room_id: Uuid,
    pub published: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plays: Option<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub likes: Option<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liked: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sprite {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "imagePath")]
    pub image_path: String,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Object {
    pub id: Uuid,
    pub name: String,
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(rename = "spriteId")]
    pub sprite_id: Option<Uuid>,
    pub script: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "backgroundSpriteId")]
    pub background_sprite_id: Option<Uuid>,
    #[serde(rename = "backgroundColor")]
    pub background_color: Option<String>, // Currently unused?
    pub objects: Vec<ObjectInstance>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectInstance {
    pub id: Uuid,
    #[serde(rename = "gameObjectId")]
    pub game_object_id: Uuid,
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    username: String,
    user_id: Uuid,
}
