use crate::translation::{CombinedTranslationData, ExampleItem, TranslationItem};
use eframe::egui;
use std::fmt;
use std::sync::mpsc::Receiver;

// App struct for the egui UI
pub struct OcrApp {
    pub text: String,
    pub translation_data: Option<CombinedTranslationData>,
    pub is_translating: bool,
    pub translation_rx: Receiver<CombinedTranslationData>,
    pub translation_started: bool,
    frame_count: u32,
}

impl OcrApp {
    pub fn new(text: String, translation_rx: Receiver<CombinedTranslationData>) -> Self {
        Self {
            text,
            translation_data: None,
            is_translating: true,
            translation_rx,
            translation_started: true,
            frame_count: 0,
        }
    }
}

impl fmt::Debug for OcrApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OcrApp")
            .field("text", &self.text)
            .field("translation_data", &self.translation_data)
            .field("is_translating", &self.is_translating)
            .field("translation_started", &self.translation_started)
            .field("frame_count", &self.frame_count)
            .finish()
    }
}

impl eframe::App for OcrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count += 1;

        // Check if translation is complete
        if let Ok(data) = self.translation_rx.try_recv() {
            self.translation_data = Some(data);
            self.is_translating = false;
        }

        setup_visuals(ctx);

        // Close on focus loss
        if self.frame_count > 2 {
            let is_focused = ctx.input(|i| i.focused);
            if !is_focused {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // Central Panel - measure content height
        let inner_response = egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(28, 28, 32),
                inner_margin: egui::Margin::same(16.0),
                stroke: egui::Stroke::new(0.0, egui::Color32::TRANSPARENT),
                ..Default::default()
            })
            .show(ctx, |ui| {
                if self.is_translating {
                    // Loading View
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.spinner();
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("Translating...")
                                .size(16.0)
                                .color(egui::Color32::from_gray(200)),
                        );
                        ui.add_space(40.0);
                    });
                    None
                } else if let Some(data) = &self.translation_data {
                    // Results View - use a layout to measure size
                    let layout_response = ui.vertical(|ui| {
                        // Set a max width to ensure proper wrapping
                        ui.set_max_width(500.0 - 32.0); // window width - margins

                        render_content(ui, &self.text, data);
                    });

                    Some(layout_response.response.rect.height())
                } else {
                    None
                }
            });

        // Auto-resize based on measured content
        if !self.is_translating {
            if let Some(content_height) = inner_response.inner {
                let has_resized_id = egui::Id::new("has_auto_resized");
                let already_resized =
                    ctx.memory(|m| m.data.get_temp::<bool>(has_resized_id).unwrap_or(false));

                if !already_resized && self.frame_count > 1 {
                    // Add margins (16 * 2)
                    let total_height = content_height + 32.0;

                    // Clamp between min and max
                    let min_height = 150.0;
                    let max_height = 800.0;
                    let desired_height = total_height.clamp(min_height, max_height);

                    // Get current width
                    let current_width = ctx.screen_rect().width();

                    // Resize
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        current_width,
                        desired_height,
                    )));

                    // Mark as resized
                    ctx.memory_mut(|m| {
                        m.data.insert_temp(has_resized_id, true);
                    });

                    ctx.request_repaint();
                }
            }
        }

        // Request repaint if still translating
        if self.is_translating {
            ctx.request_repaint();
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [28.0 / 255.0, 28.0 / 255.0, 32.0 / 255.0, 1.0]
    }
}

// --- Content Rendering ---

fn render_content(ui: &mut egui::Ui, text: &str, data: &CombinedTranslationData) {
    // 1. Search Term
    ui.label(
        egui::RichText::new(text)
            .size(24.0)
            .strong()
            .color(egui::Color32::WHITE),
    );
    ui.add(egui::Separator::default().spacing(6.0));

    // 2. Google Translate
    render_section_header(
        ui,
        &format!("Google ({}):", data.target_lang.to_uppercase()),
    );
    render_bullet_point(ui, &data.google_translation);
    ui.add_space(10.0);

    // 3. Longdo Dict
    if let Some(longdo) = &data.longdo_data {
        if !longdo.translations.is_empty() {
            render_section_header(ui, "Longdo Dict:");
            for item in &longdo.translations {
                render_translation_item(ui, item);
            }
            ui.add_space(10.0);
        }

        // 4. Examples
        if !longdo.examples.is_empty() {
            render_section_header(ui, "Example Sentences (Longdo):");
            for ex in longdo.examples.iter().take(2) {
                render_example_item(ui, ex, &data.source_lang, &data.target_lang);
            }
        }
    }
}

// --- UI Helper Functions ---

fn setup_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_shadow = egui::epaint::Shadow::NONE;
    visuals.panel_fill = egui::Color32::from_rgb(28, 28, 32);
    visuals.window_fill = egui::Color32::from_rgb(28, 28, 32);
    visuals.extreme_bg_color = egui::Color32::from_rgb(28, 28, 32);
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
    ui.horizontal_wrapped(|ui| {
        ui.label("•");
        ui.label(
            egui::RichText::new(format!(" {}:", source_lang.to_uppercase()))
                .italics()
                .color(egui::Color32::from_gray(180)),
        );
        ui.label(egui::RichText::new(&item.en).color(egui::Color32::from_gray(210)));
    });

    ui.horizontal_wrapped(|ui| {
        let indent = ui.style().spacing.icon_width + ui.style().spacing.item_spacing.x;
        ui.add_space(indent);
        ui.label(
            egui::RichText::new(format!("-> {}:", target_lang.to_uppercase()))
                .italics()
                .color(egui::Color32::from_gray(180)),
        );
        ui.label(egui::RichText::new(&item.th).color(egui::Color32::from_gray(230)));
    });
    ui.add_space(8.0);
}
