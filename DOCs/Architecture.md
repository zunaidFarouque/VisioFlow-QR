# VisioFlow: Architecture & TDD Master Context

## 1. Project Overview
VisioFlow is an "Optical Automation Engine" and "Visual Payload Router." It bridges the gap between physical visual data (QR/barcodes) and desktop environments. It allows sysadmins and developers to capture payloads via webcam or screen-snip, parse them, map the data to ephemeral environment variables, and trigger native OS executions.

## 2. Technology Stack & Constraints
* **Backend:** Rust (Native, Zero-bloat).
* **Frontend/Daemon:** Tauri (Headless mode for the background daemon).
* **Cross-Platform:** Windows and Linux (macOS is explicitly out of scope).
* **Strict Constraints:**
  * **Zero Bloat:** Do not use heavy C++ bindings or massive frameworks like OpenCV unless absolutely necessary. Rely on native Rust crates (e.g., pure Rust ports for QR decoding, native image processing crates).
  * **IPC:** The CLI and the background Daemon MUST communicate via Local Sockets (Unix Domain Sockets on Linux, Named Pipes on Windows). Do not use file polling.
  * **Optical Pre-processing:** Implement Otsu's threshold method natively in Rust before passing webcam frames to the decoder.

## 3. CLI Architecture (The "Noun-Verb" Paradigm)
The CLI must follow this exact structure using the `clap` crate:
* **Global Flags:** `--output <plain|json>`, `--verbose`, `--silent`, `--export <bash|ps1>` (CRITICAL for parent shell evaluation), `--ipc-socket <PATH>`.
* **`capture` (The Execution Engine):** * `--source <snip|webcam>`, `--filter <otsu|median>`, `--action <stdout|copy>`, `--trigger <RULE_NAME>`, `--select` (Interactive TUI for multiple payloads).
* **`rule` (The Automation Manager):**
  * `create <NAME>`, `config <NAME> --regex <PAT>`, `config <NAME> --map <G:VAR>`, `execute <NAME> --payload <STR>`, `set-action <NAME> --exec <PATH>`.
* **`daemon` (The Background Service):**
  * `start --hidden`, `stop`, `status`, `reload`.

## 4. Execution Sandbox Rules
* Environment variables populated from regex capture groups must strictly live in the child process execution block (e.g., `std::process::Command::new().env()`). They must never persist globally.
* Parent shell injection is handled EXCLUSIVELY via the `--export` flag outputting eval-compatible strings.

---

## 5. Cursor AI: TDD Engineering Protocol
You are acting as an elite Rust Systems Engineer. You will follow a strict Test-Driven Development (TDD) workflow for every feature.

### Step 1: Interface & Trait Definition
Before writing implementation code, define the abstract traits. Because this is a system-level tool, you MUST abstract the OS layer so we can write unit tests on Windows without breaking Linux.
Example: Create an `OsCommandRunner` trait rather than hardcoding `std::process::Command` directly in the logic.

### Step 2: The Red Phase (Write Tests)
Write the unit or integration tests FIRST. 
* Use the `mockall` crate to mock OS interactions (camera access, clipboard, IPC sockets).
* Assert exact expected outputs (e.g., test that Otsu's threshold mathematically returns the correct binarized matrix for a mock image array).
* Do not proceed until `cargo test` explicitly fails for the right reasons.

### Step 3: The Green Phase (Implementation)
Write the minimum viable Rust code to pass the test. Prioritize memory safety and explicit error handling (`Result<T, E>`). Never use `.unwrap()` in production logic.

### Step 4: The Refactor Phase
Optimize the code for zero-bloat. Ensure conditional compilation flags (`#[cfg(target_os = "windows")]`) are perfectly placed.