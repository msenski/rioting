use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::camera::OnvifCamera;
use crate::config::Config;
use crate::webrtc::new_rtc;

use axum::extract::{Json as ExtractJson, Path as ExtractPath, State as ExtractState}; // custom aliases for explicitness
use axum::http::{HeaderValue, StatusCode, header::CACHE_CONTROL};
use axum::{
    Json, Router, middleware,
    routing::{get, post},
};
use retina::codec::VideoFrame;
use str0m::Rtc;
use tokio::sync::broadcast::Sender;
use tower_http::services::ServeDir;

const STATIC_DIR: &str = "static";
const HLS_DIR: &str = "hls";

#[derive(serde::Deserialize)]
struct PtzMoveRequest {
    pan: f32,
    tilt: f32,
}

struct WebRtcSession {
    rtc: Rtc,
    camera_name: String,
}

type WebRtcSessions = Arc<Mutex<HashMap<String, WebRtcSession>>>;

#[derive(Clone)]
struct WebRtcState {
    camera_channels: Arc<HashMap<String, Sender<Arc<VideoFrame>>>>,
    sessions: WebRtcSessions,
}

async fn no_cache(response: axum::response::Response) -> axum::response::Response {
    let mut response = response;
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store"),
    );
    response
}

/// Builds and runs the HTTP server on port 3000.
///
/// Serves static files from the default static directory, mounts each camera's
/// HLS output under `/hls/<camera_name>`, and exposes `/cameras` as a JSON list
/// of camera names.
pub async fn serve(
    config: &Config,
    cameras: Arc<HashMap<String, Arc<OnvifCamera>>>,
    camera_channels: Arc<HashMap<String, Sender<Arc<VideoFrame>>>>,
) -> anyhow::Result<()> {
    let names: Vec<String> = config
        .cameras
        .iter()
        .map(|cam_cfg| cam_cfg.name.clone())
        .collect();

    let mut router = Router::new()
        .route("/cameras", get(move || async move { Json(names) }))
        .fallback_service(ServeDir::new(STATIC_DIR));

    // Add camera-specific paths for the HLS files
    for cam_cfg in config.cameras.iter() {
        let hls_path = format!("/hls/{}", cam_cfg.name);
        let fs_path = PathBuf::from(HLS_DIR).join(&cam_cfg.name);
        router = router.nest_service(&hls_path, ServeDir::new(fs_path));
    }

    let webrtc_state = WebRtcState {
        camera_channels: camera_channels.clone(),
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    // Add WebRTC offer handling (using separate Router just for better readability)
    let webrtc_router = Router::new()
        .route("/cameras/{name}/webrtc/offer", post(handle_webrtc_offer))
        .with_state(webrtc_state.clone());
    router = router.merge(webrtc_router);

    // Add PTZ handling (using separate Router just for better readability)
    let ptz_router = Router::new()
        .route("/cameras/{name}/ptz/move", post(handle_ptz_move))
        .route("/cameras/{name}/ptz/stop", post(handle_ptz_stop))
        .with_state(cameras);
    router = router.merge(ptz_router);

    // Sometimes browsers cache the HLS playlist or segments, such that
    // new segments are not displayed. We disable caching by default.
    router = router.layer(middleware::map_response(no_cache));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.server_port)).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

async fn handle_ptz_move(
    ExtractState(cameras): ExtractState<Arc<HashMap<String, Arc<OnvifCamera>>>>,
    ExtractPath(name): ExtractPath<String>,
    ExtractJson(body): ExtractJson<PtzMoveRequest>,
) -> StatusCode {
    let Some(camera) = cameras.get(&name) else {
        return StatusCode::NOT_FOUND;
    };
    match camera.ptz_move(body.pan, body.tilt).await {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn handle_ptz_stop(
    ExtractState(cameras): ExtractState<Arc<HashMap<String, Arc<OnvifCamera>>>>,
    ExtractPath(name): ExtractPath<String>,
) -> StatusCode {
    let Some(camera) = cameras.get(&name) else {
        return StatusCode::NOT_FOUND;
    };
    match camera.ptz_stop().await {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn handle_webrtc_offer(
    ExtractState(state): ExtractState<WebRtcState>,
    ExtractPath(name): ExtractPath<String>,
    ExtractJson(offer): ExtractJson<String>,
) -> StatusCode {
    println!("name: {}", name);
    println!("offer: {}", offer);

    // Credentials are created aby and tied to one Rtc instance. We hold the Rtc
    // instance for as long as the WebRTC session is active
    let rtc = new_rtc();

    let session = WebRtcSession {
        rtc,
        camera_name: name.clone(),
    };
    match state.sessions.lock() {
        Ok(mut sessions) => {
            sessions.insert(name.clone(), session);
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    }
    StatusCode::NOT_IMPLEMENTED
}
