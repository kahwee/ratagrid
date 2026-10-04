//! Experimental ASCII fast path using nightly portable SIMD.
#![feature(portable_simd)]

#[path = "../../../src/text.rs"]
mod scalar;

use std::{
    borrow::Cow,
    hint::black_box,
    simd::{Simd, cmp::SimdPartialOrd},
    time::{Duration, Instant},
};

fn sanitize_simd(text: &str) -> Cow<'_, str> {
    let (chunks, remainder) = text.as_bytes().as_chunks::<32>();
    for chunk in chunks {
        let bytes = Simd::<u8, 32>::from_array(*chunk);
        // ASCII printable bytes are U+0020..U+007E. Fall back for any control
        // or non-ASCII byte so Unicode and bidi policy stays in the real scalar code.
        if (bytes.simd_lt(Simd::splat(0x20)) | bytes.simd_ge(Simd::splat(0x7f))).any() {
            return scalar::sanitize(text);
        }
    }
    if remainder.iter().any(|&b| !(0x20..0x7f).contains(&b)) {
        return scalar::sanitize(text);
    }
    Cow::Borrowed(text)
}

fn measure(input: &str, iterations: usize, sanitizer: fn(&str) -> Cow<'_, str>) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(sanitizer(black_box(input)));
    }
    start.elapsed()
}

fn main() {
    println!("Nightly portable SIMD sanitization experiment; scalar is the production policy.");
    println!("case,bytes,iterations,scalar_ns_per_call,simd_ns_per_call,scalar_over_simd");
    for (name, input) in [
        ("ascii_short", "Job 42 owned by Ada".to_owned()),
        ("ascii_256", "a".repeat(256)),
        ("ascii_4096", "a".repeat(4096)),
        ("unicode", "東京 cafe\u{301} العربية עברית 👩🏽‍💻".repeat(8)),
        ("bidi", format!("{}\u{202e}spoof\u{2069}", "a".repeat(256))),
        ("terminal_controls", "a\t\n\x1b[31mb".repeat(32)),
    ] {
        assert_eq!(sanitize_simd(&input), scalar::sanitize(&input));
        // Warm both paths before timing. No performance assertions: CPU matters.
        let _ = measure(&input, 1_000, scalar::sanitize);
        let _ = measure(&input, 1_000, sanitize_simd);
        let iterations = 50_000;
        let scalar = measure(&input, iterations, scalar::sanitize).as_nanos() as f64;
        let simd = measure(&input, iterations, sanitize_simd).as_nanos() as f64;
        println!(
            "{name},{},{iterations},{:.1},{:.1},{:.2}",
            input.len(),
            scalar / iterations as f64,
            simd / iterations as f64,
            scalar / simd
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_simd_boundaries_and_ascii_bytes_match_scalar() {
        for length in [0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129] {
            let clean = "a".repeat(length);
            assert!(matches!(sanitize_simd(&clean), Cow::Borrowed(_)));
            for byte in 0..=127 {
                for position in [0, length / 2, length] {
                    let mut input = clean.clone();
                    input.insert(position, char::from(byte));
                    assert_eq!(sanitize_simd(&input), scalar::sanitize(&input));
                }
            }
        }
    }

    #[test]
    fn unicode_and_bidi_controls_match_the_production_policy() {
        for codepoint in (0..=0x2fff).chain([0xfe0f, 0x1f469, 0x1f3fd, 0x1f4bb, 0x10ffff]) {
            if let Some(c) = char::from_u32(codepoint) {
                let input = format!("{}e{c}\u{301} العربية 👩🏽‍💻", "a".repeat(31));
                assert_eq!(sanitize_simd(&input), scalar::sanitize(&input));
            }
        }
    }

    #[test]
    fn mixed_untrusted_text_matches_scalar_without_mutating_input() {
        let alphabet = [
            'a', ' ', '\0', '\t', '\n', '\x1b', '\u{7f}', '\u{85}', '\u{061c}', '\u{200e}',
            '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}',
            '\u{2067}', '\u{2068}', '\u{2069}', '東', '\u{301}', '\u{200d}', '\u{fe0f}', '👩',
        ];
        let mut seed = 0x9942u64;
        for length in 0..256 {
            let mut input = String::new();
            for _ in 0..length {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                input.push(alphabet[(seed >> 32) as usize % alphabet.len()]);
            }
            let original = input.clone();
            assert_eq!(sanitize_simd(&input), scalar::sanitize(&input));
            assert_eq!(input, original);
        }
    }
}
