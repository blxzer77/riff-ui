//! Color values of the riff design tokens.
//!
//! Copied verbatim from riff-uiux `src/styles/tokens.css` (`:root` for light,
//! `[data-theme="dark"]` for dark) so the two can be diffed line by line. A
//! token the dark block does not override repeats its light value.
//!
//! Accepted forms: `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(r, g, b)`,
//! `rgba(r, g, b, a)`, and `var(--riff-<token>, <fallback>)` naming another
//! token of the same palette.

use super::Palette;

pub(super) const LIGHT: Palette = Palette {
    ink: "rgba(0, 0, 0, 0.88)",
    muted: "rgba(0, 0, 0, 0.45)",
    placeholder: "rgba(0, 0, 0, 0.27)",
    surface: "#ffffff",
    overlay: "var(--riff-surface, #ffffff)",
    subtle: "#f0f0f0",
    border: "rgba(0, 0, 0, 0.06)",
    border_strong: "rgba(0, 0, 0, 0.1)",
    primary: "rgba(0, 0, 0, 0.88)",
    on_primary: "#fcfcfc",
    focus: "rgba(0, 96, 226, 0.7)",
    danger: "#c4320a",
    danger_soft: "rgba(240, 68, 56, 0.1)",
    success: "#1e8a4f",
    success_soft: "rgba(95, 200, 138, 0.14)",
    warning: "#b25e00",
    warning_soft: "#fff1e0",
    info: "#0070d6",
    info_soft: "#eaf1ff",
    blue: "#4fb0fe",
    red: "#f04438",
    green: "#5fc88a",
    amber: "#e8b000",
    orange: "#f28a2e",
    purple: "#a651f0",
    pink: "#ee3e9e",
    teal: "#13b3a0",
    cyan: "#14b3f0",
    violet: "#a78bfa",
    indigo: "#9b9bff",
};

pub(super) const DARK: Palette = Palette {
    ink: "rgba(255, 255, 255, 0.9)",
    muted: "rgba(255, 255, 255, 0.62)",
    placeholder: "rgba(255, 255, 255, 0.3)",
    surface: "#1c1c1e",
    overlay: "var(--riff-surface, #1b1b1e)",
    subtle: "rgba(255, 255, 255, 0.04)",
    border: "rgba(255, 255, 255, 0.08)",
    border_strong: "rgba(255, 255, 255, 0.14)",
    primary: "rgba(255, 255, 255, 0.9)",
    on_primary: "#141416",
    focus: "rgba(77, 141, 255, 0.8)",
    danger: "#ff8a7a",
    danger_soft: "rgba(255, 107, 95, 0.14)",
    success: "#7dcf9f",
    success_soft: "rgba(125, 207, 159, 0.14)",
    warning: "#ffb35c",
    warning_soft: "rgba(242, 161, 92, 0.14)",
    info: "#4fb0fe",
    info_soft: "#16243f",
    blue: "#4fb0fe",
    red: "#ff6b5f",
    green: "#5fc88a",
    amber: "#e6c24d",
    orange: "#f2a15c",
    purple: "#b98af0",
    pink: "#f06db5",
    teal: "#4fc3b4",
    cyan: "#5cc6ee",
    violet: "#a78bfa",
    indigo: "#9b9bff",
};
