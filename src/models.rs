use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub id: Uuid,
    pub platform: String,
    pub url: String,
    pub title: String,
    pub tags: Vec<String>,
    pub difficulty: u16,
    pub ac_count: u8,
    pub last_solved_at: Option<String>
}

#[derive(Debug, Deserialize)]
pub struct CreateProblemRequest {
    pub platform: String,
    pub url: String,
    pub title: String,
    pub tags: Vec<String>,
    pub difficulty: u16,
}
