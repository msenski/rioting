# rioting

Home security system. Reads RTSP streams from IP cameras and serves them as HLS video in a browser. Supports PTZ (pan/tilt/zoom) control for ONVIF-compatible cameras.

```
Camera (RTSP) ──► rioting ──► ffmpeg ──► HLS files on disk ──► axum HTTP server ──► browser
                                                                        ▲
                                        PTZ (ONVIF/SOAP) ◄─────────────┘
```

Each camera gets its own ffmpeg process that segments the stream into 2-second `.ts` chunks. The browser player fetches `/cameras` to discover which cameras are configured, then plays each one with a d-pad for PTZ control.

## Requirements

- Rust (install via [rustup](https://rustup.rs))
- ffmpeg (`sudo apt install ffmpeg`)

## Camera setup

### Finding your camera's IP address

The easiest way is to log into your router's admin UI (usually at `192.168.1.1` or `192.168.178.1`) and look for connected devices. Most routers list hostnames alongside IP addresses, so the camera should be easy to spot. Assign it a static/reserved IP so it doesn't change on reboot.

### Enabling RTSP and ONVIF access

Most cameras ship with RTSP and third-party (ONVIF) access disabled by default. You need to enable these in the camera's own settings before rioting can connect.

**Tapo cameras**
- Open the Tapo app → select your camera → Settings
- Enable *Advanced Settings → RTSP*
- Enable *Advanced Settings → ONVIF* (required for PTZ control)
- See the [Tapo camera user guide](https://www.tp-link.com/us/support/faq/2680/) for details

**Reolink cameras**
- Open the Reolink app or web UI → Camera Settings
- Enable *Encoding → Clear RTSP*
- ONVIF is enabled by default on most Reolink models
- See the [Reolink support page](https://support.reolink.com/articles/900000621783-How-to-Configure-Reolink-Ports-Settings/) for details

## Configuration

Copy `config.example.toml` to `config.toml` and edit it:

```toml
server_port = "3000"

[[cameras]]
name = "dining-area"
ip = "192.168.178.x"
user = "your-camera-user"
password = "your-camera-password"
```

`config.toml` is gitignored — it contains credentials.

## Running locally

```bash
cargo run -- --config-path config.toml
```

Then open `http://localhost:3000`.

## Cross-compiling for another target

The Makefile handles cross-compilation and deployment. The example below targets a **Raspberry Pi** (`aarch64`), but the same approach works for any supported Rust target.

### Option A — using `cross` (recommended, requires Docker)

```bash
cargo install cross
make deploy
```

`cross` handles the toolchain automatically via Docker — no manual linker setup needed.

### Option B — native toolchain (no Docker)

Install the target and linker:

```bash
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
```

The linker is already configured in `.cargo/config.toml`. Then:

```bash
make deploy
```

The Makefile detects whether `cross` is available and falls back to plain `cargo` automatically.

### Deploying the systemd service

The first time you set up the Pi, also push the service file:

```bash
make deploy-service
```

This copies `deploy/rioting.service` to `/etc/systemd/system/` and reloads the daemon. The service runs as the `ubuntu` user and restarts automatically on failure.
