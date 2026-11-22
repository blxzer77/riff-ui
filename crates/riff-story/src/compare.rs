//! The Compare tab: each cell of a story as riff-uiux renders it (a picture
//! from `tools/reference/build.py`) and as riff-ui does, side by side.
//!
//! Both are drawn in a box of the same size: the picture extends `margin`
//! pixels past the component on every side (for focus rings and shadows),
//! and the live cell is padded by the same margin, so the two align exactly.

use std::time::Duration;

use gpui_kit::*;
use riff_ui::theme::RiffTheme;

use crate::registry::{Entry, Pin};
use crate::story::{Cell, Reference};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Mode {
    /// Picture over live, one column per cell.
    #[default]
    Stacked,
    /// The pictures only.
    Reference,
    /// The live components only.
    Live,
    /// Live at half opacity over the picture, to check alignment.
    Overlay,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Stacked, Mode::Reference, Mode::Live, Mode::Overlay];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Stacked => "Stacked",
            Mode::Reference => "riff-uiux",
            Mode::Live => "riff-ui",
            Mode::Overlay => "Overlay",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| format!("{mode:?}").eq_ignore_ascii_case(name))
    }
}

/// How the reference pictures of the shown story stand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    /// Made from the current story file.
    Current,
    /// Made from an older story file; cells may be missing or different.
    Stale,
    /// Not made yet.
    Missing,
}

pub fn status(reference: Option<&Reference>) -> Status {
    match reference {
        Some(r) if r.current => Status::Current,
        Some(_) => Status::Stale,
        None => Status::Missing,
    }
}

/// The Compare tab for one story. `reference` is the manifest for the
/// active mode, loaded once by the shell.
pub fn view(
    entry: &Entry,
    reference: Option<&Reference>,
    mode: Mode,
    theme: &RiffTheme,
) -> AnyElement {
    let freeze = entry.story.freeze_ms.map(Duration::from_millis);
    let muted = theme.muted;
    div()
        .flex()
        .flex_col()
        .gap_6()
        .children(entry.story.sections.iter().map(|section| {
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(section.title.clone()),
                )
                .children(section.rows.iter().map(|row| {
                    // The title sits above its cells, so a row that wraps on a
                    // narrow window does not read as two rows.
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_xs().text_color(muted).child(row.title.clone()))
                        .child(div().flex().flex_wrap().items_end().gap(px(8.)).children(
                            row.cells.iter().map(|cell| {
                                render_cell(entry, cell, reference, mode, freeze, theme)
                            }),
                        ))
                }))
        }))
        .into_any_element()
}

fn render_cell(
    entry: &Entry,
    cell: &Cell,
    reference: Option<&Reference>,
    mode: Mode,
    freeze: Option<Duration>,
    theme: &RiffTheme,
) -> AnyElement {
    let pin = Pin {
        state: cell.state,
        freeze,
    };
    let live = match (entry.cell)(&cell.key, &cell.props, pin) {
        Ok(element) => element,
        Err(error) => problem(format!("{}: {error}", cell.id), theme),
    };
    let margin = px(reference.map_or(8., |r| r.margin));
    // The live component, padded so its box matches the picture's.
    let live = div().p(margin).flex().items_center().child(live);

    let picture = reference.and_then(|r| {
        let (width, height) = *r.cells.get(&cell.key)?;
        let margin = r.margin;
        Some(
            img(r.picture(&cell.key))
                .flex_none()
                .w(px(width + margin * 2.))
                .h(px(height + margin * 2.)),
        )
    });
    let picture = match picture {
        Some(picture) => picture.into_any_element(),
        None => placeholder(theme),
    };

    let body = div().flex().flex_col();
    let body = match mode {
        Mode::Stacked => body.child(picture).child(live),
        Mode::Reference => body.child(picture),
        Mode::Live => body.child(live),
        Mode::Overlay => body
            .relative()
            .child(picture)
            .child(div().absolute().top_0().left_0().opacity(0.5).child(live)),
    };
    // Named by its cell id, the name to use when reporting a difference.
    div()
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .child(body)
        .child(
            div()
                .text_xs()
                .text_color(theme.placeholder)
                .child(cell.id.clone()),
        )
        .into_any_element()
}

/// Where a picture is missing, so the cell keeps its place.
fn placeholder(theme: &RiffTheme) -> AnyElement {
    div()
        .min_w(px(40.))
        .h(px(24.))
        .px_2()
        .flex()
        .items_center()
        .rounded(theme.radius_sm)
        .border_1()
        .border_dashed()
        .border_color(theme.border_strong)
        .text_xs()
        .text_color(theme.placeholder)
        .child("no picture")
        .into_any_element()
}

/// A cell its component cannot build: shown in place rather than dropped.
pub fn problem(message: String, theme: &RiffTheme) -> AnyElement {
    div()
        .px_2()
        .py_1()
        .rounded(theme.radius_sm)
        .bg(theme.danger_soft)
        .text_xs()
        .text_color(theme.danger)
        .child(message)
        .into_any_element()
}
