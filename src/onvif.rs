use base64::{Engine, prelude::BASE64_STANDARD};
use rand::RngExt;
use sha1::{Digest, Sha1};

use reqwest::header::CONTENT_TYPE;

use url::Url;

use crate::config::CameraConfig;

const NS_SOAP: &str = "http://www.w3.org/2003/05/soap-envelope";
const NS_DEVICE: &str = "http://www.onvif.org/ver10/device/wsdl";
const NS_SCHEMA: &str = "http://www.onvif.org/ver10/schema";

/// ONVIF PTZ client for a single camera.
///
/// ONVIF is an industry standard for IP camera interoperability. It exposes camera
/// functionality (PTZ, media profiles, device info) as SOAP services — HTTP POST
/// requests with XML bodies.
///
/// # How it works
///
/// Every PTZ command follows this flow:
///
/// 1. **Authenticate** — ONVIF rejects plain passwords. Every request must include a
///    WS-Security header containing a digest: `Base64(SHA-1(nonce + timestamp + password))`.
///    The nonce is random bytes generated fresh per request. [`OnvifClient`] builds this
///    header automatically via a private method.
///
/// 2. **Discover service URLs** — ONVIF does not require services to live at fixed paths.
///    [`OnvifClient::connect`] calls `GetCapabilities` on the Device service at construction
///    time. The response contains the actual URLs (`XAddr`) for each service on this specific
///    camera. These are stored on the struct and reused for all subsequent calls. This is
///    what makes the client work across different camera brands without hardcoding paths.
///
/// 3. **Get a profile token** — a camera can expose multiple *media profiles*, each
///    representing a different stream configuration (e.g. "MainStream" at full resolution,
///    "SubStream" at low resolution for mobile). Each profile has a unique token string
///    chosen by the manufacturer. PTZ commands require you to specify which profile you
///    are targeting.
///
///    In practice, all profiles on the same camera move the same physical lens, so it
///    doesn't matter which profile token you use for PTZ. Call [`OnvifClient::get_profile_token`]
///    once at startup — it returns the token of the first profile in the list. Reuse this
///    token for all subsequent PTZ calls.
///
/// 4. **Send PTZ commands** — with a token in hand, call [`OnvifClient::continuous_move`]
///    to start panning/tilting and [`OnvifClient::stop`] to halt.
///
/// # Construction
///
/// Use [`OnvifClient::connect`] (async) — it runs `GetCapabilities` and returns a fully
/// initialised client:
///
/// ```ignore
/// let client = OnvifClient::connect(camera_config).await?;
/// let token  = client.get_profile_token().await?;
/// client.continuous_move(&token, 0.5, 0.0).await?;
/// client.stop(&token).await?;
/// ```
pub struct OnvifClient {
    client: reqwest::Client,
    camera_config: CameraConfig,
    media_service_url: String,
    ptz_service_url: String,
}

impl OnvifClient {
    pub async fn connect(camera_config: CameraConfig) -> anyhow::Result<Self> {
        let client = reqwest::Client::new();

        // The device (management) service is at /onvif/device_service. See section 5.1.1 in
        // https://www.onvif.org/specs/core/ONVIF-Core-Specification.pdf
        let device_service_url = Url::parse(&format!(
            "http://{ip}:{onvif_port}/onvif/device_service",
            ip = &camera_config.ip,
            onvif_port = camera_config.onvif_port()
        ))?;

        let body = format!(
            r#"
            <s:Envelope xmlns:s="{NS_SOAP}">
                <s:Header>{auth_header}</s:Header>
                <s:Body>
                <tds:GetCapabilities xmlns:tds="{NS_DEVICE}">
                    <tds:Category>All</tds:Category>
                </tds:GetCapabilities>
                </s:Body>
            </s:Envelope>
            "#,
            auth_header = build_auth_header(&camera_config),
        );

        let res = client
            .post(device_service_url)
            .header(CONTENT_TYPE, "application/soap+xml")
            .body(body)
            .send()
            .await?
            .text()
            .await?;

        let doc = roxmltree::Document::parse(&res)?;

        let media_service_url = find_xaddr(&doc, "Media")?;
        let ptz_service_url = find_xaddr(&doc, "PTZ")?;

        Ok(OnvifClient {
            client,
            camera_config,
            media_service_url,
            ptz_service_url,
        })
    }
}

fn build_auth_header(camera_config: &CameraConfig) -> String {
    // See https://www.onvif.org/wp-content/uploads/2016/12/ONVIF_WG-APG-Application_Programmers_Guide-1.pdf
    // section 6.1 for info on generating the digest.
    let raw_nonce: [u8; 16] = rand::rng().random();
    let created_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let mut hasher = Sha1::new();
    hasher.update(raw_nonce);
    hasher.update(created_at.as_bytes());
    hasher.update(camera_config.password.as_bytes());

    let digest = BASE64_STANDARD.encode(hasher.finalize());

    // We need the nonce Base64-encoded separately for the header's wsse:Nonce child
    let nonce_base64 = BASE64_STANDARD.encode(raw_nonce);

    // Note: XML ignores whitespaces between elements, so we can
    // indent the string to make it more readable
    format!(
        r#"
            <wsse:Security
              xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"
              xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
              <wsse:UsernameToken>
                <wsse:Username>{username}</wsse:Username>
                <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{digest}</wsse:Password>
                <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">{nonce}</wsse:Nonce>
                <wsu:Created>{created}</wsu:Created>
              </wsse:UsernameToken>
            </wsse:Security>
            "#,
        username = camera_config.user,
        digest = digest,
        nonce = nonce_base64,
        created = created_at,
    )
}

fn find_xaddr(doc: &roxmltree::Document, service: &str) -> anyhow::Result<String> {
    // According to ONVIF, `XAddr` elements represent service addresses.
    // See section 8.1.2.1 in ONVIF-Core-Specification
    //
    doc.descendants()
        .find(|n| n.tag_name().name() == service && n.tag_name().namespace() == Some(NS_SCHEMA))
        .and_then(|n| {
            n.children().find(|c| {
                c.tag_name().name() == "XAddr" && c.tag_name().namespace() == Some(NS_SCHEMA)
            })
        })
        .and_then(|n| n.text())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No XAddr found for service: {service}"))
}
