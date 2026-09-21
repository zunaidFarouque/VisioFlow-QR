/// Safe, pure-standard-library URL percent decoding helper.
/// Decodes `%xx` hex sequences into UTF-8 characters while preserving literal `+` signs.
#[must_use]
pub fn percent_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut byte_iter = input.bytes();
    while let Some(b) = byte_iter.next() {
        if b == b'%' {
            let h1 = byte_iter.next();
            let h2 = byte_iter.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                let hex_str = [c1, c2];
                if let Ok(hex_utf8) = std::str::from_utf8(&hex_str) {
                    if let Ok(byte) = u8::from_str_radix(hex_utf8, 16) {
                        bytes.push(byte);
                        continue;
                    }
                }
                bytes.push(b'%');
                bytes.push(c1);
                bytes.push(c2);
            } else {
                bytes.push(b'%');
                if let Some(c) = h1 {
                    bytes.push(c);
                }
            }
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_standard_percent_escapes() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("user%40example.com"), "user@example.com");
        assert_eq!(percent_decode("GitHub%3Aoctocat"), "GitHub:octocat");
        assert_eq!(percent_decode("+1234567890"), "+1234567890");
    }

    #[test]
    fn handles_malformed_sequences_gracefully() {
        assert_eq!(percent_decode("foo%2"), "foo%2");
        assert_eq!(percent_decode("foo%zzbar"), "foo%zzbar");
        assert_eq!(percent_decode("%"), "%");
    }
}
