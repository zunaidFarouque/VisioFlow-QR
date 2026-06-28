# VisioFlow: Core Engine & Security Rules

## 1. The Parsing Engine & Variable Hierarchy
VisioFlow routes visual payloads into isolated environment variables. Cursor must strictly enforce this namespace hierarchy to prevent variable collisions in user scripts.

### The Three Namespaces:
1. `QR_RAW`: The complete, unedited string decoded from the capture. (e.g., `$env:QR_RAW`)
2. `QR_NATIVE_*`: Variables extracted via built-in, zero-config Rust parsers for standard protocols (e.g., `$env:QR_NATIVE_WIFI_SSID`, `$env:QR_NATIVE_URI_PORT`).
3. `QR_VAR_*`: Custom variables extracted via user-defined Regex Named Capture Groups. 
   * *Example:* If regex is `(?P<asset>\d+)`, it maps to `$env:QR_VAR_ASSET`.

### CRITICAL INJECTION RULE (Zero String Interpolation)
Under absolutely NO circumstances should the Rust engine modify the text of a user's script file to insert variables (no `str::replace`). Variables MUST be passed exclusively to the OS via the child process environment block (e.g., `Command::new().env("QR_VAR_ASSET", match_value)`). 

## 2. The Security Sandbox & Logging Doctrine
VisioFlow operates in high-security, air-gapped environments. The engine must default to absolute silence and safety.

* **Child Process Isolation:** Scripts triggered by VisioFlow must run in a detached child process. When the process dies, its environment block must be instantly reclaimed by the OS.
* **Sensitive Data Redaction (Anti-Leakage):** The background daemon's logging subscriber must NEVER log the actual values of `QR_NATIVE_WIFI_PASSWORD` or `OTP_SECRET` to standard output or log files. Implement a redaction layer that replaces sensitive payload values with `[REDACTED]` before writing to disk.
* **The Air-Gap Override:** If the `--disable-telemetry` flag is present, or if an environment variable `VISIOFLOW_AIRGAP=1` is detected on the host machine, the Rust engine must hard-panic and refuse to initialize any asynchronous network tracking (OTLP).

## 3. The Optical Pre-Processing Pipeline
Raw webcam data is dirty. Relying on default barcode decoders will result in high failure rates. Cursor must implement the following pipeline natively in Rust (using crates like `image` and `imageproc`):

1. **Down-sampling:** Scale high-resolution frames down to ~300 DPI equivalents to prevent CPU spiking.
2. **Median Blur (Noise Reduction):** Apply a fast median filter to remove webcam salt-and-pepper artifacts while preserving sharp barcode edges.
3. **Otsu's Binarization:** Do NOT use global static thresholding for contrast. Implement Otsu's method to mathematically calculate the optimal black/white threshold based on the frame's lighting variance.

## 4. Execution Halts & Interactive Routing
A single frame may contain multiple payloads (e.g., a shipping label with a QR code and a Code-128 barcode).

* **The TUI Halt:** If `capture --select` is passed and the array of decoded payloads has a `length > 1`, the engine MUST halt the execution pipeline. It must render an interactive terminal list allowing the user to select the correct payload before triggering the child script.
* **Cryptographic Pause:** If `capture --interactive` is passed, the engine must print the extracted payload to the terminal and wait for a `[y/N]` standard input confirmation before firing the OS action.