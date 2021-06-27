use serde::{Serialize, Deserialize};
use uuid::Uuid;
use bson::{Document, doc};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerProfile {
    #[serde(with = "bson::serde_helpers::uuid_as_binary")]
    pub uuid: Uuid,
    pub username: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerProfileResponse {
    pub uuid: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

impl From<PlayerProfile> for PlayerProfileResponse {
    fn from(p: PlayerProfile) -> Self {
        Self {
            uuid: p.uuid.clone(),
            username: p.username.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerGameStats {
    #[serde(with = "bson::serde_helpers::uuid_as_binary")]
    pub uuid: Uuid,
    pub namespace: String,
    pub stats: HashMap<String, PlayerStat>,
}

impl PlayerGameStats {
    pub fn create(uuid: &Uuid, namespace: &String) -> Self {
        Self {
            uuid: uuid.clone(),
            namespace: namespace.clone(),
            stats: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum PlayerStat {
    #[serde(rename = "itotal")]
    IntTotal(i32),
    #[serde(rename = "iaverage")]
    IntAverage {
        total: i32,
        count: i32,
    },
    #[serde(rename = "ftotal")]
    FloatTotal(f64),
    #[serde(rename = "faverage")]
    FloatAverage {
        total: f64,
        count: i32,
    },
}

impl Into<f64> for PlayerStat {
    fn into(self) -> f64 {
        match self {
            PlayerStat::IntTotal(v) => v as f64,
            PlayerStat::IntAverage { total, count } => (total as f64) / (count as f64),
            PlayerStat::FloatTotal(v) => v,
            PlayerStat::FloatAverage { total, count } => total / (count as f64),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GameStatsBundle {
    pub server_name: String,
    pub namespace: String,
    pub stats: HashMap<Uuid, HashMap<String, UploadStat>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum UploadStat {
    IntTotal(i32),
    IntRollingAverage(i32),
    FloatTotal(f64),
    FloatRollingAverage(f64),
}

impl UploadStat {
    /// Generate a BSON document for increasing this value.
    pub fn create_increment_operation(&self, id: &String) -> Document {
        let value_key = format!("stats.{}.value", id);
        let total_key = format!("{}.total", value_key);
        let count_key = format!("{}.count", value_key);

        return match self {
            UploadStat::IntTotal(value) => doc! { "$inc": { value_key: value } },
            UploadStat::IntRollingAverage(value) => doc! { "$inc": { total_key: value, count_key: 1 } },
            UploadStat::FloatTotal(value) => doc! { "$inc": { value_key: value } },
            UploadStat::FloatRollingAverage(value) => doc! { "$inc": { total_key: value, count_key: 1 } },
        }
    }
}
