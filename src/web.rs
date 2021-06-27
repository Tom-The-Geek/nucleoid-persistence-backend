use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::Filter;
use warp::http::StatusCode;
use xtra::Address;

use crate::config::Config;
use crate::database::{GetPlayerProfile, MongoDatabaseHandler, UpdatePlayerProfile, GetPlayerStats, UploadStatsBundle};
use crate::model::{PlayerProfileResponse, GameStatsBundle};

#[derive(Serialize, Deserialize)]
pub struct PlayerStats(HashMap<String, i32>);

pub async fn run(config: &Config, database: Address<MongoDatabaseHandler>) {
    let cors = warp::cors()
        .allow_any_origin();

    let player_profile = warp::path("player")
        .and(warp::path::param::<Uuid>())
        .and(warp::filters::method::get())
        .and(warp::filters::path::end())
        .and_then({
            let database = database.clone();
            move |uuid| get_player_profile(database.clone(), uuid)
        });

    let update_player_profile = warp::path("player")
        .and(warp::path::param::<Uuid>())
        .and(warp::filters::path::end())
        .and(warp::filters::method::put())
        .and(warp::header("authorization"))
        .and(warp::filters::body::json())
        .and_then({
            let config = config.clone();
            let database = database.clone();
            move |uuid, authorization, body: UpdatePlayerProfileRequest| update_player_profile(config.clone(), database.clone(), uuid, authorization, body.username)
        });

    let player_game_stats = warp::path("player")
        .and(warp::path::param::<Uuid>())
        .and(warp::path("stats"))
        .and(warp::path::param::<String>())
        .and_then({
            let database = database.clone();
            move |uuid, game_mode| get_player_stats(database.clone(), uuid, game_mode)
        });

    let upload_game_stats = warp::path("stats")
        .and(warp::path("upload"))
        .and(warp::filters::method::post())
        .and(warp::header("Authorization"))
        .and(warp::filters::body::json())
        .and_then({
            let config = config.clone();
            let database = database.clone();
            move |authorization, game_stats: GameStatsBundle|
                upload_game_stats(config.clone(), database.clone(), authorization, game_stats)
        });

    let combined = player_profile
        // Management
        .or(update_player_profile)
        // Stats
        .or(player_game_stats)
        .or(upload_game_stats);

    warp::serve(combined.with(cors))
        .run(([127, 0, 0, 1], config.api_port))
        .await;
}

type ApiResult = Result<Box<dyn warp::Reply>, warp::Rejection>;

async fn get_player_stats(database: Address<MongoDatabaseHandler>, uuid: Uuid, game_mode: String) -> ApiResult {
    let res = database.send(GetPlayerStats {
        uuid,
        namespace: game_mode,
    }).await.unwrap();
    return match res {
        Ok(stats) => {
            Ok(if let Some(stats) = stats {
                let mut stat_values: HashMap<String, f64> = HashMap::new();
                for (key, value) in stats.stats {
                    stat_values.insert(key, value.into());
                }
                Box::new(warp::reply::json(&stat_values))
            } else {
                send_http_status(StatusCode::NOT_FOUND)
            })
        },
        Err(e) => {
            Ok(handle_server_error(&e))
        }
    }
}

async fn get_player_profile(database: Address<MongoDatabaseHandler>, uuid: Uuid) -> ApiResult {
    let res = database.send(GetPlayerProfile(uuid)).await.unwrap();
    return match res {
        Ok(profile) => {
            Ok(if let Some(profile) = profile {
                Box::new(warp::reply::json(&PlayerProfileResponse::from(profile)))
            } else {
                send_http_status(StatusCode::NOT_FOUND)
            })
        },
        Err(e) => {
            Ok(handle_server_error(&e))
        }
    }
}

#[derive(Serialize, Deserialize)]
struct UpdatePlayerProfileRequest {
    username: String,
}

async fn update_player_profile(config: Config, database: Address<MongoDatabaseHandler>, uuid: Uuid, authorization: String, username: String) -> ApiResult {
    if !config.server_tokens.contains(&authorization) {
        return Ok(send_http_status(StatusCode::UNAUTHORIZED))
    }

    let res = database.send(UpdatePlayerProfile {
        uuid, username
    }).await.unwrap();

    match res {
        Ok(_) => Ok(Box::new(warp::reply::with_status("", StatusCode::NO_CONTENT))),
        Err(e) => Ok(handle_server_error(&e))
    }
}

#[derive(Serialize, Deserialize)]
struct UpdatedResponse {
    updated: bool,
}

async fn upload_game_stats(config: Config, database: Address<MongoDatabaseHandler>, authorization: String, game_stats: GameStatsBundle) -> ApiResult {
    if !config.server_tokens.contains(&authorization) {
        return Ok(send_http_status(StatusCode::UNAUTHORIZED))
    }

    log::debug!("server '{}' uploaded {} statistics in statistics bundle for {}",
                game_stats.server_name, game_stats.stats.len(), game_stats.namespace);

    for (_, stats) in game_stats.stats {
        for (name, _) in stats {
            if name.contains('.') {
                return Ok(send_http_status(StatusCode::BAD_REQUEST));
            }
        }
    }

    let res = database.send(UploadStatsBundle(game_stats)).await.unwrap();
    match res {
        Ok(()) => Ok(Box::new(warp::reply::with_status("", StatusCode::NO_CONTENT))),
        Err(e) => Ok(handle_server_error(&e)),
    }
}

fn handle_server_error(e: &anyhow::Error) -> Box<dyn warp::Reply> {
    log::warn!("error handling request: {}", e);
    send_http_status(StatusCode::INTERNAL_SERVER_ERROR)
}

fn send_http_status(status: StatusCode) -> Box<dyn warp::Reply> {
    Box::new(warp::reply::with_status(status.canonical_reason().unwrap_or_else(|| ""), status))
}
