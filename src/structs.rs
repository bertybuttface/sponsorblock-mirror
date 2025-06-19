use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct Sponsor {
    pub hash: String,
    #[serde(rename = "videoID")]
    pub video_id: String,
    pub segments: Vec<Segment>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct Segment {
    #[serde(rename = "UUID")]
    pub uuid: String,
    #[serde(rename = "actionType")]
    pub action_type: String,
    pub category: String,
    pub description: String,
    pub locked: i32,
    pub segment: Vec<f32>,
    #[serde(rename = "userID")]
    pub user_id: String,
    #[serde(rename = "videoDuration")]
    pub video_duration: f32,
    pub votes: i32,
}

impl PartialEq for Segment {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl PartialOrd for Segment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.segment[0].partial_cmp(&other.segment[0])
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub checks: HealthChecks,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HealthChecks {
    pub database: HealthCheck,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HealthCheck {
    pub status: String,
    pub message: Option<String>,
    pub response_time_ms: Option<u64>,
}
