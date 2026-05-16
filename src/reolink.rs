use crate::camera::Camera;
use crate::config::CameraConfig;
use async_trait::async_trait;
use url::Url;

pub struct ReolinkCamera {
    camera_config: CameraConfig,
    rtsp_url: Url,
}

impl ReolinkCamera {
    pub fn new(camera_config: CameraConfig) -> anyhow::Result<Self> {
        // TODO support different quality stream paths
        let rtsp_url = Url::parse(&format!(
            "rtsp://{}:554/h264Preview_01_main",
            camera_config.ip,
        ))?;
        Ok(ReolinkCamera {
            camera_config,
            rtsp_url,
        })
    }
}

#[async_trait]
impl Camera for ReolinkCamera {
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
