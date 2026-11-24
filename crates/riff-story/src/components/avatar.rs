//! Avatar: the story's cells, and an interactive avatar & group demo.

use std::sync::{Arc, OnceLock};

use gpui_kit::*;
use riff_ui::avatar::{Avatar, AvatarGroup, AvatarKind, AvatarShape, AvatarSize, AvatarStatus};
use riff_ui::button::{Button, ButtonSize};
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "name", "size", "shape", "status", "kind", "seed", "color", "working", "photo", "broken",
    "group", "max", "total",
];

const ITEM_PROPS: &[&str] = &[
    "name", "size", "shape", "status", "kind", "seed", "color", "working", "photo", "broken",
];

const SAMPLE_PHOTO_BYTES: &[u8] = include_bytes!("../../assets/sample-avatar.png");

fn sample_photo() -> ImageSource {
    static RENDER: OnceLock<Arc<RenderImage>> = OnceLock::new();
    ImageSource::Custom(Arc::new(|_, cx| {
        if let Some(cached) = RENDER.get() {
            return Some(Ok(cached.clone()));
        }
        let img = Image::from_bytes(ImageFormat::Png, SAMPLE_PHOTO_BYTES.to_vec());
        match img.to_image_data(cx.svg_renderer()) {
            Ok(data) => {
                let data = RENDER.get_or_init(|| data).clone();
                Some(Ok(data))
            }
            Err(err) => Some(Err(err.into())),
        }
    }))
}

fn parse_size(v: i64) -> Result<AvatarSize, String> {
    match v {
        20 => Ok(AvatarSize::S20),
        24 => Ok(AvatarSize::S24),
        32 => Ok(AvatarSize::S32),
        40 => Ok(AvatarSize::S40),
        other => Err(format!("unknown avatar size {other}")),
    }
}

fn parse_shape(s: &str) -> Result<AvatarShape, String> {
    match s {
        "circle" => Ok(AvatarShape::Circle),
        "square" => Ok(AvatarShape::Square),
        other => Err(format!("unknown avatar shape {other:?}")),
    }
}

fn parse_status(s: &str) -> Result<AvatarStatus, String> {
    match s {
        "online" => Ok(AvatarStatus::Online),
        "away" => Ok(AvatarStatus::Away),
        "busy" => Ok(AvatarStatus::Busy),
        other => Err(format!("unknown avatar status {other:?}")),
    }
}

fn parse_kind(s: &str) -> Result<AvatarKind, String> {
    match s {
        "person" => Ok(AvatarKind::Person),
        "agent" => Ok(AvatarKind::Agent),
        other => Err(format!("unknown avatar kind {other:?}")),
    }
}

fn parse_hex_color(s: &str) -> Result<Hsla, String> {
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| format!("expected #rrggbb color, got {s:?}"))?;
    if hex.len() != 6 {
        return Err(format!("expected 6-digit hex color, got {s:?}"));
    }
    let rgb = u32::from_str_radix(hex, 16).map_err(|_| format!("invalid hex color {s:?}"))?;
    Ok(gpui_kit::rgb(rgb).into())
}

fn build_avatar(props: &Props, pin: Pin) -> Result<Avatar, String> {
    let mut avatar = Avatar::new();
    if let Some(name) = props.str("name") {
        avatar = avatar.name(name.to_owned());
    }
    if let Some(size) = props.i64("size") {
        avatar = avatar.size(parse_size(size)?);
    }
    if let Some(shape) = props.str("shape") {
        avatar = avatar.shape(parse_shape(shape)?);
    }
    if let Some(status) = props.str("status") {
        avatar = avatar.status(parse_status(status)?);
    }
    if let Some(kind) = props.str("kind") {
        avatar = avatar.kind(parse_kind(kind)?);
    }
    if let Some(seed) = props.str("seed") {
        avatar = avatar.seed(seed.to_owned());
    }
    if let Some(color) = props.str("color") {
        avatar = avatar.color(parse_hex_color(color)?);
    }
    if props.bool("working") {
        avatar = avatar.working(true);
    }
    if props.bool("broken") {
        avatar = avatar.src("nonexistent://missing-avatar.png");
    } else if props.bool("photo") {
        avatar = avatar.src(sample_photo());
    }
    if let Some(at) = pin.freeze {
        avatar = avatar.freeze_motion_at(at);
    }
    Ok(avatar)
}

/// The live avatar or avatar group for one cell of `stories/avatar.json`.
pub fn cell(_id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    if let Some(items) = props.objects("group") {
        let mut group = AvatarGroup::new();
        if let Some(max) = props.i64("max") {
            if max < 1 {
                return Err(format!("invalid group max {max}"));
            }
            group = group.max(max as usize);
        }
        if let Some(total) = props.i64("total") {
            if total < 0 {
                return Err(format!("negative group total {total}"));
            }
            group = group.total(total as usize);
        }
        if let Some(size) = props.i64("size") {
            group = group.size(parse_size(size)?);
        }
        for item in items {
            if let Some(unknown) = item.keys().find(|key| !ITEM_PROPS.contains(key)) {
                return Err(format!("unknown group item prop {unknown:?}"));
            }
            group = group.avatar(build_avatar(&item, pin)?);
        }
        return Ok(group.into_any_element());
    }

    Ok(build_avatar(props, pin)?.into_any_element())
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::new()).into());

struct Demo {
    size: AvatarSize,
    shape: Option<AvatarShape>,
    status: Option<AvatarStatus>,
    agent: bool,
    working: bool,
    photo: bool,
    total_members: usize,
}

impl Demo {
    fn new() -> Self {
        Self {
            size: AvatarSize::S32,
            shape: None,
            status: Some(AvatarStatus::Online),
            agent: false,
            working: false,
            photo: false,
            total_members: 6,
        }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let mut single = if self.agent {
            Avatar::new()
                .kind(AvatarKind::Agent)
                .seed("triage")
                .name("Triage agent")
                .working(self.working)
        } else {
            let mut a = Avatar::new().name("Mara Lindqvist");
            if self.photo {
                a = a.src(sample_photo());
            }
            a
        }
        .size(self.size);

        if let Some(shape) = self.shape {
            single = single.shape(shape);
        }
        if let Some(status) = self.status {
            single = single.status(status);
        }

        let group_size = self.size;
        let members = [
            Avatar::new()
                .name("Grace Hall")
                .status(AvatarStatus::Online),
            Avatar::new().name("Alice Reed").src(sample_photo()),
            Avatar::new().name("David Evans").status(AvatarStatus::Away),
            Avatar::new().name("Iris Lin"),
            Avatar::new()
                .kind(AvatarKind::Agent)
                .seed("triage")
                .name("Triage agent")
                .working(self.working),
        ];

        div()
            .flex()
            .flex_col()
            .gap_4()
            .max_w(px(420.))
            .child(div().flex().items_center().gap_3().child(single).child(
                div().text_sm().text_color(theme.ink).child(if self.agent {
                    "Triage agent"
                } else {
                    "Mara Lindqvist"
                }),
            ))
            .child(
                div().flex().flex_wrap().gap_2().children(
                    [
                        (AvatarSize::S20, "20"),
                        (AvatarSize::S24, "24"),
                        (AvatarSize::S32, "32"),
                        (AvatarSize::S40, "40"),
                    ]
                    .into_iter()
                    .map(|(sz, label)| {
                        Button::new(("avt-sz", sz as usize))
                            .label(label)
                            .size(ButtonSize::Sm)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.size = sz;
                                cx.notify();
                            }))
                    }),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("avt-kind")
                            .label(if self.agent {
                                "Kind: Agent"
                            } else {
                                "Kind: Person"
                            })
                            .size(ButtonSize::Sm)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.agent = !this.agent;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("avt-shape")
                            .label(match self.shape {
                                None => "Shape: Default",
                                Some(AvatarShape::Circle) => "Shape: Circle",
                                Some(AvatarShape::Square) => "Shape: Square",
                            })
                            .size(ButtonSize::Sm)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.shape = match this.shape {
                                    None => Some(AvatarShape::Square),
                                    Some(AvatarShape::Square) => Some(AvatarShape::Circle),
                                    Some(AvatarShape::Circle) => None,
                                };
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("avt-status")
                            .label(match self.status {
                                None => "Status: None",
                                Some(AvatarStatus::Online) => "Status: Online",
                                Some(AvatarStatus::Away) => "Status: Away",
                                Some(AvatarStatus::Busy) => "Status: Busy",
                            })
                            .size(ButtonSize::Sm)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.status = match this.status {
                                    None => Some(AvatarStatus::Online),
                                    Some(AvatarStatus::Online) => Some(AvatarStatus::Away),
                                    Some(AvatarStatus::Away) => Some(AvatarStatus::Busy),
                                    Some(AvatarStatus::Busy) => None,
                                };
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("avt-photo")
                            .label(if self.photo {
                                "Photo: On"
                            } else {
                                "Photo: Off"
                            })
                            .size(ButtonSize::Sm)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.photo = !this.photo;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("avt-work")
                            .label(if self.working {
                                "Working: On"
                            } else {
                                "Working: Off"
                            })
                            .size(ButtonSize::Sm)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.working = !this.working;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        AvatarGroup::new()
                            .size(group_size)
                            .max(4)
                            .total(self.total_members)
                            .avatars(members),
                    )
                    .child(
                        Button::new("avt-more")
                            .label("+ Member")
                            .size(ButtonSize::Sm)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.total_members = if this.total_members >= 120 {
                                    4
                                } else if this.total_members >= 12 {
                                    120
                                } else {
                                    this.total_members + 3
                                };
                                cx.notify();
                            })),
                    ),
            )
    }
}
