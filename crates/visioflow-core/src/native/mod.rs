mod keys;
mod uri;
mod wifi;

pub use keys::{is_sensitive_native_key, SENSITIVE_NATIVE_KEYS};
pub use uri::UriParser;
pub use wifi::WifiParser;

use std::collections::HashMap;

/// Built-in protocol parser that maps a raw QR payload to `QR_NATIVE_*` env keys.
pub trait NativeParser {
    fn parse(&self, raw: &str) -> HashMap<String, String>;
}

#[cfg(test)]
mod keys_test;
#[cfg(test)]
mod wifi_test;
#[cfg(test)]
mod uri_test;
