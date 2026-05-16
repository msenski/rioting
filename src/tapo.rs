use crate::camera::Camera;
use crate::config::CameraConfig;
use async_trait::async_trait;
use url::Url;

pub struct TapoCamera {
    camera_config: CameraConfig,
    rtsp_url: Url,
}

impl TapoCamera {
    pub fn new(camera_config: CameraConfig) -> anyhow::Result<Self> {
        // TODO support different quality stream paths
        let rtsp_url = Url::parse(&format!("rtsp://{}:554/stream1", camera_config.ip,))?;
        Ok(TapoCamera {
            camera_config,
            rtsp_url,
        })
    }
}

#[async_trait]
impl Camera for TapoCamera {
    fn rtsp_url(&self) -> &Url {
        &self.rtsp_url
    }

    async fn ptz_move(&self, pan: f32, tilt: f32) -> anyhow::Result<()> {
        todo!()
    }

    async fn ptz_stop(&self) -> anyhow::Result<()> {
        todo!()
    }
}
