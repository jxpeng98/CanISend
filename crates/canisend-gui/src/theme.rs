use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

pub const TEAL_700: Color32 = Color32::from_rgb(15, 91, 88);
pub const TEAL_600: Color32 = Color32::from_rgb(16, 111, 106);
pub const TEAL_100: Color32 = Color32::from_rgb(213, 237, 235);
pub const AMBER_600: Color32 = Color32::from_rgb(194, 104, 10);
pub const RED_600: Color32 = Color32::from_rgb(190, 50, 50);
pub const SLATE_700: Color32 = Color32::from_rgb(65, 78, 85);
pub const SLATE_300: Color32 = Color32::from_rgb(195, 204, 208);
pub const SLATE_100: Color32 = Color32::from_rgb(239, 243, 244);

pub fn apply(ctx: &egui::Context, dark: bool, compact: bool) {
    ctx.set_theme(egui::Theme::from_dark_mode(dark));
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    visuals.selection.bg_fill = TEAL_600;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.hyperlink_color = if dark { TEAL_100 } else { TEAL_700 };
    visuals.panel_fill = if dark {
        Color32::from_rgb(22, 31, 34)
    } else {
        Color32::from_rgb(248, 250, 250)
    };
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
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, TEAL_600);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(5);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(5);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.window_shadow = Default::default();
    ctx.set_visuals(visuals);

    ctx.global_style_mut(|style| {
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
        style.spacing.interact_size.y = if compact { 28.0 } else { 34.0 };
    });
}

pub fn primary_button(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text).fill(TEAL_700)
}

pub fn next_button(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text).fill(AMBER_600)
}

pub fn destructive_button(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text).fill(RED_600)
}
