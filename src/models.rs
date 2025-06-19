use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct SponsorTime {
    #[serde(rename = "videoID")]
    #[sqlx(rename = "videoID")]
    pub video_id: String,
    #[serde(rename = "startTime")]
    #[sqlx(rename = "startTime")]
    pub start_time: f32,
    #[serde(rename = "endTime")]
    #[sqlx(rename = "endTime")]
    pub end_time: f32,
    pub votes: i32,
    pub locked: i32,
    #[serde(rename = "incorrectVotes")]
    #[sqlx(rename = "incorrectVotes")]
    pub incorrect_votes: i32,
    #[serde(rename = "UUID")]
    #[sqlx(rename = "UUID")]
    pub uuid: String,
    #[serde(rename = "userID")]
    #[sqlx(rename = "userID")]
    pub user_id: String,
    #[serde(rename = "timeSubmitted")]
    #[sqlx(rename = "timeSubmitted")]
    pub time_submitted: i64,
    pub views: i32,
    pub category: String,
    #[serde(rename = "actionType")]
    #[sqlx(rename = "actionType")]
    pub action_type: String,
    pub service: String,
    #[serde(rename = "videoDuration")]
    #[sqlx(rename = "videoDuration")]
    pub video_duration: f32,
    pub hidden: i32,
    pub reputation: f32,
    #[serde(rename = "shadowHidden")]
    #[sqlx(rename = "shadowHidden")]
    pub shadow_hidden: i32,
    #[serde(rename = "hashedVideoID")]
    #[sqlx(rename = "hashedVideoID")]
    pub hashed_video_id: String,
    #[serde(rename = "userAgent")]
    #[sqlx(rename = "userAgent")]
    pub user_agent: String,
    pub description: String,
}
