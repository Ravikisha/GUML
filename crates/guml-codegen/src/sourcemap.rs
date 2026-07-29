//! Source Map v3 for emitted code.
//!
//! # Why this is not optional
//!
//! The compiler expands 24 lines of GUML into 160 lines of TSX. Without a mapping, every stack
//! trace, every breakpoint and every framework error points at generated code the author never
//! wrote — and the research report names the absence of source maps as an adoption blocker
//! rather than a nicety.
//!
//! # What is mapped, and what is not
//!
//! Line granularity, not column. A GUML line becomes a *region* of emitted code — one `data`
//! directive becomes a `useState`, an effect and three callbacks — so there is no honest column
//! correspondence to report. Line-to-line is the strongest true statement, and a map that
//! claimed columns would send a debugger to an arbitrary character.
//!
//! The format is standard v3 with VLQ mappings and `sourcesContent` inlined, so DevTools and
//! every bundler read it without knowing what GUML is.

use std::fmt::Write as _;

/// One emitted line and the source line it came from, both 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    pub emitted_line: u32,
    pub source_line: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    mappings: Vec<Mapping>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that the next emitted line came from `source_line` (1-based, as diagnostics use).
    pub fn mark(&mut self, emitted_line: u32, source_line: u32) {
        if source_line == 0 {
            return;
        }
        self.mappings.push(Mapping { emitted_line, source_line: source_line.saturating_sub(1) });
    }

    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// The source line for an emitted line, for `guml where`-style lookups and for tests.
    pub fn source_line_of(&self, emitted_line: u32) -> Option<u32> {
        self.mappings
            .iter()
            .filter(|m| m.emitted_line <= emitted_line)
            .max_by_key(|m| m.emitted_line)
            .map(|m| m.source_line + 1)
    }

    /// Serialise as Source Map v3.
    ///
    /// `emitted_lines` is needed because the `mappings` field has one `;`-separated group per
    /// emitted line, including the lines that map to nothing.
    pub fn to_json(&self, source_path: &str, source_text: &str, emitted_lines: usize) -> String {
        let mut groups: Vec<String> = vec![String::new(); emitted_lines.max(1)];

        // VLQ fields are deltas against the previous *mapped* segment, not absolutes.
        let mut prev_source_line: i64 = 0;
        let mut sorted = self.mappings.clone();
        sorted.sort_by_key(|m| m.emitted_line);

        for m in &sorted {
            let Some(slot) = groups.get_mut(m.emitted_line as usize) else { continue };
            if !slot.is_empty() {
                // One mapping per line is enough at line granularity; keep the first.
                continue;
            }
            let mut segment = String::new();
            // generated column 0, source index 0, source line delta, source column 0.
            vlq(&mut segment, 0);
            vlq(&mut segment, 0);
            vlq(&mut segment, m.source_line as i64 - prev_source_line);
            vlq(&mut segment, 0);
            prev_source_line = m.source_line as i64;
            *slot = segment;
        }

        let mut out = String::from("{\n  \"version\": 3,\n");
        let _ = writeln!(out, "  \"file\": {:?},", generated_name(source_path));
        let _ = writeln!(out, "  \"sources\": [{:?}],", source_path);
        // Inlined so the map is self-contained: a `.guml` file next to a `.tsx` in someone
        // else's build output is not something a debugger can be relied on to find.
        let _ = writeln!(out, "  \"sourcesContent\": [{:?}],", source_text);
        let _ = writeln!(out, "  \"names\": [],");
        let _ = writeln!(out, "  \"mappings\": {:?}", groups.join(";"));
        out.push_str("}\n");
        out
    }
}

fn generated_name(source_path: &str) -> String {
    source_path.trim_end_matches(".guml").to_string() + ".tsx"
}

/// Base64 VLQ, as the Source Map spec defines it.
fn vlq(out: &mut String, value: i64) {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    // Sign goes in the low bit.
    let mut v = if value < 0 { ((-value) << 1) | 1 } else { value << 1 };
    loop {
        let mut digit = v & 0b1_1111;
        v >>= 5;
        if v > 0 {
            // Continuation bit.
            digit |= 0b10_0000;
        }
        out.push(ALPHABET[digit as usize] as char);
        if v == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlq_matches_the_specs_examples() {
        // The canonical values from the Source Map v3 spec.
        let cases = [(0, "A"), (1, "C"), (-1, "D"), (16, "gB"), (-16, "hB"), (123, "2H")];
        for (value, expected) in cases {
            let mut out = String::new();
            vlq(&mut out, value);
            assert_eq!(out, expected, "vlq({value})");
        }
    }

    #[test]
    fn a_mapping_is_recoverable() {
        let mut map = SourceMap::new();
        map.mark(0, 1);
        map.mark(5, 3);
        map.mark(9, 12);

        assert_eq!(map.source_line_of(0), Some(1));
        // Between marks, the most recent mark still owns the line: an emitted region belongs
        // to the source line that produced it.
        assert_eq!(map.source_line_of(4), Some(1));
        assert_eq!(map.source_line_of(5), Some(3));
        assert_eq!(map.source_line_of(11), Some(12));
    }

    #[test]
    fn json_is_v3_with_a_group_per_emitted_line() {
        let mut map = SourceMap::new();
        map.mark(0, 1);
        map.mark(2, 4);
        let json = map.to_json("page.guml", "page P\n", 4);

        assert!(json.contains("\"version\": 3"));
        assert!(json.contains("\"sources\": [\"page.guml\"]"));
        assert!(json.contains("\"sourcesContent\""));
        // Four emitted lines → three separators. Lines 1 and 3 map to nothing.
        let mappings = json.split("\"mappings\": ").nth(1).unwrap();
        assert_eq!(mappings.matches(';').count(), 3, "{mappings}");
        // First group: column 0, source 0, line 0, column 0.
        assert!(mappings.starts_with("\"AAAA;"), "{mappings}");
    }

    #[test]
    fn line_zero_is_not_recorded() {
        // Diagnostics use 1-based lines; a zero means "no span", and mapping it to line -1
        // would emit a map that points above the file.
        let mut map = SourceMap::new();
        map.mark(3, 0);
        assert!(map.is_empty());
    }
}
