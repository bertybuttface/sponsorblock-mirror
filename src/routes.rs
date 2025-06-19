use std::collections::HashMap;

use actix_web::{web, HttpResponse, Result};
use lazy_static::lazy_static;
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::{Segment, Sponsor};
use crate::models::SponsorTime;
use crate::structs::{HealthResponse, HealthChecks, HealthCheck};

#[derive(OpenApi)]
#[openapi(
    paths(
        skip_segments,
        skip_segments_by_id,
        fake_is_user_vip,
        fake_user_info,
        health_check,
        metrics
    ),
    components(
        schemas(Sponsor, Segment, SponsorTime, HealthResponse, HealthChecks, HealthCheck)
    ),
    tags(
        (name = "Skip Segments", description = "SponsorBlock segment retrieval endpoints"),
        (name = "User Info", description = "User information endpoints (mocked for ReVanced compatibility)"),
        (name = "Health", description = "Service health monitoring endpoints"),
        (name = "Metrics", description = "Prometheus metrics endpoints")
    ),
    info(
        title = "SponsorBlock Mirror API",
        description = "A mirror of the SponsorBlock API for retrieving video sponsor segments",
        version = "1.0.0",
        contact(
            name = "API Support"
        )
    )
)]
pub struct ApiDoc;

// init regexes to match hash/hex or video ID
lazy_static! {
    static ref HASH_RE: regex::Regex = regex::Regex::new(r"^[0-9a-f]{4}$").unwrap();
    static ref ID_RE: regex::Regex = regex::Regex::new(r"^[a-zA-Z0-9_-]{6,11}$").unwrap();
}

// Segments can be fetched either by full video ID, or by prefix of hashed
// video ID. Different clients make different queries. This represents either
// kind of constraint.
enum VideoName {
    ByHashPrefix(String),
    ByID(String),
}


#[utoipa::path(
    get,
    path = "/api/skipSegments/{hash}",
    params(
        ("hash" = String, Path, description = "4-character hex prefix of hashed video ID")
    ),
    params(
        ("categories" = Option<String>, Query, description = "JSON array of sponsor categories to filter by")
    ),
    responses(
        (status = 200, description = "List of sponsors with segments", body = [Sponsor]),
        (status = 400, description = "Invalid hash format")
    ),
    tag = "Skip Segments"
)]
pub async fn skip_segments(
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse> {
    let hash = path.into_inner().to_lowercase();
    let categories = query.get("categories");

    // Check if hash matches hex regex
    if !HASH_RE.is_match(&hash) {
        return Ok(HttpResponse::BadRequest().body("Hash prefix does not match format requirements."));
    }

    let sponsors = find_skip_segments(VideoName::ByHashPrefix(hash.clone()), categories.map(|s| s.as_str()), &db).await;

    if sponsors.is_empty() {
        // Fall back to central Sponsorblock server
        let resp = reqwest::get(format!(
            "https://sponsor.ajay.app/api/skipSegments/{}?categories={}",
            hash,
            categories.map(|s| s.as_str()).unwrap_or("[\"sponsor\"]"),
        ))
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        return Ok(HttpResponse::Ok().content_type("application/json").body(resp));
    }

    Ok(HttpResponse::Ok().json(&sponsors))
}

#[utoipa::path(
    get,
    path = "/api/skipSegments",
    params(
        ("videoID" = String, Query, description = "YouTube video ID (6-11 characters)"),
        ("categories" = Option<String>, Query, description = "JSON array of sponsor categories to filter by")
    ),
    responses(
        (status = 200, description = "List of segments for the video", body = [Segment]),
        (status = 400, description = "Invalid or missing videoID")
    ),
    tag = "Skip Segments"
)]
pub async fn skip_segments_by_id(
    query: web::Query<HashMap<String, String>>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse> {
    let video_id = match query.get("videoID") {
        Some(id) => id,
        None => return Ok(HttpResponse::BadRequest().body("videoID parameter is required")),
    };
    let categories = query.get("categories");

    // Check if ID matches ID regex
    if !ID_RE.is_match(video_id) {
        return Ok(HttpResponse::BadRequest().body("videoID does not match format requirements"));
    }

    let sponsors = find_skip_segments(VideoName::ByID(video_id.clone()), categories.map(|s| s.as_str()), &db).await;

    if sponsors.is_empty() {
        // Fall back to central Sponsorblock server
        let resp = reqwest::get(format!(
            "https://sponsor.ajay.app/api/skipSegments?videoID={}&categories={}",
            video_id,
            categories.map(|s| s.as_str()).unwrap_or("[\"sponsor\"]"),
        ))
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        return Ok(HttpResponse::Ok().content_type("application/json").body(resp));
    }

    // Doing a lookup by video ID should return only one Sponsor object with
    // one list of segments. We need to return just the list of segments.
    Ok(HttpResponse::Ok().json(&sponsors[0].segments))
}

async fn find_skip_segments(
    name: VideoName,
    categories: Option<&str>,
    db: &PgPool,
) -> Vec<Sponsor> {
    let cat: Vec<String> = serde_json::from_str(categories.unwrap_or("[\"sponsor\"]")).unwrap();

    if cat.is_empty() {
        return Vec::new();
    }

    let results: Vec<SponsorTime> = match name {
        VideoName::ByHashPrefix(hash_prefix) => {
            sqlx::query_as::<_, SponsorTime>(
                r#"SELECT * FROM "sponsorTimes" 
                   WHERE "shadowHidden" = 0 
                   AND "hidden" = 0 
                   AND "votes" >= 0 
                   AND "category" = ANY($1)
                   AND "hashedVideoID" LIKE $2"#,
            )
            .bind(&cat)
            .bind(format!("{}%", hash_prefix))
            .fetch_all(db)
            .await
            .expect("Failed to query sponsor times")
        }
        VideoName::ByID(video_id) => {
            sqlx::query_as::<_, SponsorTime>(
                r#"SELECT * FROM "sponsorTimes" 
                   WHERE "shadowHidden" = 0 
                   AND "hidden" = 0 
                   AND "votes" >= 0 
                   AND "category" = ANY($1)
                   AND "videoID" = $2"#,
            )
            .bind(&cat)
            .bind(video_id)
            .fetch_all(db)
            .await
            .expect("Failed to query sponsor times")
        }
    };

    // Create map of Sponsors - Hash, Sponsor
    let mut sponsors: HashMap<String, Sponsor> = HashMap::new();

    for result in &results {
        let sponsor = {
            sponsors.entry(result.hashed_video_id.clone()).or_insert(Sponsor {
                hash: result.hashed_video_id.clone(),
                video_id: result.video_id.clone(),
                segments: Vec::new(),
            })
        };

        let segment = build_segment(result);

        let hash = result.hashed_video_id.clone();

        let mut found_similar = false;

        for seg in &sponsor.segments {
            if is_overlap(&segment, &seg.category, &seg.action_type, seg.segment[0], seg.segment[1]) {
                found_similar = true;
                break;
            }
        }

        if found_similar {
            continue;
        }

        let mut similar_segments = similar_segments(&segment, &hash, &results);
        similar_segments.push(segment.clone());

        let best_segment = best_segment(&similar_segments);

        // Add if not already in sponsor
        if !sponsor.segments.contains(&best_segment) {
            sponsor.segments.push(best_segment);
        }
    }

    for sponsor in sponsors.values_mut() {
        sponsor.segments.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }

    sponsors.into_values().collect()
}

fn similar_segments(segment: &Segment, hash: &str, segments: &Vec<SponsorTime>) -> Vec<Segment> {
    let mut similar_segments: Vec<Segment> = Vec::new();

    for seg in segments {
        if seg.uuid == segment.uuid {
            continue;
        }

        if seg.hashed_video_id != hash {
            continue;
        }

        let is_similar = is_overlap(segment, &seg.category, &seg.action_type, seg.start_time, seg.end_time);

        if is_similar {
            similar_segments.push(build_segment(seg));
        }
    }

    similar_segments
}

fn is_overlap(seg: &Segment, cat: &str, action_type: &str, start: f32, end: f32) -> bool {
    if seg.category != cat {
        return false;
    }

    if seg.segment[0] > start && seg.segment[1] < end {
        return true;
    }
    let overlap = f32::min(seg.segment[1], end) - f32::max(seg.segment[0], start);
    let duration = f32::max(seg.segment[1], end) - f32::min(seg.segment[0], start);
    overlap / duration > {
        if cat == "chapter" {
            0.8
        } else if seg.action_type == action_type {
            0.6
        } else {
            0.1
        }
    }
}

fn best_segment(segments: &Vec<Segment>) -> Segment {
    let mut best_segment = segments[0].clone();
    let mut best_votes = segments[0].votes;

    for segment in segments {
        if segment.votes > best_votes {
            best_segment = segment.clone();
            best_votes = segment.votes;
        }
    }

    best_segment
}

fn build_segment(sponsor_time: &SponsorTime) -> Segment {
    Segment {
        uuid: sponsor_time.uuid.clone(),
        action_type: sponsor_time.action_type.clone(),
        category: sponsor_time.category.clone(),
        description: sponsor_time.description.clone(),
        locked: sponsor_time.locked,
        segment: vec![sponsor_time.start_time, sponsor_time.end_time],
        user_id: sponsor_time.user_id.clone(),
        video_duration: sponsor_time.video_duration,
        votes: sponsor_time.votes,
    }
}

// These additional routes are faked to protect ReVanced from seeing errors. We
// don't *need* to do this to support ReVanced, but it gets rid of the
// perpetual "Loading..." in the settings.

#[utoipa::path(
    get,
    path = "/api/isUserVIP",
    responses(
        (status = 200, description = "User VIP status", body = serde_json::Value)
    ),
    tag = "User Info"
)]
pub async fn fake_is_user_vip() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "hashedUserID": "",
        "vip": false
    })))
}

#[utoipa::path(
    get,
    path = "/api/userInfo",
    responses(
        (status = 200, description = "User information and statistics", body = serde_json::Value)
    ),
    tag = "User Info"
)]
pub async fn fake_user_info() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "userID": "",
        "userName": "",
        "minutesSaved": 0,
        "segmentCount": 0,
        "viewCount": 0
    })))
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Service is unhealthy", body = HealthResponse)
    ),
    tag = "Health"
)]
pub async fn health_check(db: web::Data<PgPool>) -> Result<HttpResponse> {
    use std::time::Instant;
    
    let start = Instant::now();
    
    // Check database connectivity
    let db_check = match sqlx::query("SELECT 1").fetch_one(db.as_ref()).await {
        Ok(_) => HealthCheck {
            status: "healthy".to_string(),
            message: Some("Database connection successful".to_string()),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
        },
        Err(e) => HealthCheck {
            status: "unhealthy".to_string(),
            message: Some(format!("Database connection failed: {}", e)),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
        },
    };
    
    let overall_status = if db_check.status == "healthy" {
        "healthy"
    } else {
        "unhealthy"
    };
    
    let health_response = HealthResponse {
        status: overall_status.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        checks: HealthChecks {
            database: db_check,
        },
    };
    
    let status_code = if overall_status == "healthy" {
        actix_web::http::StatusCode::OK
    } else {
        actix_web::http::StatusCode::SERVICE_UNAVAILABLE
    };
    
    Ok(HttpResponse::build(status_code).json(&health_response))
}

#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, description = "Prometheus metrics in text format", body = String)
    ),
    tag = "Metrics"
)]
pub async fn metrics() -> Result<HttpResponse> {
    // This endpoint is handled by actix-web-prom middleware
    // This function is just for OpenAPI documentation
    Ok(HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body("# Metrics handled by actix-web-prom middleware"))
}
