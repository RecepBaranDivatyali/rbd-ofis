use serde::{Deserialize, Serialize};

/// A 2D bounding rectangle in PDF point coordinates: [x0, y0, x1, y1].
/// Standard PDF coordinate space origin (0, 0) is at bottom-left of the page.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PdfRect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl PdfRect {
    pub const ZERO: Self = Self {
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
    };

    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn width(&self) -> f32 {
        (self.x1 - self.x0).abs()
    }

    pub fn height(&self) -> f32 {
        (self.y1 - self.y0).abs()
    }

    pub fn union(&self, other: &PdfRect) -> PdfRect {
        PdfRect {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }
}

/// A search hit found on a document page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub page_index: u32,
    pub match_text: String,
    pub snippet: String,
    pub rects: Vec<PdfRect>,
}

/// Convert text to lowercase with Turkish-specific orthographic rules:
/// - 'İ' (U+0130, Latin Capital Letter I With Dot Above) -> 'i' (U+0069)
/// - 'I' (U+0049, Latin Capital Letter I) -> 'ı' (U+0131)
/// - 'I' followed by combining dot above (U+0307) -> 'i'
/// - 'i' followed by combining dot above (U+0307) -> 'i' (de-duplicate combining dot)
/// - Standard Unicode lowercase for all other characters (e.g. 'Ş'->'ş', 'Ğ'->'ğ', 'Ç'->'ç', 'Ö'->'ö', 'Ü'->'ü')
pub fn turkish_lowercase(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'İ' => result.push('i'),
            'I' => {
                if chars.peek() == Some(&'\u{0307}') {
                    chars.next();
                    result.push('i');
                } else {
                    result.push('ı');
                }
            }
            'i' => {
                if chars.peek() == Some(&'\u{0307}') {
                    chars.next();
                }
                result.push('i');
            }
            other => {
                for lower in other.to_lowercase() {
                    result.push(lower);
                }
            }
        }
    }
    result
}

/// Character-level search match span: `(start_char_idx, end_char_idx)` (half-open range).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchSpan {
    pub start: usize,
    pub end: usize,
}

/// Find all matching spans of `query` within `text` (in original character indices).
/// Supports Turkish-aware case folding and ASCII-I / Turkish-I cross-matching when case-insensitive.
pub fn find_matches(text: &str, query: &str, case_sensitive: bool) -> Vec<MatchSpan> {
    if query.is_empty() || text.is_empty() {
        return Vec::new();
    }

    let orig_chars: Vec<char> = text.chars().collect();
    if case_sensitive {
        let query_chars: Vec<char> = query.chars().collect();
        if query_chars.len() > orig_chars.len() {
            return Vec::new();
        }

        let mut matches = Vec::new();
        for i in 0..=(orig_chars.len() - query_chars.len()) {
            if orig_chars[i..i + query_chars.len()] == query_chars[..] {
                matches.push(MatchSpan {
                    start: i,
                    end: i + query_chars.len(),
                });
            }
        }
        return matches;
    }

    // Case-insensitive Turkish search:
    // Build folded stream of characters mapped back to original character indices
    let mut folded_chars: Vec<char> = Vec::with_capacity(orig_chars.len());
    let mut char_map: Vec<usize> = Vec::with_capacity(orig_chars.len());

    let mut i = 0;
    while i < orig_chars.len() {
        let c = orig_chars[i];
        let next_c = if i + 1 < orig_chars.len() {
            Some(orig_chars[i + 1])
        } else {
            None
        };

        match c {
            'İ' => {
                folded_chars.push('i');
                char_map.push(i);
                i += 1;
            }
            'I' => {
                if next_c == Some('\u{0307}') {
                    folded_chars.push('i');
                    char_map.push(i);
                    i += 2; // skip combining dot
                } else {
                    folded_chars.push('ı');
                    char_map.push(i);
                    i += 1;
                }
            }
            'i' => {
                if next_c == Some('\u{0307}') {
                    folded_chars.push('i');
                    char_map.push(i);
                    i += 2; // skip redundant combining dot
                } else {
                    folded_chars.push('i');
                    char_map.push(i);
                    i += 1;
                }
            }
            other => {
                for lower in other.to_lowercase() {
                    folded_chars.push(lower);
                    char_map.push(i);
                }
                i += 1;
            }
        }
    }

    let query_folded: Vec<char> = turkish_lowercase(query).chars().collect();
    if query_folded.is_empty() || query_folded.len() > folded_chars.len() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let q_len = query_folded.len();

    let chars_match =
        |f: char, q: char| -> bool { f == q || (f == 'i' && q == 'ı') || (f == 'ı' && q == 'i') };

    for idx in 0..=(folded_chars.len() - q_len) {
        let mut matched = true;
        for j in 0..q_len {
            if !chars_match(folded_chars[idx + j], query_folded[j]) {
                matched = false;
                break;
            }
        }

        if matched {
            let start = char_map[idx];
            let end_folded = idx + q_len - 1;
            let end = char_map[end_folded] + 1;
            // Prevent duplicate spans if combining characters collapsed
            if matches.last().is_none_or(|m: &MatchSpan| m.start != start) {
                matches.push(MatchSpan { start, end });
            }
        }
    }

    matches
}

/// Create a snippet of text surrounding the match with up to `context_chars` characters on each side.
pub fn build_snippet(
    orig_chars: &[char],
    start: usize,
    end: usize,
    context_chars: usize,
) -> String {
    let snippet_start = start.saturating_sub(context_chars);
    let snippet_end = (end + context_chars).min(orig_chars.len());

    let prefix = if snippet_start > 0 { "..." } else { "" };
    let suffix = if snippet_end < orig_chars.len() {
        "..."
    } else {
        ""
    };

    let body: String = orig_chars[snippet_start..snippet_end].iter().collect();
    format!("{prefix}{body}{suffix}")
}

/// Group a sequence of character rectangles into continuous line bounding boxes.
pub fn group_char_rects_into_lines(rects: &[PdfRect]) -> Vec<PdfRect> {
    if rects.is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<PdfRect> = Vec::new();
    let mut current_line = rects[0];

    for &r in &rects[1..] {
        // If roughly on the same vertical baseline (allowing small font variance)
        let y_diff = (r.y0 - current_line.y0).abs();
        let height = current_line.height().max(1.0);
        if y_diff <= height * 0.5 {
            current_line = current_line.union(&r);
        } else {
            lines.push(current_line);
            current_line = r;
        }
    }
    lines.push(current_line);

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turkish_case_folding() {
        assert_eq!(turkish_lowercase("İSTANBUL"), "istanbul");
        assert_eq!(turkish_lowercase("Isparta"), "ısparta");
        assert_eq!(turkish_lowercase("şoför"), "şoför");
        assert_eq!(turkish_lowercase("ŞOFÖR"), "şoför");
        assert_eq!(turkish_lowercase("BAŞLIK"), "başlık");
        assert_eq!(turkish_lowercase("ÇİÇEK"), "çiçek");
        assert_eq!(turkish_lowercase("ĞÜZEL"), "ğüzel");
        assert_eq!(turkish_lowercase("i\u{0307}stanbul"), "istanbul");
    }

    #[test]
    fn test_find_turkish_matches_case_insensitive() {
        let text = "İstanbul Boğazı ve ŞOFÖR ile Isparta gülü.";

        // Search "istanbul" -> matches "İstanbul"
        let m1 = find_matches(text, "istanbul", false);
        assert_eq!(m1.len(), 1);
        let s1: String = text
            .chars()
            .skip(m1[0].start)
            .take(m1[0].end - m1[0].start)
            .collect();
        assert_eq!(s1, "İstanbul");

        // Search "i̇stanbul" (with combining dot) -> matches "İstanbul"
        let m2 = find_matches(text, "i\u{0307}stanbul", false);
        assert_eq!(m2.len(), 1);

        // Search "ISTANBUL" (ASCII I) -> matches "İstanbul"
        let m3 = find_matches(text, "ISTANBUL", false);
        assert_eq!(m3.len(), 1);

        // Search "şoför" -> matches "ŞOFÖR"
        let m4 = find_matches(text, "şoför", false);
        assert_eq!(m4.len(), 1);
        let s4: String = text
            .chars()
            .skip(m4[0].start)
            .take(m4[0].end - m4[0].start)
            .collect();
        assert_eq!(s4, "ŞOFÖR");

        // Search "isparta" -> matches "Isparta"
        let m5 = find_matches(text, "isparta", false);
        assert_eq!(m5.len(), 1);
        let s5: String = text
            .chars()
            .skip(m5[0].start)
            .take(m5[0].end - m5[0].start)
            .collect();
        assert_eq!(s5, "Isparta");
    }

    #[test]
    fn test_find_matches_case_sensitive() {
        let text = "İstanbul ve istanbul";
        let m_upper = find_matches(text, "İstanbul", true);
        assert_eq!(m_upper.len(), 1);
        assert_eq!(m_upper[0].start, 0);

        let m_lower = find_matches(text, "istanbul", true);
        assert_eq!(m_lower.len(), 1);
        assert_eq!(m_lower[0].start, 12);
    }
}
