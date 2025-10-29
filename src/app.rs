use crate::translation::{CombinedTranslationData, ExampleItem, TranslationItem};
use eframe::egui;
use std::fmt;
use std::sync::mpsc::Receiver;

// App struct for the egui UI
pub struct OcrApp {
    pub text: String,
    pub translation_data: Option<CombinedTranslationData>,
    pub has_gained_focus: bool,
    pub is_translating: bool,
    pub translation_rx: Receiver<CombinedTranslationData>,
    pub translation_started: bool,
}

// Implement Debug manually (skip clipboard field)
impl fmt::Debug for OcrApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OcrApp")
            .field("text", &self.text)
            .field("translation_data", &self.translation_data)
            .field("has_gained_focus", &self.has_gained_focus)
            .field("is_translating", &self.is_translating)
            .field("translation_started", &self.translation_started)
            .finish()
    }
}

impl eframe::App for OcrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if translation is complete
        if let Ok(data) = self.translation_rx.try_recv() {
            self.translation_data = Some(data);
            self.is_translating = false;
        }

        // Apply visual styles. Font and text styles are now set globally in main.rs
        setup_visuals(ctx);

        // --- Close on focus loss ---
        let is_focused = ctx.input(|i| i.focused);
        if !self.has_gained_focus && is_focused {
            self.has_gained_focus = true;
        }
        if self.has_gained_focus && !is_focused {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // --- Central Panel ---
        let panel_response = egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::from_rgba_premultiplied(28, 28, 32, 250),
                inner_margin: egui::Margin::same(16.0),
                rounding: egui::Rounding::same(12.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                if self.is_translating {
                    // --- Loading View ---
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() / 2.5);
                        ui.spinner();
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("Translating...")
                                .size(16.0)
                                .color(egui::Color32::from_gray(200)),
                        );
                    });
                    0.0 // Return 0.0 for height when loading
                } else if let Some(data) = &self.translation_data {
                    // --- Results View ---
                    let scroll_response = egui::ScrollArea::vertical().show(ui, |ui| {
                        // 1. Search Term (Header)
                        ui.label(
                            egui::RichText::new(&data.search_word)
                                .size(24.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.add(egui::Separator::default().spacing(6.0));

                        // 2. Google Translate Section
                        render_section_header(
                            ui,
                            &format!("Google ({}):", data.target_lang.to_uppercase()),
                        );
                        render_bullet_point(ui, &data.google_translation);
                        ui.add_space(10.0);

                        // 3. Longdo Dict Section
                        if let Some(longdo) = &data.longdo_data {
                            if !longdo.translations.is_empty() {
                                render_section_header(ui, "Longdo Dict:");
                                for item in &longdo.translations {
                                    render_translation_item(ui, item);
                                }
                                ui.add_space(10.0);
                            }

                            // 4. Examples Section
                            if !longdo.examples.is_empty() {
                                render_section_header(ui, "Example Sentences (Longdo):");
                                for ex in longdo.examples.iter().take(2) {
                                    render_example_item(
                                        ui,
                                        ex,
                                        &data.source_lang,
                                        &data.target_lang,
                                    );
                                }
                            }
                        }
                        ui.min_rect().height() // Return the full content height
                    });
                    scroll_response.inner
                } else {
                    0.0 // Should not happen, but return 0.0 as a fallback
                }
            });

        // --- Auto-resize window after translation is done ---
        let has_resized_id = egui::Id::new("has_resized");
        let already_resized = ctx
            .memory(|m| m.data.get_temp::<bool>(has_resized_id))
            .unwrap_or(false);

        if !self.is_translating && !already_resized {
            let content_height = panel_response.inner;
            // The frame has a 16.0 margin on top and bottom
            let mut desired_height = content_height + 16.0 * 2.0;
            let current_width = ctx.screen_rect().width();

            // Enforce the maximum height (should match the value in main.rs)
            let max_height = 600.0;
            desired_height = desired_height.min(max_height);

            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                current_width,
                desired_height,
            )));

            // Mark as resized to avoid doing it every frame
            ctx.memory_mut(|m| m.data.insert_temp(has_resized_id, true));
        }

        // Request repaint if still translating
        if self.is_translating {
            ctx.request_repaint();
        }
    }
}

// --- UI Helper Functions ---

fn setup_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_rounding = egui::Rounding::from(12.0);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::Vec2::new(0.0, 2.0),
        blur: 12.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(100),
    };

    visuals.panel_fill = egui::Color32::from_rgba_premultiplied(28, 28, 32, 250);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 80, 120);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 100, 150);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(70, 110, 170);

    ctx.set_visuals(visuals);
}

fn render_section_header(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title)
            .size(18.0)
            .underline()
            .strong()
            .color(egui::Color32::from_gray(220)),
    );
    ui.add_space(2.0);
}

fn render_bullet_point(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.label("•");
        ui.add(
            egui::Label::new(egui::RichText::new(text).color(egui::Color32::from_gray(240))).wrap(),
        );
    });
}

fn render_translation_item(ui: &mut egui::Ui, item: &TranslationItem) {
    ui.horizontal(|ui| {
        ui.label("•");
        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(&item.word)
                        .strong()
                        .color(egui::Color32::from_rgb(160, 220, 255)),
                );
                ui.label(
                    egui::RichText::new(format!("[{}]", item.pos))
                        .italics()
                        .color(egui::Color32::from_gray(180)),
                );
            });
            ui.label(
                egui::RichText::new(format!("{} ({})", item.translation, item.dictionary))
                    .color(egui::Color32::from_gray(230)),
            );
        });
    });
    ui.add_space(4.0);
}

fn render_example_item(
    ui: &mut egui::Ui,
    item: &ExampleItem,
    source_lang: &str,
    target_lang: &str,
) {
    // English Line
    ui.horizontal_wrapped(|ui| {
        ui.label("•");
        ui.label(
            egui::RichText::new(format!(" {}:", source_lang.to_uppercase()))
                .italics()
                .color(egui::Color32::from_gray(180)),
        );
        ui.label(egui::RichText::new(&item.en).color(egui::Color32::from_gray(210)));
    });

    // Thai Line, indented to show it's related to the line above
    ui.horizontal_wrapped(|ui| {
        // Calculate indent based on the width of the bullet point and spacing
        let indent = ui.style().spacing.icon_width + ui.style().spacing.item_spacing.x;
        ui.add_space(indent);
        ui.label(
            egui::RichText::new(format!("-> {}:", target_lang.to_uppercase()))
                .italics()
                .color(egui::Color32::from_gray(180)),
        );
        ui.label(egui::RichText::new(&item.th).color(egui::Color32::from_gray(230)));
    });
    ui.add_space(8.0); // Add a bit more space between full examples
}
