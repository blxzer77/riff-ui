//! Sizes and animation speeds of riff-uiux's `.ui-spn`.

use std::time::Duration;

use gpui_kit::{Pixels, px};

pub(super) const DEFAULT_SIZE: Pixels = px(14.);
pub(super) const NORMAL_SPEED: Duration = Duration::from_millis(750);
pub(super) const REDUCED_SPEED: Duration = Duration::from_millis(2400);

/// `<svg viewBox="0 0 16 16">` from `spinner.tsx`: a 20%-opacity ring and a
/// quarter-turn round-capped arc.
pub(super) const SPINNER_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="2" stroke-linecap="round"><circle cx="8" cy="8" r="6" opacity="0.2"/><path d="M8 2a6 6 0 0 1 6 6"/></svg>"#;
pub(super) const SPINNER_SVG_KEY: &str = "riff-spinner";
