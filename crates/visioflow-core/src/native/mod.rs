mod geo;
mod keys;
mod mailto;
mod tel;
mod uri;
mod vcard;
mod wifi;

pub use geo::GeoParser;
pub use keys::{is_sensitive_native_key, SENSITIVE_NATIVE_KEYS};
pub use mailto::MailtoParser;
pub use tel::TelParser;
pub use uri::UriParser;
pub use vcard::VcardParser;
pub use wifi::WifiParser;

use std::collections::HashMap;

/// Built-in protocol parser that maps a raw QR payload to `QR_NATIVE_*` env keys.
pub trait NativeParser {
    fn parse(&self, raw: &str) -> HashMap<String, String>;
}

#[cfg(test)]
mod geo_test;
#[cfg(test)]
mod keys_test;
#[cfg(test)]
mod mailto_test;
#[cfg(test)]
mod tel_test;
#[cfg(test)]
mod uri_test;
#[cfg(test)]
mod vcard_test;
#[cfg(test)]
mod wifi_test;
