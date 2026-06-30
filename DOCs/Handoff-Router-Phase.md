# VisioFlow — Handoff: Visual Payload Router Phase

> **Purpose:** Handoff for continuing VisioFlow after router phase (rules, export, IPC, daemon, capture halts). Attach when implementing WiFi `SystemExecutor`, Tauri, or extra parsers.

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
| [`USER_GUIDE.md`](USER_GUIDE.md) | End-user kickstart: install, rules, export, daemon, troubleshooting |

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
| Tests | `cargo test` — core + `capture_stdout`, `capture_trigger`, `rule_execute` |

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

## 4. Router phase status

### Done (do not regress)

| Item | Location |
|---|---|
| Rule model + JSON store + regex / `QR_VAR_*` | `visioflow-core/src/rules/` |
| `rule` CLI (`create`, `config`, `set-action`, `execute`) | `visioflow-cli/src/commands/rule.rs` |
| `--export bash\|ps1` | `visioflow-core/src/export/`, wired in `main.rs` |
| IPC (named pipes / UDS) + protocol | `visioflow-core/src/ipc/`, `DOCs/IPC_PROTOCOL.md` |
| `daemon` CLI (`start`, `stop`, `status`, `reload`) | `visioflow-cli/src/commands/daemon.rs` |
| `capture --trigger RULE` | `capture.rs` + IPC/local routing |
| `capture --select` / `--interactive` | Multi-payload TUI + `[y/N]` confirm (`capture.rs`) |
| Native parsers (URI, WiFi, mailto, tel, geo, vCard) | `visioflow-core/src/native/` |
| Sensitive logging redaction | `visioflow-core/src/logging/` |
| Air-gap startup guard | `visioflow-core/src/airgap.rs`, `--disable-telemetry` |
| `rule execute --no-exec` + full native merge | `commands/rule.rs`, `commands/exec.rs` |

### Not built yet

| Item | Notes |
|---|---|
| Tauri headless daemon | Deferred; pure-Rust daemon in repo |
| WiFi `SystemExecutor` | `connect_wifi` stub in `sys/` |
| OTLP / network telemetry | Air-gap hook exists; no telemetry crate yet |

### Extension points

- `visioflow_core::sys::SystemExecutor` — platform trait (wifi action stub)
- `visioflow_core::traits` — optical / decode traits (stable)
- Global `--ipc-socket` / `VISIOFLOW_IPC_SOCKET` — CLI ↔ daemon routing

---

## 5. Target CLI shape (from Architecture.md)

### Global flags

- `--output <plain|json>`
- `--verbose`, `--silent`
- `--export <bash|ps1>` — **critical** for parent shell variable injection
- `--ipc-socket <PATH>`

### `capture` (execution engine)

- `--source <snip|webcam>`, `--filter <otsu|median>`, `--action <stdout|copy>`
- `--trigger <RULE_NAME>` — **done**
- `--select` — interactive TUI when multiple payloads — **done**
- `--interactive` — `[y/N]` confirm — **done**

### `rule` (automation manager) — **done**

- `create <NAME>`, `config <NAME> --regex`, `config <NAME> --map`, `execute <NAME> --payload`, `set-action <NAME> --exec`

### `daemon` (background service) — **done**

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

## 7. Suggested next work

| Priority | Deliverable |
|---|---|
| **1** | WiFi `SystemExecutor::connect_wifi` (Windows + Linux) |
| **2** | Rule actions that invoke `SystemExecutor` (e.g. auto-connect WiFi QR) |
| **Later** | Tauri headless daemon (optional; Rust daemon already works) |
| **Later** | OTLP telemetry with air-gap-aware init |

Phases A–F (rules, export, IPC, daemon, `--trigger`, `--select`, `--interactive`) are complete.

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

> Router phase is complete (rules, export, IPC, daemon, capture halts, native parsers, air-gap). Next: WiFi `SystemExecutor::connect_wifi` and rule actions that use it. Read `DOCs/Handoff-Router-Phase.md`, `DOCs/USER_GUIDE.md`. Follow TDD; do not refactor webcam/OpenCV unless required. Tauri daemon is deferred.
