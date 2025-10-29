use eframe::egui;
use futures_util::stream::StreamExt;
use rand::Rng;
use std::collections::HashMap;
use std::fs;
use zbus::zvariant::{ObjectPath, Str, Value};
use zbus::Connection;

// App struct for the egui UI
struct OcrApp {
    text: String,
    has_gained_focus: bool,
    clipboard: Option<arboard::Clipboard>,
}

impl eframe::App for OcrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Minimal Style Setup ---
        let mut visuals = egui::Visuals::dark();
        visuals.window_rounding = egui::Rounding::from(12.0); // เพิ่มความโค้งมน
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

        // ตั้งค่าฟอนต์
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (
                egui::TextStyle::Body,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(15.0, egui::FontFamily::Monospace),
            ),
        ]
        .into();

        style.spacing.item_spacing = egui::Vec2::new(6.0, 6.0);
        style.spacing.button_padding = egui::Vec2::new(12.0, 6.0);

        ctx.set_style(style);

        // --- Close on focus loss ---
        let is_focused = ctx.input(|i| i.focused);
        if !self.has_gained_focus && is_focused {
            self.has_gained_focus = true;
        }
        if self.has_gained_focus && !is_focused {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // --- Central Panel - Minimal Design ---
        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::from_rgba_premultiplied(28, 28, 32, 250),
                inner_margin: egui::Margin::same(16.0), // เพิ่ม margin
                rounding: egui::Rounding::same(12.0),   // ขอบโค้งมน
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // พื้นที่แสดงข้อความ
                    let scroll_height = ui.available_height() - 55.0; // เพิ่มพื้นที่สำหรับปุ่ม

                    egui::ScrollArea::vertical()
                        .max_height(scroll_height)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&self.text)
                                        .size(16.0)
                                        .color(egui::Color32::from_rgb(235, 235, 240)),
                                )
                                .wrap()
                                .selectable(true),
                            );
                        });

                    ui.add_space(8.0); // เพิ่ม spacing
                    ui.separator();
                    ui.add_space(8.0); // เพิ่ม spacing

                    // ปุ่ม Copy แบบ minimal และสวย
                    if ui
                        .button(egui::RichText::new("📋 Copy").size(14.0))
                        .clicked()
                    {
                        if let Some(clipboard) = self.clipboard.as_mut() {
                            let _ = clipboard.set_text(self.text.clone());
                        }
                    }

                    ui.add_space(4.0); // เพิ่มระยะห่างจากขอบล่าง
                });
            });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Phase 1: Capture and OCR (Async) ---
    let ocr_text = capture_and_ocr().await?;

    // --- Phase 2: Show Results in UI (Sync) ---
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, 300.0])
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "OCR Result",
        options,
        Box::new(|_cc| {
            let clipboard = arboard::Clipboard::new().ok();
            Ok(Box::new(OcrApp {
                text: ocr_text,
                has_gained_focus: false,
                clipboard,
            }))
        }),
    )?;

    Ok(())
}

async fn capture_and_ocr() -> Result<String, Box<dyn std::error::Error>> {
    let connection = Connection::session().await?;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    let token: String = (0..10)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    let sender = connection
        .unique_name()
        .unwrap()
        .trim_start_matches(':')
        .replace('.', "_");
    let handle_str = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
    let handle = ObjectPath::try_from(handle_str)?;
    let mut options: HashMap<&str, Value> = HashMap::new();
    options.insert("handle_token", Str::from(token).into());
    options.insert("interactive", true.into());

    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
    )
    .await?;

    let _ = proxy.call_method("Screenshot", &("", options)).await?;

    let request_proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        handle,
        "org.freedesktop.portal.Request",
    )
    .await?;

    let mut signal_stream = request_proxy.receive_signal("Response").await?;
    let response_signal = signal_stream.next().await.unwrap();
    let body = response_signal.body();
    let (response_code, results): (u32, HashMap<String, Value>) = body.deserialize()?;

    if response_code != 0 {
        return Err("Portal request failed".into());
    }

    let uri_value = results.get("uri").unwrap();
    let uri_binding = uri_value.downcast_ref::<Str>().unwrap();
    let uri_str = uri_binding.as_str();
    let path_str = uri_str.strip_prefix("file://").unwrap();
    let decoded_path = urlencoding::decode(path_str)?.into_owned();
    let source_path = std::path::PathBuf::from(decoded_path);
    let image_data = fs::read(&source_path)?;
    fs::remove_file(&source_path)?;

    let ocr_text = tesseract::Tesseract::new(None, Some("eng"))?
        .set_image_from_mem(&image_data)?
        .get_text()?;

    Ok(ocr_text)
}
