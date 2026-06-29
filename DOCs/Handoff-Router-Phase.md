# VisioFlow — Handoff: Visual Payload Router Phase

> **Purpose:** Context for starting the next development chapter after the capture engine (snip + webcam) MVP. Attach this file to a new AI conversation when implementing rules, export, IPC, and daemon.

---

## 1. What this project is

**VisioFlow** is an optical automation engine and **visual payload router**. It captures QR payloads via webcam or screen snip, routes them through rules (regex, native parsers), maps data to ephemeral environment variables, and triggers OS actions.

It is **not** just a QR scanner.

### Authoritative docs (read first)

| Document | Governs |
|---|---|
| [`Architecture.md`](Architecture.md) | CLI noun-verb structure, TDD protocol, IPC requirement, daemon shape |
| [`ENGINE_RULES.md`](ENGINE_RULES.md) | `QR_RAW` / `QR_NATIVE_*` / `QR_VAR_*`, security sandbox, no script file mutation |
| [`Rust OpenCV QR Scanning Architecture.md`](Rust%20OpenCV%20QR%20Scanning%20Architecture.md) | Webcam path only (already implemented) |
| [`PLATFORM_CI.md`](PLATFORM_CI.md) | Cross-platform patterns, `interprocess` for IPC |

---

## 2. Repository layout

```
crates/visioflow-core/   # Engine: optical pipeline, decode, traits, opencv_webcam (feature-gated)
crates/visioflow-cli/    # Binary `visioflow`, capture UX, minifb preview
scripts/dev-env.ps1      # Windows: VCPKG_ROOT, LLVM PATH (required for webcam builds)
scripts/download-wechat-models.ps1
models/                  # WeChat CNN models (.caffemodel gitignored)
.github/workflows/build.yml
```

**Workspace:** Rust 2021, `mockall` in core dev-deps, CI on `windows-latest` + `ubuntu-latest`.

---

## 3. What works today (do not break)

| Feature | Status |
|---|---|
| `capture --source snip` | rqrr + Otsu/Median preprocessing |
| `capture --source webcam` | OpenCV + WeChat CNN, live minifb preview, exposure bracket guard |
| `--action stdout \| copy` | Working |
| `--output plain \| json` | Working |
| `--input-image` (hidden) | Integration test hook |
| Webcam tuning flags | `--preview-position`, `--preview-scale`, `--exposure-step-ms`, `--exposure-flush-grabs`, `--decode-interval-ms`, `--exposure-bracket auto\|on\|off` |
| Tests | `cargo test` — core optical/decode tests + `capture_stdout` integration |

### Webcam architecture (completed — treat as stable)

- **OpenCV path:** `crates/visioflow-core/src/opencv_webcam/` — `frame_stream` (spin-thread `grab()`), `wechat_decoder`, `exposure_hal`, `exposure_probe`, `bracket`
- **CLI preview:** `webcam_session.rs`, `webcam_preview.rs`, `preview_overlay.rs`, `screen_bounds.rs`, `decode_worker.rs` (background CNN decode thread)
- **Defaults:** exposure timing `100 / 2 / 100` ms (step / flush / decode interval)
- **Exposure guard:** startup luma probe + runtime plunge detection; overlay shows "Auto exposure only" when bracketing is off
- **Windows build requires** sourcing dev env before `cargo run`:

```powershell
. .\scripts\dev-env.ps1
cargo run --release -p visioflow-cli -- capture --source webcam --action stdout --verbose --timeout 30
```

`dev-env.ps1` sets `VCPKG_ROOT=D:\vcpkg`, `VCPKGRS_TRIPLET=x64-windows-static-md`, and adds LLVM to `PATH`.

---

## 4. What is stubbed / not built (this phase)

| Item | Current state |
|---|---|
| `--export bash\|ps1` | `main.rs` returns `UnsupportedAction` |
| `--ipc-socket` | Same stub |
| `rule` subcommand | Not in CLI (`create`, `config`, `execute`, `set-action`) |
| `daemon` subcommand | Not in CLI (`start`, `stop`, `status`, `reload`) |
| `capture --trigger`, `--select` | Not in CLI |
| Rule storage / regex routing | Not in core |
| IPC (named pipes / UDS) | Not started |
| Tauri headless daemon | Planned in Architecture; **not in repo yet** |

### Existing extension points

- `visioflow_core::sys::SystemExecutor` — platform trait (wifi stub only)
- `visioflow_core::traits` — `FrameSource`, `PayloadDecoder`, `CnnQrDecoder`, `LiveFrameSource`, `ExposureHal`, `OpticalScanner`
- `main.rs` — only `Commands::Capture` today; global flags `--export`, `--ipc-socket` parsed but rejected

---

## 5. Target CLI shape (from Architecture.md)

### Global flags

- `--output <plain|json>`
- `--verbose`, `--silent`
- `--export <bash|ps1>` — **critical** for parent shell variable injection
- `--ipc-socket <PATH>`

### `capture` (execution engine)

- `--source <snip|webcam>`, `--filter <otsu|median>`, `--action <stdout|copy>`
- `--trigger <RULE_NAME>` — not built
- `--select` — interactive TUI when multiple payloads; not built

### `rule` (automation manager) — not built

- `create <NAME>`
- `config <NAME> --regex <PAT>`
- `config <NAME> --map <G:VAR>`
- `execute <NAME> --payload <STR>`
- `set-action <NAME> --exec <PATH>`

### `daemon` (background service) — not built

- `start --hidden`, `stop`, `status`, `reload`

---

## 6. Non-negotiable constraints

1. **Env vars only via child process** — use `Command::new().env(...)`; **never** `str::replace` on user script files.
2. **Variable namespaces** (`ENGINE_RULES.md`):
   - `QR_RAW` — full decoded string
   - `QR_NATIVE_*` — built-in protocol parsers (WiFi, URI, etc.)
   - `QR_VAR_*` — regex named capture groups (e.g. `(?P<asset>\d+)` → `QR_VAR_ASSET`)
3. **`--export`** prints eval-compatible strings (`export ...` / `$env:...`) so the **parent shell** can source them.
4. **IPC:** CLI ↔ daemon via **local sockets** (Named Pipes on Windows, Unix domain sockets on Linux). No file polling.
5. **TDD:** traits → failing tests → minimal impl → refactor; `mockall` for OS/IPC mocks.
6. **Platforms:** Windows + Linux only; macOS out of scope.
7. **Snip/file capture stays native Rust** (`rqrr`, Otsu/Median) — do not pull OpenCV into static capture path.
8. **Sensitive logging:** never log `QR_NATIVE_WIFI_PASSWORD` etc.; redact as `[REDACTED]`.

---

## 7. Suggested implementation order

Agree on phasing in the first session. Recommended sequence:

| Phase | Deliverable |
|---|---|
| **A** | Rule model + persistence in `visioflow-core` (TOML/JSON under user config dir); unit tests for regex match + `QR_VAR_*` mapping |
| **B** | `rule` CLI — `create`, `config`, `set-action`, `execute` (local, no daemon) |
| **C** | `--export bash/ps1` — emit eval-safe env assignments from matched vars |
| **D** | IPC protocol (newline-delimited JSON); traits `IpcClient` / `IpcServer` |
| **E** | `daemon` subcommand — start/stop/status/reload; in-memory rules; CLI uses `--ipc-socket` |
| **F** | `capture --trigger RULE` and `capture --select` (TUI halt when multiple payloads) |

**Recommendation:** Phases **A → B → C** before daemon/IPC so routing logic is fully unit-testable without sockets.

**Open design choice:** Pure Rust daemon first vs Tauri headless (Architecture mentions Tauri). Start with Rust unless user specifies otherwise.

---

## 8. Development commands

```powershell
# Windows (webcam / OpenCV)
. .\scripts\dev-env.ps1
cargo test
cargo clippy -- -D warnings
cargo run --release -p visioflow-cli -- capture --source snip --action stdout

# Linux / CI
cargo test
# Ubuntu needs: libopencv-contrib-dev, clang, WeChat models in models/
```

Release binary (~19 MB standalone on Windows with static vcpkg triplet). WeChat `.caffemodel` files are runtime deps in `models/`.

---

## 9. User / maintainer notes

- **TDD** expected for all new features.
- **Do not auto-commit** unless explicitly asked.
- Built-in webcam may need `--exposure-bracket off` for stable auto exposure; runtime plunge guard usually handles this automatically.
- Large uncommitted working tree may exist on `main`; read `git status` before assuming clean state.

---

## 10. Prompt for the new AI session

Copy and adapt:

> Phase 1 of VisioFlow (capture: snip + webcam) is complete. Start the **visual payload router** chapter: implement rule management, regex routing, `QR_*` env mapping, `--export`, then IPC + `daemon` per `DOCs/Architecture.md` and `DOCs/ENGINE_RULES.md`. Follow TDD (traits → tests → impl). Do not refactor the webcam/OpenCV path unless strictly required. Read `DOCs/Handoff-Router-Phase.md` and propose a phased plan before coding.
