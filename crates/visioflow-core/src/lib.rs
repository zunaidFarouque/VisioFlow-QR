pub mod airgap;
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
pub mod share;
pub mod sys;
pub mod traits;

pub use airgap::{airgap_active, enforce_airgap_policy};
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
pub use share::{default_rules_asset_path, resolve_share_path};
pub use rules::{
    apply_rule, connect_wifi_from_vars, format_routing_json_line, format_routing_message,
    is_builtin_trigger, is_excluded_from_auto_scan, is_reserved_rule_name, merge_native_vars, resolve_payload_fully,
    route_payload, run_rule_actions, AutoRouteOptions, FileRuleStore, PayloadRouter, ResolvedVars,
    RoutedPayload, RouteMode, RouteResult, Rule, RuleEngine, RuleError, RuleResult, RuleStore,
    RoutingEvent, RESERVED_RULE_NAMES,
};
pub use sys::{platform_executor, SystemExecutor};
pub use traits::{
    BgrFrame, CnnQrDecoder, ExposureHal, FrameSource, LiveFrameSource, OpticalFilterKind,
    OpticalScanner, PayloadDecoder,
};
