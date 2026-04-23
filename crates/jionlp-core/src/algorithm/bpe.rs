//! Byte-level BPE encoder/decoder — port of
//! `jionlp/algorithm/bpe/encoder_decoder.py`.
//!
//! Maps each byte of UTF-8 text to a *printable* Unicode code point and
//! back. This isn't a full BPE tokenizer (no merge table, no vocab); it's
//! the byte-layer building block that GPT-style BPE layers sit on.
//!
//! Why this mapping exists: raw bytes 0x00-0x1F / 0x7F etc. are control
//! chars that cause trouble in BPE training pipelines. The standard fix
//! (from GPT-2 / HuggingFace) shifts them into a safe range of U+0100+
//! Latin code points that are both printable and outside the ASCII
//! whitespace hot zones.
//!
//! Usage:
//! ```ignore
//! let encoded = jionlp_core::bpe_encode("メトロ");
//! let decoded = jionlp_core::bpe_decode(&encoded);
//! assert_eq!(decoded, "メトロ");
//! ```
//!
//! `bpe_decode` is robust: when the input has stray bytes that don't form
//! valid UTF-8, the offending code point is replaced with U+FFFD (`�`)
//! and decoding continues — mirroring the Python behavior.

use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;

/// Lazy static tables for the 256-byte printable-Unicode mapping.
struct ByteTables {
    enc: [char; 256],
    dec: FxHashMap<char, u8>,
}

static TABLES: Lazy<ByteTables> = Lazy::new(build_tables);

fn build_tables() -> ByteTables {
    // Printable ASCII range + the two printable Latin-1 Supplement ranges
    // used by GPT-2's tokenizer. Byte values OUTSIDE this set get remapped
    // to U+0100 + n.
    let printable: Vec<u8> = (b'!'..=b'~')
        .chain(0xA1..=0xAC)
        .chain(0xAE..=0xFF)
        .collect();
    let printable_set: std::collections::HashSet<u8> = printable.iter().copied().collect();

    let mut enc: [char; 256] = ['\0'; 256];
    // First pass: printable bytes map to themselves (as the corresponding
    // code point).
    for &b in &printable {
        enc[b as usize] = char::from_u32(b as u32).unwrap();
    }
    // Second pass: non-printable bytes map to U+0100 + n where n counts up.
    let mut n: u32 = 0;
    for b in 0u8..=255 {
        if !printable_set.contains(&b) {
            enc[b as usize] = char::from_u32(256 + n).unwrap();
            n += 1;
        }
    }
    let dec: FxHashMap<char, u8> = enc.iter().enumerate().map(|(i, &c)| (c, i as u8)).collect();
    ByteTables { enc, dec }
}

/// Encode a string to its byte-level BPE representation. Each UTF-8 byte
/// is replaced by its mapped printable Unicode code point.
pub fn bpe_encode(text: &str) -> String {
    let t = &*TABLES;
    let mut out = String::with_capacity(text.len() * 2);
    for &b in text.as_bytes() {
        out.push(t.enc[b as usize]);
    }
    out
}

/// Inverse of [`bpe_encode`]. Gracefully handles invalid / stray code
/// points by emitting U+FFFD and continuing.
pub fn bpe_decode(encoded: &str) -> String {
    let t = &*TABLES;
    let mut bytes: Vec<u8> = Vec::with_capacity(encoded.len());
    let mut iter = encoded.chars();

    while let Some(c) = iter.next() {
        match t.dec.get(&c) {
            Some(&b) => bytes.push(b),
            None => {
                // Unknown code point — flush what we have as UTF-8, emit
                // REPLACEMENT CHARACTER, continue.
                let flushed = String::from_utf8_lossy(&bytes).to_string();
                let mut combined = flushed;
                combined.push('\u{FFFD}');
                // Restart byte collection from remaining chars.
                bytes.clear();
                let mut rest = String::new();
                for next_c in iter.by_ref() {
                    match t.dec.get(&next_c) {
                        Some(&b) => bytes.push(b),
                        None => rest.push('\u{FFFD}'),
                    }
                }
                combined.push_str(&String::from_utf8_lossy(&bytes));
                combined.push_str(&rest);
                return combined;
            }
        }
    }

    // Fast path: all bytes collected, UTF-8 decode them.
    match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(&bytes).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_japanese() {
        let src = "メトロ";
        let enc = bpe_encode(src);
        let dec = bpe_decode(&enc);
        assert_eq!(dec, src);
    }

    #[test]
    fn roundtrip_chinese() {
        let src = "今天天气真好";
        assert_eq!(bpe_decode(&bpe_encode(src)), src);
    }

    #[test]
    fn roundtrip_ascii_printable_preserved() {
        // Printable ASCII (0x21..=0x7e) maps to itself — space is
        // *deliberately* NOT in that set (see GPT-2 tokenizer notes).
        let src = "hello!";
        assert_eq!(bpe_encode(src), src);
        assert_eq!(bpe_decode(&bpe_encode(src)), src);
    }

    #[test]
    fn ascii_space_remapped_to_g_with_dot() {
        // Space (0x20) remaps to Ġ (U+0120) in GPT-2's byte-level scheme.
        let enc = bpe_encode(" ");
        assert_eq!(enc, "Ġ");
        assert_eq!(bpe_decode("Ġ"), " ");
    }

    #[test]
    fn control_bytes_remapped() {
        // Tab is 0x09, remapped to U+0100 + n.
        let enc = bpe_encode("\t");
        let c = enc.chars().next().unwrap() as u32;
        assert!(c >= 0x100, "tab should remap to U+0100+, got {c:#x}");
        assert_eq!(bpe_decode(&enc), "\t");
    }

    #[test]
    fn multiple_scripts_roundtrip() {
        let src = "Hello 世界 🌍 テスト";
        assert_eq!(bpe_decode(&bpe_encode(src)), src);
    }

    #[test]
    fn empty_text() {
        assert_eq!(bpe_encode(""), "");
        assert_eq!(bpe_decode(""), "");
    }

    #[test]
    fn encoded_length_matches_utf8_length() {
        // Each byte maps to exactly one char.
        let src = "中国";
        assert_eq!(bpe_encode(src).chars().count(), src.as_bytes().len());
    }
}
