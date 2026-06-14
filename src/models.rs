use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Problem {
    pub id: Uuid,
    pub platform: String,
    pub url: String,
    pub title: String,
    pub tags: Vec<String>,
}
