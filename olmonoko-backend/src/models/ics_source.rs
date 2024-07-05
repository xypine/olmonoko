use crate::utils::time::from_timestamp;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RawIcsSource {
    pub id: i64,
    pub user_id: i64,
    pub is_public: bool,
    pub name: String,
    pub url: String,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub last_fetched_at: Option<i64>,
    pub persist_events: bool,
    pub all_as_allday: bool,
    pub import_template: Option<String>,
    pub file_hash: Option<String>,
    pub object_hash: Option<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IcsSource {
    pub id: i64,
    pub user_id: i64,
    pub is_public: bool,
    pub name: String,
    pub url: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_fetched_at: Option<chrono::DateTime<chrono::Utc>>,
    pub persist_events: bool,
    pub all_as_allday: bool,
    pub import_template: Option<String>,
    pub chosen_priority: Option<i64>,
    pub file_hash: Option<String>,
    pub object_hash: Option<String>,
}
impl From<(RawIcsSource, Option<i64>)> for IcsSource {
    fn from((raw, chosen_priority): (RawIcsSource, Option<i64>)) -> Self {
        Self {
            id: raw.id,
            user_id: raw.user_id,
            is_public: raw.is_public,
            name: raw.name,
            url: raw.url,
            created_at: from_timestamp(raw.created_at),
            updated_at: raw.updated_at.map(from_timestamp),
            last_fetched_at: raw.last_fetched_at.map(from_timestamp),
            persist_events: raw.persist_events,
            all_as_allday: raw.all_as_allday,
            import_template: raw.import_template,
            chosen_priority,
            file_hash: raw.file_hash,
            object_hash: raw.object_hash,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct NewIcsSource {
    pub user_id: i64,
    pub is_public: bool,
    pub name: String,
    pub url: String,
    pub last_fetched_at: Option<i64>,
    pub import_template: Option<String>,
}

use serde_with::As;
use serde_with::NoneAsEmptyString;
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct IcsSourceForm {
    #[serde(deserialize_with = "deserialize_checkbox", default)]
    pub is_public: bool,
    pub name: String,
    pub url: String,
    #[serde(default, with = "As::<NoneAsEmptyString>")]
    pub import_template: Option<String>,
}

use serde::de;

pub fn deserialize_checkbox<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: &str = de::Deserialize::deserialize(deserializer)?;

    match s {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(de::Error::unknown_variant(s, &["on", "off"])),
    }
}

pub fn serialize_checkbox<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if *value {
        serializer.serialize_str("on")
    } else {
        serializer.serialize_str("off")
    }
}
