//! Shared plain-text policy for terminal output, search and clipboard payloads.
use std::borrow::Cow;

pub(crate) fn is_unsafe(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

pub(crate) fn sanitize(text: &str) -> Cow<'_, str> {
    if text.chars().any(is_unsafe) {
        Cow::Owned(text.chars().filter(|&c| !is_unsafe(c)).collect())
    } else {
        Cow::Borrowed(text)
    }
}
