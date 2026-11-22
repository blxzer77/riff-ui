//! riff-story: riff-ui's component gallery, with every component compared
//! against riff-uiux cell by cell.
//!
//! ```text
//! cargo run                                  # first component, Compare tab
//! cargo run -- button                        # a component (or `tokens`)
//! cargo run -- button --demo                 # its Demo tab
//! cargo run -- button --mode overlay --dark  # a comparison mode, dark theme
//! ```

mod compare;
mod components;
mod registry;
mod shell;
mod story;

use gpui_kit::assets::AllAssets;
use gpui_kit::*;
use riff_ui::theme::{ThemeMode, set_mode};

use shell::{Options, Shell, Tab};

const USAGE: &str =
    "usage: riff-story [SLUG] [--demo] [--mode stacked|reference|live|overlay] [--dark]";

fn parse_args() -> (Options, bool) {
    let mut options = Options::default();
    let mut dark = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--demo" => options.tab = Tab::Demo,
            "--dark" => dark = true,
            "--mode" => {
                let name = args.next().unwrap_or_default();
                options.mode = compare::Mode::parse(&name).unwrap_or_else(|| {
                    eprintln!("unknown mode {name:?}\n{USAGE}");
                    std::process::exit(2);
                });
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            slug if !slug.starts_with('-') && options.slug.is_none() => {
                options.slug = Some(slug.to_owned());
            }
            other => {
                eprintln!("unknown argument {other:?}\n{USAGE}");
                std::process::exit(2);
            }
        }
    }
    (options, dark)
}

fn main() {
    let (options, dark) = parse_args();
    // The full Lucide catalog, so stories can use any icon.
    gpui_kit::application()
        .with_assets(AllAssets)
        .run(move |cx| {
            riff_ui::init(cx);
            if dark {
                set_mode(ThemeMode::Dark, cx);
            }
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(1500.), px(900.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| Shell::new(options, window, cx)),
            )
            .expect("failed to open window");
        });
}
