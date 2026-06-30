use crate::rules::{format_routing_json_line, format_routing_message, RoutingEvent};

#[test]
fn format_auto_matched_message() {
    let msg = format_routing_message(&RoutingEvent::Matched {
        rule: "url".to_owned(),
        auto_route: true,
    });
    assert_eq!(msg, r#"visioflow: matched rule "url""#);
}

#[test]
fn format_explicit_matched_message() {
    let msg = format_routing_message(&RoutingEvent::Matched {
        rule: "asset".to_owned(),
        auto_route: false,
    });
    assert_eq!(msg, r#"visioflow: rule "asset" applied"#);
}

#[test]
fn format_explicit_mismatch_message() {
    let msg = format_routing_message(&RoutingEvent::Mismatch {
        rule: "asset".to_owned(),
    });
    assert_eq!(
        msg,
        r#"visioflow: rule "asset" did not match; copied payload to clipboard"#
    );
}

#[test]
fn format_no_auto_match_message() {
    let msg = format_routing_message(&RoutingEvent::NoAutoMatch);
    assert_eq!(
        msg,
        "visioflow: no auto rule matched; copied payload to clipboard"
    );
}

#[test]
fn format_builtin_copy_message() {
    let msg = format_routing_message(&RoutingEvent::BuiltinCopy);
    assert_eq!(msg, "visioflow: copy-only mode");
}

#[test]
fn format_routing_json_line_matched() {
    let line = format_routing_json_line(&RoutingEvent::Matched {
        rule: "url".to_owned(),
        auto_route: true,
    });
    assert_eq!(
        line,
        r#"{"event":"rule_matched","rule":"url","fallback":false}"#
    );
}

#[test]
fn format_routing_json_line_mismatch() {
    let line = format_routing_json_line(&RoutingEvent::Mismatch {
        rule: "asset".to_owned(),
    });
    assert_eq!(
        line,
        r#"{"event":"rule_mismatch","rule":"asset","fallback":"copy"}"#
    );
}

#[test]
fn format_routing_json_line_no_auto_match() {
    let line = format_routing_json_line(&RoutingEvent::NoAutoMatch);
    assert_eq!(line, r#"{"event":"no_auto_match","fallback":"copy"}"#);
}
