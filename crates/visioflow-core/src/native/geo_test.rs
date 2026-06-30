use std::collections::HashMap;

use super::{GeoParser, NativeParser};

fn geo_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn parses_geo_coordinates() {
    let parser = GeoParser;
    let raw = "geo:48.85,2.35";

    let vars = parser.parse(raw);

    assert_eq!(
        vars,
        geo_map(&[
            ("QR_NATIVE_GEO_LAT", "48.85"),
            ("QR_NATIVE_GEO_LON", "2.35"),
        ])
    );
}

#[test]
fn parses_geo_case_insensitive_prefix() {
    let parser = GeoParser;

    let vars = parser.parse("GEO:-33.86,151.21");

    assert_eq!(
        vars.get("QR_NATIVE_GEO_LAT").map(String::as_str),
        Some("-33.86")
    );
    assert_eq!(
        vars.get("QR_NATIVE_GEO_LON").map(String::as_str),
        Some("151.21")
    );
}

#[test]
fn returns_empty_for_invalid_geo_payload() {
    let parser = GeoParser;

    assert!(parser.parse("geo:48.85").is_empty());
    assert!(parser.parse("tel:+15551234").is_empty());
}

#[test]
fn parses_geo_with_query_suffix() {
    let parser = GeoParser;
    let vars = parser.parse("geo:23.72427395,90.39364864?q=come here!");

    assert_eq!(
        vars.get("QR_NATIVE_GEO_LAT").map(String::as_str),
        Some("23.72427395")
    );
    assert_eq!(
        vars.get("QR_NATIVE_GEO_LON").map(String::as_str),
        Some("90.39364864")
    );
}

#[test]
fn parses_geo_with_parameter_suffix() {
    let parser = GeoParser;
    let vars = parser.parse("geo:37.786971,-122.399677;u=35");

    assert_eq!(
        vars.get("QR_NATIVE_GEO_LAT").map(String::as_str),
        Some("37.786971")
    );
    assert_eq!(
        vars.get("QR_NATIVE_GEO_LON").map(String::as_str),
        Some("-122.399677")
    );
}
