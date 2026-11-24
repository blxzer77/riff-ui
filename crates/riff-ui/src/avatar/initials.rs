//! Deterministic hue selection, initials extraction, and 3×3 agent pixel patterns.

use unicode_segmentation::UnicodeSegmentation;

/// Picks one of 8 palette indices from a stable key (name or id).
pub(super) fn hue_of(key: &str) -> usize {
    let mut h: u32 = 7;
    for ch in key.chars() {
        h = h.wrapping_mul(31).wrapping_add(ch as u32);
    }
    (h % 8) as usize
}

/// Deterministic symmetric 3×3 pattern with at least 5 active cells.
pub(super) fn agent_pattern(id: &str) -> [bool; 9] {
    let mut h: u32 = 0;
    for ch in id.chars() {
        // Matches JS `c.charCodeAt(0)` for BMP chars (all typical agent ids).
        h = h.wrapping_mul(31).wrapping_add((ch as u32) & 0xffff);
    }
    for k in 0..64u32 {
        let bits = (h >> (k % 26)) & 63;
        let mut cells = [false; 9];
        let mut count = 0;
        for r in 0..3 {
            let a = ((bits >> (r * 2)) & 1) != 0;
            let b = ((bits >> (r * 2 + 1)) & 1) != 0;
            cells[r * 3] = a;
            cells[r * 3 + 1] = b;
            cells[r * 3 + 2] = a;
            count += usize::from(a) * 2 + usize::from(b);
        }
        if count >= 5 {
            return cells;
        }
        h = h.wrapping_mul(1_103_515_245).wrapping_add(12_345);
    }
    [true, false, true, false, true, false, true, false, true]
}

fn is_letter_or_digit(grapheme: &str) -> bool {
    grapheme.chars().any(char::is_alphanumeric)
}

fn is_emoji_codepoint(cp: u32) -> bool {
    matches!(
        cp,
        0x1F1E6..=0x1F1FF // Regional indicators (flags)
            | 0x1F300..=0x1FAFF // Pictographs & symbols
            | 0x2600..=0x27BF // Misc symbols & dingbats
            | 0x231A..=0x231B
            | 0x23E9..=0x23F3
            | 0x23F8..=0x23FA
            | 0x25AA..=0x25AB
            | 0x25B6
            | 0x25C0
            | 0x25FB..=0x25FE
            | 0x2B05..=0x2B07
            | 0x2B1B..=0x2B1C
            | 0x2B50
            | 0x2B55
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
    )
}

fn is_emoji(grapheme: &str) -> bool {
    !is_letter_or_digit(grapheme) && grapheme.chars().any(|ch| is_emoji_codepoint(ch as u32))
}

fn is_suffix(word: &str) -> bool {
    let trimmed = word.strip_suffix('.').unwrap_or(word);
    matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "jr" | "sr" | "ii" | "iii" | "iv" | "vi" | "phd" | "md" | "esq"
    )
}

/// First and last word initials: `"Mara Lindqvist"` → `"ML"`, `"Cher"` → `"C"`,
/// `"Christopher Montgomery III"` → `"CM"`, `"🦊 Fox Team"` → `"🦊"`, `"(Ana) Ruiz"` → `"AR"`.
pub(super) fn initials_of(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }
    if let Some(lead) = words[0].graphemes(true).next()
        && is_emoji(lead)
    {
        return lead.to_owned();
    }

    let end = if words.len() > 1 && is_suffix(words[words.len() - 1]) {
        words.len() - 1
    } else {
        words.len()
    };
    let firsts: Vec<&str> = words[..end]
        .iter()
        .filter_map(|word| word.graphemes(true).find(|g| is_letter_or_digit(g)))
        .collect();

    if firsts.is_empty() {
        return words[0].graphemes(true).next().unwrap_or("").to_owned();
    }

    let mut out = String::new();
    out.push_str(firsts[0]);
    if firsts.len() > 1 {
        out.push_str(firsts[firsts.len() - 1]);
    }
    out.to_uppercase()
}
