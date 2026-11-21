#![allow(dead_code)]

use eframe::egui;
use eframe::egui::{
    Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin,
    RichText, Shadow, Stroke, Vec2,
};

pub const BG_TOP: Color32 = Color32::from_rgb(8, 12, 16);
pub const BG_BOTTOM: Color32 = Color32::from_rgb(13, 45, 40);
pub const CARD_BG: Color32 = Color32::from_rgb(17, 25, 29);
pub const CARD_HOVER: Color32 = Color32::from_rgb(25, 40, 44);
pub const CHIP_BG: Color32 = Color32::from_rgb(22, 33, 37);
pub const CHIP_HOVER: Color32 = Color32::from_rgb(30, 50, 54);
pub const CHIP_ACTIVE: Color32 = Color32::from_rgb(34, 62, 62);
pub const BORDER: Color32 = Color32::from_rgb(42, 74, 68);
pub const ACCENT: Color32 = Color32::from_rgb(88, 205, 168);
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(9, 21, 17, 26);
pub const TEXT: Color32 = Color32::from_rgb(226, 236, 238);
pub const MUTED: Color32 = Color32::from_rgb(138, 158, 162);
pub const GREEN: Color32 = Color32::from_rgb(96, 214, 150);
pub const GREEN_SOFT: Color32 = Color32::from_rgba_premultiplied(9, 20, 14, 24);
pub const RED: Color32 = Color32::from_rgb(238, 112, 112);
pub const RED_SOFT: Color32 = Color32::from_rgba_premultiplied(22, 10, 10, 24);
pub const DEEP_BG: Color32 = Color32::from_rgb(10, 15, 19);
pub const FAINT_BG: Color32 = Color32::from_rgb(20, 30, 34);
pub const SELECT_BG: Color32 = Color32::from_rgb(26, 92, 78);

pub const CARD_SHADOW: Shadow = Shadow {
    offset: [0, 4],
    blur: 16,
    spread: 0,
    color: Color32::from_black_alpha(90),
};
pub const HOVER_SHADOW: Shadow = Shadow {
    offset: [0, 8],
    blur: 24,
    spread: 0,
    color: Color32::from_black_alpha(120),
};

const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");
const PHOSPHOR: &[u8] = include_bytes!("../assets/fonts/Phosphor.ttf");

pub const FAMILY_SEMI: &str = "inter_semibold";
pub const FAMILY_BOLD: &str = "inter_bold";
pub const FAMILY_ICONS: &str = "icons";

pub mod ph {
    pub const ARROW_LEFT: &str = "\u{E058}";
    pub const ARROW_RIGHT: &str = "\u{E06C}";
    pub const CARET_RIGHT: &str = "\u{E13A}";
    pub const CHECK: &str = "\u{E182}";
    pub const X: &str = "\u{E4F6}";
    pub const X_CIRCLE: &str = "\u{E4F8}";
    pub const WARNING_CIRCLE: &str = "\u{E4E2}";
    pub const SEAL_CHECK: &str = "\u{E606}";
    pub const MATH_OPERATIONS: &str = "\u{E31E}";
    pub const ATOM: &str = "\u{E5E4}";
    pub const FLASK: &str = "\u{E79E}";
    pub const PLANT: &str = "\u{EBAE}";
    pub const SCROLL: &str = "\u{EB7A}";
    pub const GLOBE: &str = "\u{E288}";
    pub const TRANSLATE: &str = "\u{E4A2}";
    pub const BOOK_OPEN: &str = "\u{E0E6}";
    pub const BOOK: &str = "\u{E0E2}";
    pub const LIGHTNING: &str = "\u{E2DE}";
    pub const PLAY: &str = "\u{E3D0}";
    pub const FOLDER_OPEN: &str = "\u{E256}";
    pub const ARCHIVE: &str = "\u{E00C}";
    pub const SHUFFLE: &str = "\u{E422}";
    pub const STACK: &str = "\u{E466}";
    pub const LIST_CHECKS: &str = "\u{EADC}";
    pub const GRADUATION_CAP: &str = "\u{E62C}";
    pub const TIMER: &str = "\u{E492}";
    pub const TRAY: &str = "\u{E4AA}";
    pub const EXAM: &str = "\u{E742}";
    pub const NOTE_PENCIL: &str = "\u{E34C}";
    pub const CLOCK_COUNTER_CLOCKWISE: &str = "\u{E1A0}";
    pub const HOUSE: &str = "\u{E2C2}";
    pub const FILE_TEXT: &str = "\u{E23A}";
    pub const TARGET: &str = "\u{E47C}";
    pub const QUESTION: &str = "\u{E3E8}";
    pub const NOTEBOOK: &str = "\u{E34E}";
    pub const HOUSE_LINE: &str = "\u{E2C4}";
    pub const PENCIL_LINE: &str = "\u{E3B2}";
}

pub fn load_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter_regular".into(),
        FontData::from_static(INTER_REGULAR).into(),
    );
    fonts.font_data.insert(
        "inter_semibold".into(),
        FontData::from_static(INTER_SEMIBOLD).into(),
    );
    fonts
        .font_data
        .insert("inter_bold".into(), FontData::from_static(INTER_BOLD).into());
    fonts
        .font_data
        .insert("phosphor".into(), FontData::from_static(PHOSPHOR).into());
    let prop = fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default();
    prop.insert(0, "inter_regular".into());
    fonts.families.insert(
        FontFamily::Name(FAMILY_ICONS.into()),
        vec!["phosphor".into()],
    );
    fonts.families.insert(
        FontFamily::Name(FAMILY_SEMI.into()),
        vec!["inter_semibold".into()],
    );
    fonts.families.insert(
        FontFamily::Name(FAMILY_BOLD.into()),
        vec!["inter_bold".into()],
    );
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &Context) {
    let mut vis = egui::Visuals::dark();
    vis.override_text_color = Some(TEXT);
    vis.panel_fill = Color32::TRANSPARENT;
    vis.window_fill = CARD_BG;
    vis.extreme_bg_color = DEEP_BG;
    vis.faint_bg_color = FAINT_BG;
    vis.hyperlink_color = ACCENT;
    vis.selection.bg_fill = SELECT_BG;
    vis.selection.stroke = Stroke::new(1.0, ACCENT);
    vis.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, 130),
    };
    vis.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, 110),
    };
    vis.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    vis.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    vis.widgets.inactive.bg_fill = CHIP_BG;
    vis.widgets.inactive.weak_bg_fill = CHIP_BG;
    vis.widgets.inactive.fg_stroke = Stroke::new(1.0, MUTED);
    vis.widgets.hovered.bg_fill = CHIP_HOVER;
    vis.widgets.hovered.weak_bg_fill = CHIP_HOVER;
    vis.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    vis.widgets.active.bg_fill = CHIP_ACTIVE;
    vis.widgets.active.weak_bg_fill = CHIP_ACTIVE;
    vis.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
    vis.widgets.inactive.corner_radius = CornerRadius::same(8);
    vis.widgets.hovered.corner_radius = CornerRadius::same(8);
    vis.widgets.active.corner_radius = CornerRadius::same(8);
    ctx.set_visuals(vis);

    ctx.all_styles_mut(|s| {
        use egui::TextStyle;
        s.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, family(FAMILY_SEMI))),
            (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(16.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(13.0, FontFamily::Proportional)),
            (
                TextStyle::Monospace,
                FontId::new(15.0, FontFamily::Monospace),
            ),
        ]
        .into();
        s.spacing.item_spacing = Vec2::new(10.0, 8.0);
        s.spacing.button_padding = Vec2::new(14.0, 8.0);
        s.spacing.icon_width = 22.0;
        s.spacing.icon_width_inner = 16.0;
        s.spacing.interact_size = Vec2::new(40.0, 32.0);
        s.spacing.scroll.bar_width = 8.0;
        s.spacing.scroll.bar_inner_margin = 3.0;
        s.spacing.scroll.bar_outer_margin = 2.0;
        s.spacing.scroll.floating = false;
    });
}

fn family(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

pub fn semi_family() -> FontFamily {
    FontFamily::Name(FAMILY_SEMI.into())
}

#[allow(dead_code)]
pub fn bold_family() -> FontFamily {
    FontFamily::Name(FAMILY_BOLD.into())
}

pub fn t(s: impl Into<String>, size: f32) -> RichText {
    RichText::new(s).font(FontId::new(size, FontFamily::Proportional))
}

pub fn semi(s: impl Into<String>, size: f32) -> RichText {
    RichText::new(s).font(FontId::new(size, family(FAMILY_SEMI)))
}

pub fn bold(s: impl Into<String>, size: f32) -> RichText {
    RichText::new(s).font(FontId::new(size, family(FAMILY_BOLD)))
}

pub fn icon(glyph: &str, size: f32, color: Color32) -> RichText {
    RichText::new(glyph)
        .font(FontId::new(size, FontFamily::Name(FAMILY_ICONS.into())))
        .color(color)
}

pub fn icon_id(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY_ICONS.into()))
}

pub fn mix(a: Color32, b: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    let (r1, g1, b1, a1) = (a.r(), a.g(), a.b(), a.a());
    let (r2, g2, b2, a2) = (b.r(), b.g(), b.b(), b.a());
    Color32::from_rgba_unmultiplied(
        (r1 as f32 + (r2 as f32 - r1 as f32) * k) as u8,
        (g1 as f32 + (g2 as f32 - g1 as f32) * k) as u8,
        (b1 as f32 + (b2 as f32 - b1 as f32) * k) as u8,
        (a1 as f32 + (a2 as f32 - a1 as f32) * k) as u8,
    )
}

pub fn hover_t(ctx: &Context, id: impl std::hash::Hash + std::fmt::Debug, hovered: bool) -> f32 {
    ctx.animate_bool_with_time(egui::Id::unique(id), hovered, 0.12)
}

pub fn card_frame(hover_k: f32) -> Frame {
    Frame::new()
        .fill(mix(CARD_BG, CARD_HOVER, hover_k))
        .stroke(Stroke::new(
            1.0 + 0.5 * hover_k,
            mix(BORDER, ACCENT, hover_k),
        ))
        .corner_radius(12)
        .inner_margin(Margin::same(16))
        .shadow(if hover_k > 0.5 {
            HOVER_SHADOW
        } else {
            CARD_SHADOW
        })
}

pub fn chip_frame(bg: Color32, stroke: Option<Stroke>) -> Frame {
    let f = Frame::new()
        .fill(bg)
        .corner_radius(9)
        .inner_margin(Margin::symmetric(9, 4));
    match stroke {
        Some(s) => f.stroke(s),
        None => f,
    }
}

pub fn badge(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    chip_frame(bg, None).show(ui, |ui| {
        ui.label(t(text, 12.0).color(fg));
    });
}

pub fn icon_chip(ui: &mut egui::Ui, glyph: &str, size: f32, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(10)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::splat(size));
            ui.centered_and_justified(|ui| {
                ui.label(icon(glyph, size * 0.55, fg));
            });
        });
}

pub fn subject_icon(slug: &str) -> &'static str {
    match slug {
        "mathematics" => ph::MATH_OPERATIONS,
        "ukrainian" | "ukrmova" => ph::BOOK_OPEN,
        "ukrlit" => ph::BOOK,
        "ukraine-history" => ph::SCROLL,
        "physics" => ph::ATOM,
        "chemistry" => ph::FLASK,
        "biology" => ph::PLANT,
        "geography" => ph::GLOBE,
        "english" | "german" | "french" | "spanish" => ph::TRANSLATE,
        _ => ph::GRADUATION_CAP,
    }
}
