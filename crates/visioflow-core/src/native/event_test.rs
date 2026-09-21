use super::{EventParser, NativeParser};

#[test]
fn parses_standard_vevent() {
    let payload = "BEGIN:VEVENT\r\n\
SUMMARY:Team Sync Meeting\r\n\
DTSTART:20261001T090000Z\r\n\
DTEND:20261001T100000Z\r\n\
LOCATION:Conference Room B\r\n\
DESCRIPTION:Discuss Q4 roadmap and deliverables.\r\n\
END:VEVENT";

    let vars = EventParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_EVENT_SUMMARY").map(String::as_str), Some("Team Sync Meeting"));
    assert_eq!(vars.get("QR_NATIVE_EVENT_START").map(String::as_str), Some("20261001T090000Z"));
    assert_eq!(vars.get("QR_NATIVE_EVENT_END").map(String::as_str), Some("20261001T100000Z"));
    assert_eq!(vars.get("QR_NATIVE_EVENT_LOCATION").map(String::as_str), Some("Conference Room B"));
    assert_eq!(vars.get("QR_NATIVE_EVENT_DESCRIPTION").map(String::as_str), Some("Discuss Q4 roadmap and deliverables."));
}

#[test]
fn parses_vevent_inside_vcalendar_with_parameters() {
    let payload = "BEGIN:VCALENDAR\n\
VERSION:2.0\n\
BEGIN:VEVENT\n\
SUMMARY;LANGUAGE=en-US:Launch Party\n\
DTSTART;VALUE=DATE:20261231\n\
LOCATION:Main Hall\n\
END:VEVENT\n\
END:VCALENDAR";

    let vars = EventParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_EVENT_SUMMARY").map(String::as_str), Some("Launch Party"));
    assert_eq!(vars.get("QR_NATIVE_EVENT_START").map(String::as_str), Some("20261231"));
    assert_eq!(vars.get("QR_NATIVE_EVENT_LOCATION").map(String::as_str), Some("Main Hall"));
}

#[test]
fn handles_folded_lines_and_escaped_characters() {
    let payload = "BEGIN:VEVENT\nSUMMARY:Quarterly Strategy\n  Planning Session\nLOCATION:Room 101\\, Bldg 2\nDESCRIPTION:Line 1\\nLine 2\\; with semicolon\nEND:VEVENT";

    let vars = EventParser.parse(payload);

    assert_eq!(
        vars.get("QR_NATIVE_EVENT_SUMMARY").map(String::as_str),
        Some("Quarterly Strategy Planning Session")
    );
    assert_eq!(
        vars.get("QR_NATIVE_EVENT_LOCATION").map(String::as_str),
        Some("Room 101, Bldg 2")
    );
    assert_eq!(
        vars.get("QR_NATIVE_EVENT_DESCRIPTION").map(String::as_str),
        Some("Line 1\nLine 2; with semicolon")
    );
}

#[test]
fn returns_empty_for_non_event_payload() {
    assert!(EventParser.parse("BEGIN:VCARD\nFN:Alice\nEND:VCARD").is_empty());
    assert!(EventParser.parse("https://example.com").is_empty());
}
