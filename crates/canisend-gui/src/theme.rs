use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

pub const TEAL_700: Color32 = Color32::from_rgb(15, 91, 88);
pub const TEAL_600: Color32 = Color32::from_rgb(16, 111, 106);
pub const TEAL_100: Color32 = Color32::from_rgb(213, 237, 235);
pub const AMBER_600: Color32 = Color32::from_rgb(194, 104, 10);
pub const RED_600: Color32 = Color32::from_rgb(190, 50, 50);
pub const SLATE_700: Color32 = Color32::from_rgb(65, 78, 85);
pub const SLATE_300: Color32 = Color32::from_rgb(195, 204, 208);
pub const SLATE_100: Color32 = Color32::from_rgb(239, 243, 244);
pub const DARK_PANEL: Color32 = Color32::from_rgb(22, 31, 34);

const DARK_TEXT: Color32 = Color32::from_rgb(229, 237, 239);
const BUTTON_INK: Color32 = Color32::from_rgb(16, 20, 23);
const DARK_POSITIVE: Color32 = Color32::from_rgb(153, 246, 228);
const DARK_WARNING: Color32 = Color32::from_rgb(252, 211, 77);
const DARK_ERROR: Color32 = Color32::from_rgb(252, 165, 165);
const DARK_NEUTRAL: Color32 = Color32::from_rgb(203, 213, 225);
const DARK_INFO: Color32 = Color32::from_rgb(147, 197, 253);
const LIGHT_WARNING: Color32 = Color32::from_rgb(154, 79, 0);
const LIGHT_INFO: Color32 = Color32::from_rgb(29, 78, 216);

pub fn apply(ctx: &egui::Context, dark: bool, compact: bool, reduce_motion: bool) {
    ctx.set_theme(egui::Theme::from_dark_mode(dark));
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    visuals.selection.bg_fill = TEAL_600;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.hyperlink_color = if dark { TEAL_100 } else { TEAL_700 };
    visuals.panel_fill = panel_background(dark);
    visuals.window_fill = if dark {
        Color32::from_rgb(29, 40, 44)
    } else {
        Color32::WHITE
    };
    visuals.faint_bg_color = if dark {
        Color32::from_rgb(36, 48, 52)
    } else {
        SLATE_100
    };
    visuals.widgets.active.bg_fill = TEAL_700;
    visuals.widgets.active.bg_stroke = Stroke::new(2.0, AMBER_600);
    visuals.widgets.active.expansion = 1.0;
    let text_color = if dark { DARK_TEXT } else { SLATE_700 };
    visuals.widgets.noninteractive.fg_stroke.color = text_color;
    visuals.widgets.inactive.fg_stroke.color = text_color;
    visuals.widgets.hovered.fg_stroke.color = text_color;
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    visuals.widgets.open.fg_stroke.color = text_color;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, TEAL_600);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(5);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(5);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.window_shadow = Default::default();
    ctx.set_visuals(visuals);

    ctx.global_style_mut(|style| {
        style.animation_time = if reduce_motion { 0.0 } else { 0.2 };
        style.scroll_animation = if reduce_motion {
            egui::style::ScrollAnimation::none()
        } else {
            egui::style::ScrollAnimation::default()
        };
        style.spacing.item_spacing = if compact {
            egui::vec2(6.0, 6.0)
        } else {
            egui::vec2(8.0, 10.0)
        };
        style.spacing.button_padding = if compact {
            egui::vec2(9.0, 5.0)
        } else {
            egui::vec2(12.0, 7.0)
        };
        style.spacing.interact_size.y = if compact { 32.0 } else { 36.0 };
    });
}

#[must_use]
pub fn panel_background(dark: bool) -> Color32 {
    if dark {
        DARK_PANEL
    } else {
        Color32::from_rgb(248, 250, 250)
    }
}

#[must_use]
pub fn positive(dark: bool) -> Color32 {
    if dark { DARK_POSITIVE } else { TEAL_700 }
}

#[must_use]
pub fn warning(dark: bool) -> Color32 {
    if dark { DARK_WARNING } else { LIGHT_WARNING }
}

#[must_use]
pub fn error(dark: bool) -> Color32 {
    if dark { DARK_ERROR } else { RED_600 }
}

#[must_use]
pub fn neutral(dark: bool) -> Color32 {
    if dark { DARK_NEUTRAL } else { SLATE_700 }
}

#[must_use]
pub fn info(dark: bool) -> Color32 {
    if dark { DARK_INFO } else { LIGHT_INFO }
}

pub fn primary_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.into()).color(Color32::WHITE)).fill(TEAL_700)
}

pub fn next_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.into()).color(BUTTON_INK)).fill(AMBER_600)
}

pub fn destructive_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.into()).color(Color32::WHITE)).fill(RED_600)
}

#[cfg(test)]
mod tests {
    use eframe::egui::{self, Color32};

    use super::{
        AMBER_600, BUTTON_INK, DARK_PANEL, RED_600, TEAL_700, apply, error, info, neutral,
        panel_background, positive, warning,
    };

    #[test]
    fn semantic_text_and_button_pairs_meet_normal_text_contrast() {
        for (foreground, background) in [
            (Color32::WHITE, TEAL_700),
            (Color32::WHITE, RED_600),
            (BUTTON_INK, AMBER_600),
            (positive(true), DARK_PANEL),
            (warning(true), DARK_PANEL),
            (error(true), DARK_PANEL),
            (neutral(true), DARK_PANEL),
            (info(true), DARK_PANEL),
            (positive(false), panel_background(false)),
            (warning(false), panel_background(false)),
            (error(false), panel_background(false)),
            (neutral(false), panel_background(false)),
            (info(false), panel_background(false)),
        ] {
            assert!(
                contrast_ratio(foreground, background) >= 4.5,
                "{foreground:?} on {background:?} is below WCAG AA"
            );
        }
    }

    #[test]
    fn accessibility_style_has_visible_focus_and_motion_free_mode() {
        let context = egui::Context::default();
        apply(&context, false, false, true);
        let style = context.global_style();
        assert_eq!(style.animation_time, 0.0);
        assert_eq!(style.scroll_animation, egui::style::ScrollAnimation::none());
        assert_eq!(style.visuals.widgets.active.bg_stroke.width, 2.0);
        assert_eq!(style.visuals.widgets.active.bg_stroke.color, AMBER_600);
        assert!(style.spacing.interact_size.y >= 36.0);

        apply(&context, true, true, false);
        let style = context.global_style();
        assert_eq!(style.animation_time, 0.2);
        assert_eq!(
            style.scroll_animation,
            egui::style::ScrollAnimation::default()
        );
        assert!(style.spacing.interact_size.y >= 32.0);
    }

    fn contrast_ratio(left: Color32, right: Color32) -> f32 {
        let (lighter, darker) = {
            let left = luminance(left);
            let right = luminance(right);
            (left.max(right), left.min(right))
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn luminance(color: Color32) -> f32 {
        let channel = |value: u8| {
            let value = f32::from(value) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }
}
