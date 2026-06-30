# VisioFlow

Optical automation engine that captures QR payloads via webcam or screen snip, preprocesses frames natively in Rust, and routes decoded data to the desktop.

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## Windows install paths

Preferred order:

1. Scoop portable (`scripts/packaging/scoop/visioflow.json`)
2. Traditional install (`scripts/install-traditional.ps1`)
3. Zip/no-install bootstrap (`scripts/bootstrap-portable.ps1`)

Full commands and smoke checks are documented in `DOCs/USER_GUIDE.md`.

## Usage (MVP)

```bash
# Decode a QR code from a screen region selection
cargo run -p visioflow-cli -- capture --source snip --action stdout

# Decode a QR code from the default webcam (20s live preview window)
cargo run -p visioflow-cli -- capture --source webcam --action stdout

# Custom webcam scan timeout in seconds
cargo run -p visioflow-cli -- capture --source webcam --action stdout --timeout 30

# Apply median noise reduction before Otsu binarization (default: otsu pipeline)
cargo run -p visioflow-cli -- capture --source snip --filter median --action stdout
```

### Webcam manual exposure (sensor)

Focus the live preview window and use **↑ / ↓** to adjust **sensor exposure** in **0.5 EV** steps (↑ brighter, ↓ darker). This changes camera shutter/gain before capture — not a software filter on already-clipped pixels.

After each adjustment the app discards a few frames while the ISP settles, then shows the new sensor image.

```bash
cargo run -p visioflow-cli -- capture --source webcam --action stdout --verbose
```

For bright phone screens, press **↓** until QR modules are visible in the **preview itself**. If arrow keys have no effect, the driver may not expose manual exposure/gain.
