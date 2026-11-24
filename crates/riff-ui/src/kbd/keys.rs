//! Key name normalization, platform formatting (`⌘` vs `Ctrl`), and spoken
//! screen-reader labels matching riff-uiux `kbd.tsx`.

use gpui_kit::SharedString;

/// Target platform for keycap glyphs and names.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum KbdPlatform {
    /// macOS glyphs (`⌘`, `⌃`, `⌥`, `⇧`, `↵`, `⌫`, `⌦`).
    Mac,
    /// Windows and Linux names (`Ctrl`, `Alt`, `Shift`, `Enter`, `Backspace`, `Del`).
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct KeyPart {
    pub label: SharedString,
    pub spoken: SharedString,
    pub glyph: bool,
}

struct Named {
    mac: &'static str,
    other: &'static str,
    spoken_mac: &'static str,
    spoken_other: &'static str,
    glyph: bool,
}

const fn named(
    mac: &'static str,
    other: &'static str,
    spoken_mac: &'static str,
    spoken_other: &'static str,
    glyph: bool,
) -> Named {
    Named {
        mac,
        other,
        spoken_mac,
        spoken_other,
        glyph,
    }
}

fn lookup_named(id: &str) -> Option<Named> {
    Some(match id {
        "mod" => named("⌘", "Ctrl", "Command", "Control", true),
        "cmd" => named("⌘", "⌘", "Command", "Command", true),
        "ctrl" => named("⌃", "Ctrl", "Control", "Control", true),
        "alt" => named("⌥", "Alt", "Option", "Alt", true),
        "shift" => named("⇧", "Shift", "Shift", "Shift", true),
        "enter" => named("↵", "Enter", "Return", "Enter", true),
        "esc" => named("Esc", "Esc", "Escape", "Escape", false),
        "tab" => named("Tab", "Tab", "Tab", "Tab", false),
        "space" => named("Space", "Space", "Space", "Space", false),
        "backspace" => named("⌫", "Backspace", "Delete", "Backspace", true),
        "delete" => named("⌦", "Del", "Forward delete", "Delete", true),
        "up" => named("↑", "↑", "Up arrow", "Up arrow", true),
        "down" => named("↓", "↓", "Down arrow", "Down arrow", true),
        "left" => named("←", "←", "Left arrow", "Left arrow", true),
        "right" => named("→", "→", "Right arrow", "Right arrow", true),
        "pageup" => named("PgUp", "PgUp", "Page up", "Page up", false),
        "pagedown" => named("PgDn", "PgDn", "Page down", "Page down", false),
        "home" => named("Home", "Home", "Home", "Home", false),
        "end" => named("End", "End", "End", "End", false),
        _ => return None,
    })
}

fn resolve_alias(lower: &str) -> &str {
    match lower {
        "command" | "meta" | "⌘" => "cmd",
        "control" => "ctrl",
        "option" | "opt" => "alt",
        "return" => "enter",
        "escape" => "esc",
        "del" => "delete",
        "arrowup" => "up",
        "arrowdown" => "down",
        "arrowleft" => "left",
        "arrowright" => "right",
        "pgup" => "pageup",
        "pgdn" => "pagedown",
        other => other,
    }
}

fn lookup_spoken_punctuation(key: &str) -> Option<&'static str> {
    Some(match key {
        "." => "Period",
        "," => "Comma",
        "/" => "Slash",
        "\\" => "Backslash",
        "?" => "Question mark",
        "[" => "Left bracket",
        "]" => "Right bracket",
        "-" => "Minus",
        "+" => "Plus",
        "=" => "Equals",
        ";" => "Semicolon",
        "'" => "Quote",
        "`" => "Backtick",
        _ => return None,
    })
}

pub(super) fn describe(raw: &str, platform: KbdPlatform) -> KeyPart {
    let key = raw.trim();
    let lower = key.to_lowercase();
    let id = resolve_alias(&lower);
    if let Some(named) = lookup_named(id) {
        let mac = platform == KbdPlatform::Mac;
        let label = if mac { named.mac } else { named.other };
        let spoken = if mac {
            named.spoken_mac
        } else {
            named.spoken_other
        };
        return KeyPart {
            label: label.into(),
            spoken: spoken.into(),
            glyph: named.glyph && label.chars().count() == 1,
        };
    }

    let label: SharedString = if key.chars().count() == 1 {
        key.to_uppercase().into()
    } else {
        key.to_owned().into()
    };
    let spoken = lookup_spoken_punctuation(key)
        .map(SharedString::from)
        .unwrap_or_else(|| label.clone());
    KeyPart {
        label,
        spoken,
        glyph: false,
    }
}

pub(super) fn spoken_combo(parts: &[KeyPart]) -> SharedString {
    parts
        .iter()
        .map(|part| part.spoken.as_ref())
        .collect::<Vec<_>>()
        .join(" ")
        .into()
}
