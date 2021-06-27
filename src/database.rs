use anyhow::Result;
use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{bson::doc, Client, Collection, Database};
use mongodb::options::FindOptions;
use uuid::Uuid;
use xtra::{Actor, Context, Handler, Message};

use crate::config::Config;
use crate::model::{PlayerGameStats, PlayerProfile, GameStatsBundle, PlayerStatsResponse};
use crate::util::uuid_to_bson;
use std::collections::HashMap;

pub struct MongoDatabaseHandler {
    client: Client,
    config: Config,
}

impl MongoDatabaseHandler {
    pub async fn connect(config: &Config) -> Result<Self> {
        let handler = Self {
            client: Client::with_uri_str(&*config.database_url).await?,
            config: config.clone(),
        };

        // Ping the database to ensure we can connect.
        handler.client.database("admin")
            .run_command(doc! {"ping": 1}, None)
            .await?;

        Ok(handler)
    }

    fn database(&self) -> Database {
        self.client.database(&*self.config.database_name)
    }

    fn player_profiles(&self) -> Collection<PlayerProfile> {
        self.database().collection::<PlayerProfile>("players")
    }

    fn player_stats(&self) -> Collection<PlayerGameStats> {
        self.database().collection::<PlayerGameStats>("player-stats")
    }

    async fn get_player_profile(&self, uuid: &Uuid) -> Result<Option<PlayerProfile>> {
        let options = FindOptions::builder().limit(1).build();
        let profile = self.player_profiles()
            .find(doc! {"uuid": uuid_to_bson(uuid)?}, options).await?
            .try_next().await?;
        Ok(profile)
    }

    async fn update_player_profile(&self, uuid: &Uuid, username: Option<String>) -> Result<PlayerProfile> {
        match self.get_player_profile(uuid).await? {
            Some(profile) => {
                if let Some(username) = username {
                    if let Some(profile_username) = profile.username.clone() {
                        if username != profile_username {
                            log::debug!("Player {} updated username to {}", uuid, &username);
                            self.player_profiles().update_one(
                                doc! {"uuid": uuid_to_bson(uuid)?},
                                doc! {"$set": {
                                    "username": username.clone(),
                                }},
                                None,
                            ).await?;

                            let mut profile = profile.clone();
                            profile.username = Some(username.clone());
                            return Ok(profile);
                        }
                    }
                }
                Ok(profile.clone())
            }
            None => {
                let profile = PlayerProfile {
                    uuid: uuid.clone(),
                    username: username.clone(),
                };
                self.player_profiles().insert_one(&profile, None).await?;
                Ok(profile)
            }
        }
    }

    async fn get_player_stats(&self, uuid: &Uuid, namespace: &Option<String>) -> Result<Option<PlayerStatsResponse>> {
        if let None = self.get_player_profile(uuid).await? { // player not found.
            return Ok(None);
        }

        let options = FindOptions::builder().build();
        let mut stats = self.player_stats().find(match namespace {
            Some(namespace) => doc! {
                "uuid": uuid_to_bson(uuid)?,
                "namespace": namespace.clone(),
            },
            None => doc! {
                "uuid": uuid_to_bson(uuid)?,
            },
        }, options).await?;

        let mut final_stats: HashMap<String, HashMap<String, f64>> = HashMap::new();
        while let Some(stats) = stats.try_next().await? {
            let mut s = HashMap::new();
            for (name, stat) in stats.stats {
                s.insert(name, stat.into());
            }
            final_stats.insert(stats.namespace, s);
        }

        Ok(Some(final_stats))
    }

    async fn ensure_player_stats_document(&self, uuid: &Uuid, namespace: &String) -> Result<()> {
        self.update_player_profile(uuid, None).await?; // Ensure that the player is tracked in the database.

        let options = FindOptions::builder().limit(1).build();
        let stats = self.player_stats().find(doc! {
            "uuid": uuid_to_bson(uuid)?,
            "namespace": namespace,
        }, options).await?.try_next().await?;

        if let None = stats {
            self.player_stats().insert_one(PlayerGameStats {
                uuid: uuid.clone(),
                namespace: namespace.clone(),
                stats: HashMap::new(),
            }, None).await?;
        }

        Ok(())
    }

    async fn upload_stats_bundle(&self, bundle: GameStatsBundle) -> Result<()> {
        for (player, stats) in bundle.stats {
            // Ensure that there is a document to upload stats to.
            self.ensure_player_stats_document(&player, &bundle.namespace).await?;
            for (stat_name, stat) in stats {
                self.player_stats().update_one(doc! {
                    "uuid": uuid_to_bson(&player)?,
                    "namespace": &bundle.namespace,
                }, stat.create_increment_operation(&stat_name), None).await?;
            }
        }

        Ok(())
    }
}

impl Actor for MongoDatabaseHandler {}

pub struct GetPlayerProfile(pub Uuid);
impl Message for GetPlayerProfile {
    type Result = Result<Option<PlayerProfile>>;
}

#[async_trait]
impl Handler<GetPlayerProfile> for MongoDatabaseHandler {
    async fn handle(&mut self, message: GetPlayerProfile, _ctx: &mut Context<Self>) -> <GetPlayerProfile as Message>::Result {
        self.get_player_profile(&message.0).await
    }
}

pub struct UpdatePlayerProfile {
    pub uuid: Uuid,
    pub username: String,
}

impl Message for UpdatePlayerProfile {
    type Result = Result<()>;
}

#[async_trait]
impl Handler<UpdatePlayerProfile> for MongoDatabaseHandler {
    async fn handle(&mut self, message: UpdatePlayerProfile, _ctx: &mut Context<Self>) -> <UpdatePlayerProfile as Message>::Result {
        self.update_player_profile(&message.uuid, Some(message.username)).await?;
        Ok(())
    }
}

pub struct GetPlayerStats {
    pub uuid: Uuid,
    pub namespace: Option<String>,
}

impl Message for GetPlayerStats {
    type Result = Result<Option<PlayerStatsResponse>>;
}

#[async_trait]
impl Handler<GetPlayerStats> for MongoDatabaseHandler {
    async fn handle(&mut self, message: GetPlayerStats, _ctx: &mut Context<Self>) -> <GetPlayerStats as Message>::Result {
        self.get_player_stats(&message.uuid, &message.namespace).await
    }
}

pub struct UploadStatsBundle(pub GameStatsBundle);

impl Message for UploadStatsBundle {
    type Result = Result<()>;
}

#[async_trait]
impl Handler<UploadStatsBundle> for MongoDatabaseHandler {
    async fn handle(&mut self, message: UploadStatsBundle, _ctx: &mut Context<Self>) -> <UploadStatsBundle as Message>::Result {
        self.upload_stats_bundle(message.0).await
    }
}
