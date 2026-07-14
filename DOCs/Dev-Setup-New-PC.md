# Development setup on a new PC

Use this guide when moving VisioFlow **development** to a fresh Windows machine. It covers cloning the repo, installing the Rust/OpenCV toolchain, verifying builds, and optional migration from an old dev PC.

For **end-user install** (Scoop, shortcuts, first scan), see [Getting Started](Getting-Started.md). You can keep using a Scoop install on one PC while developing on another — they are independent.

---

## What you need (Windows)

| Tier | Goal | Required tools |
|------|------|----------------|
| **Router-only** | Snip, rules, daemon, tests, CI parity | Rust (stable + clippy), Git, MSVC build tools |
| **Full build** | Webcam / OpenCV + release zip | Everything above + LLVM + vcpkg |

Linux router-only setup is summarized at the end.

---

## 1. Base tools

### Git

Install [Git for Windows](https://git-scm.com/download/win) and configure your identity:

```powershell
git config --global user.name "Your Name"
git config --global user.email "you@example.com"
```

### Rust

Install from [rustup.rs](https://rustup.rs/), then add clippy:

```powershell
rustup default stable
rustup component add clippy
rustc --version
cargo --version
```

### MSVC build tools

Rust on Windows links with the Microsoft C++ toolchain. Install **Visual Studio Build Tools** (or full Visual Studio) with the **Desktop development with C++** workload. Without this, `cargo build` fails with linker errors.

---

## 2. Clone the repository

```powershell
git clone https://github.com/zunaidFarouque/VisioFlow-QR.git
cd VisioFlow-QR
```

Use your fork URL if you develop via a fork. Confirm you are on the branch you expect:

```powershell
git status
git pull
```

---

## 3. Router-only dev (fast path)

This matches CI ([`.github/workflows/build.yml`](../.github/workflows/build.yml)): no OpenCV, no webcam.

```powershell
# Lint
cargo clippy --workspace --no-default-features -- -D warnings

# Test
cargo test --workspace --no-default-features

# Debug binary
cargo build -p visioflow-cli --no-default-features
```

Smoke check:

```powershell
.\scripts\smoke-router.ps1
.\scripts\smoke-default-rules.ps1
```

Run snip capture from the debug build:

```powershell
.\target\debug\visioflow.exe capture --source snip
```

---

## 4. Full Windows build (webcam + OpenCV)

Full builds need **LLVM** and **vcpkg**. The repo assumes defaults in `scripts/dev-env.ps1`:

| Variable / path | Default in repo |
|-----------------|-----------------|
| LLVM | `C:\Program Files\LLVM\bin` |
| vcpkg root | `D:\vcpkg` |
| Triplet | `x64-windows-static-md` |

### 4.1 Install LLVM

Install [LLVM for Windows](https://releases.llvm.org/) (or `winget install LLVM.LLVM`). Confirm:

```powershell
clang --version
```

If LLVM lives elsewhere, edit `scripts/dev-env.ps1` on your machine (local change — do not commit machine-specific paths unless the team agrees on a new default).

### 4.2 Install vcpkg

Pick a permanent location (example `D:\vcpkg`):

```powershell
git clone https://github.com/microsoft/vcpkg D:\vcpkg
D:\vcpkg\bootstrap-vcpkg.bat
```

The first **full** `cargo build` triggers vcpkg to fetch and compile OpenCV (contrib + `wechat_qrcode`). Expect a long first build and several GB under `D:\vcpkg\`.

If your vcpkg root is not `D:\vcpkg`, either:

- Pass `-VcpkgRoot` to `scripts/build-release.ps1`, or
- Set `$vcpkgRoot` at the top of `scripts/dev-env.ps1` locally.

### 4.3 Load dev environment (each new terminal)

```powershell
cd VisioFlow-QR
. .\scripts\dev-env.ps1
```

You should see `VCPKG_ROOT`, `VCPKGRS_TRIPLET`, and `LLVM in PATH = yes`.

### 4.4 Download WeChat CNN models (dev)

Release zips bundle models automatically. For local dev:

```powershell
.\scripts\download-wechat-models.ps1
```

Files land in `models/` (see [models/README.md](../models/README.md)).

### 4.5 Build and test webcam

```powershell
. .\scripts\dev-env.ps1
cargo build --release -p visioflow-cli
.\target\release\visioflow.exe capture --source webcam --timeout 15
```

Release packaging:

```powershell
. .\scripts\dev-env.ps1
.\scripts\build-release.ps1
```

Output: `dist/visioflow-win-x64/` and `dist/visioflow-win-x64.zip`. See [Distribution-Windows.md](Distribution-Windows.md) for publish steps.

---

## 5. Verify everything

Run the full smoke suite before your first commit on the new machine:

```powershell
.\scripts\smoke-router.ps1
.\scripts\smoke-default-rules.ps1
.\scripts\smoke-shortcuts.ps1
.\scripts\smoke-distribution.ps1
```

Optional notification check:

```powershell
.\scripts\smoke-notify.ps1
```

---

## 6. IDE (Cursor / VS Code)

Recommended extensions:

- **rust-analyzer** — completions, go-to-definition, inline errors
- **CodeLLDB** or **Native Debug** — optional native debugging

Open the repo root (where the workspace `Cargo.toml` lives). rust-analyzer picks up the workspace automatically.

### TDD workflow

Follow [Architecture.md](Architecture.md) § TDD: write a failing test first, implement, refactor. Router-only tests run without `dev-env.ps1`:

```powershell
cargo test -p visioflow-core
cargo test -p visioflow-cli --no-default-features
```

---

## 7. Migrating from an old dev PC

### Git / source

Ensure all work is pushed from the old machine:

```powershell
git push origin main
```

On the new PC, clone fresh (section 2). Do not copy `target/` — it is huge and not portable.

### User config (optional)

If you customized routing rules and want them on the new PC **for daily use** (Scoop or install):

| Item | Windows path |
|------|----------------|
| Rules store | `%APPDATA%\visioflow\rules.json` |
| Daemon PID | `%APPDATA%\visioflow\daemon.pid` |

Copy `rules.json` to the same path on the new PC, or re-run `visioflow rule init-defaults` and re-apply edits.

Scoop persist (if using Scoop on the new PC): `~\scoop\persist\visioflow\rules.json`.

### Machine-specific paths

Update locally (do not commit unless shared):

- `scripts/dev-env.ps1` — LLVM and vcpkg paths
- Hotkeys / PowerToys bindings pointing at old `.vbs` launcher paths

### Old PC after migration

You can keep using VisioFlow via Scoop on the old PC without the dev toolchain. Safe cleanup on the old machine (dev only):

- Delete repo `target/` to reclaim disk space
- Remove `D:\vcpkg` if you will not build there again
- Keep Rust/LLVM if other projects use them

---

## 8. Linux (router-only)

For snip + rules + daemon without webcam:

```bash
# Debian/Ubuntu example
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup component add clippy

git clone https://github.com/zunaidFarouque/VisioFlow-QR.git
cd VisioFlow-QR

cargo clippy --workspace --no-default-features -- -D warnings
cargo test --workspace --no-default-features
cargo build --release -p visioflow-cli --no-default-features
```

Webcam/OpenCV on Linux is not the primary dev path; see [Getting-Started.md](Getting-Started.md) prerequisites for distro packages if you experiment locally.

---

## 9. Troubleshooting

| Symptom | Likely fix |
|---------|------------|
| `link.exe` not found | Install Visual Studio Build Tools (C++ workload) |
| `clang` not found during OpenCV build | Install LLVM; run `. .\scripts\dev-env.ps1` |
| vcpkg / OpenCV build fails | Confirm `VCPKG_ROOT` points to bootstrapped vcpkg; check disk space (~30 GB for vcpkg + OpenCV) |
| Webcam decode fails at runtime | Run `.\scripts\download-wechat-models.ps1`; or set `VISIOFLOW_MODELS_DIR` to a folder with the four model files |
| `dev-env.ps1` warnings | Adjust `$llvmBin` / `$vcpkgRoot` in that script for your machine |
| Tests pass locally but differ on CI | CI uses `--no-default-features` only; run the same flags before pushing |

---

## Related docs

| Document | When to read |
|----------|--------------|
| [Getting Started](Getting-Started.md) | Install VisioFlow as a user, shortcuts, first scan |
| [Architecture](Architecture.md) | TDD protocol, CLI design |
| [PLATFORM_CI.md](PLATFORM_CI.md) | What CI runs vs local full builds |
| [Distribution-Windows.md](Distribution-Windows.md) | Release zip and Scoop publish |
| [Capture](Capture.md) | Snip/webcam flags and models |
