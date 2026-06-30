use crate::rules::auto::RoutingEvent;

/// Human-readable stderr notification for a routing event.
#[must_use]
pub fn format_routing_message(event: &RoutingEvent) -> String {
    match event {
        RoutingEvent::Matched {
            rule,
            auto_route: true,
        } => format!(r#"visioflow: matched rule "{rule}""#),
        RoutingEvent::Matched {
            rule,
            auto_route: false,
        } => format!(r#"visioflow: rule "{rule}" applied"#),
        RoutingEvent::Mismatch { rule } => format!(
            r#"visioflow: rule "{rule}" did not match; copied payload to clipboard"#
        ),
        RoutingEvent::NoAutoMatch => {
            "visioflow: no auto rule matched; copied payload to clipboard".to_owned()
        }
        RoutingEvent::BuiltinCopy => "visioflow: copy-only mode".to_owned(),
        RoutingEvent::BuiltinPlain => "visioflow: plain stdout mode".to_owned(),
    }
}

/// Single-line JSON event for `--output json` tooling.
#[must_use]
pub fn format_routing_json_line(event: &RoutingEvent) -> String {
    match event {
        RoutingEvent::Matched { rule, .. } => {
            format!(r#"{{"event":"rule_matched","rule":"{rule}","fallback":false}}"#)
        }
        RoutingEvent::Mismatch { rule } => {
            format!(r#"{{"event":"rule_mismatch","rule":"{rule}","fallback":"copy"}}"#)
        }
        RoutingEvent::NoAutoMatch => r#"{"event":"no_auto_match","fallback":"copy"}"#.to_owned(),
        RoutingEvent::BuiltinCopy => r#"{"event":"builtin_copy","fallback":false}"#.to_owned(),
        RoutingEvent::BuiltinPlain => r#"{"event":"builtin_plain","fallback":false}"#.to_owned(),
    }
}
