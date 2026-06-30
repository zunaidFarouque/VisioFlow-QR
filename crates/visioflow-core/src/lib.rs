pub mod capture;
pub mod decode;
pub mod error;
pub mod export;
pub mod ipc;
pub mod logging;
pub mod native;
pub mod opencv_webcam;
pub mod optical;
pub mod rules;
pub mod sys;
pub mod traits;

pub use capture::{
    decode_captured_frame, decode_captured_frame_live, decode_captured_frame_live_with_profile,
    CaptureEngine,
};
pub use decode::MANUAL_EV_STEP;
pub use error::VisioFlowError;
pub use export::{emit_bash, emit_ps1, vars_from_payloads, vars_from_resolved};
pub use ipc::{
    default_socket_path, parse_socket_name, ClientMessage, DaemonHandler, IpcClient, IpcServer,
    ServerMessage, SocketIpcClient, SocketIpcServer, DEFAULT_SOCKET_NAME,
};
pub use logging::{format_log_line, is_sensitive_key, redact_env_map, redact_sensitive, REDACTED};
pub use optical::{apply_ev_adjustment_f32, preprocess_frame, MAX_FRAME_WIDTH};
pub use rules::{
    apply_rule, merge_native_vars, resolve_payload_fully, FileRuleStore, PayloadRouter,
    ResolvedVars, RoutedPayload, Rule, RuleEngine, RuleError, RuleResult, RuleStore,
};
pub use traits::{
    BgrFrame, CnnQrDecoder, ExposureHal, FrameSource, LiveFrameSource, OpticalFilterKind,
    OpticalScanner, PayloadDecoder,
};
