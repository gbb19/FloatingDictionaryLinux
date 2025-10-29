use eframe::egui;
use futures_util::stream::StreamExt;
use rand::Rng;
use regex::Regex;
use reqwest;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::fs;
use std::sync::mpsc::{channel, Receiver, Sender};
use zbus::zvariant::{ObjectPath, Str, Value};
use zbus::Connection;

// App struct for the egui UI
struct OcrApp {
    text: String,
    translation: Option<String>,
    has_gained_focus: bool,
    clipboard: Option<arboard::Clipboard>,
    is_translating: bool,
    translation_rx: Receiver<String>,
    translation_tx: Sender<String>,
}

impl eframe::App for OcrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ตรวจสอบว่ามีการแปลเสร็จหรือไม่
        if let Ok(translation) = self.translation_rx.try_recv() {
            self.translation = Some(translation);
            self.is_translating = false;
        }

        // --- Minimal Style Setup ---
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
                inner_margin: egui::Margin::same(16.0),
                rounding: egui::Rounding::same(12.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // พื้นที่แสดงข้อความต้นฉบับ
                    let scroll_height = if self.translation.is_some() {
                        (ui.available_height() - 55.0) / 2.0 - 20.0
                    } else {
                        ui.available_height() - 55.0
                    };

                    ui.label(
                        egui::RichText::new("Original Text")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(150, 150, 160)),
                    );
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .id_source("original_text_scroll") // เพิ่ม ID
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

                    // แสดงคำแปล (ถ้ามี)
                    if let Some(translation) = &self.translation {
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.label(
                            egui::RichText::new("Translation")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(150, 200, 255)),
                        );
                        ui.add_space(4.0);

                        egui::ScrollArea::vertical()
                            .id_source("translation_text_scroll") // เพิ่ม ID ที่แตกต่าง
                            .max_height(scroll_height)
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(translation)
                                            .size(16.0)
                                            .color(egui::Color32::from_rgb(220, 230, 255)),
                                    )
                                    .wrap()
                                    .selectable(true),
                                );
                            });
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // ปุ่มต่างๆ
                    ui.horizontal(|ui| {
                        if ui
                            .button(egui::RichText::new("📋 Copy").size(14.0))
                            .clicked()
                        {
                            if let Some(clipboard) = self.clipboard.as_mut() {
                                let _ = clipboard.set_text(self.text.clone());
                            }
                        }

                        if !self.is_translating && self.translation.is_none() {
                            if ui
                                .button(egui::RichText::new("🌐 Translate").size(14.0))
                                .clicked()
                            {
                                self.is_translating = true;
                                let text = self.text.clone();
                                let tx = self.translation_tx.clone();
                                let ctx_clone = ctx.clone();

                                std::thread::spawn(move || {
                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    let translation = rt.block_on(translate_text(&text));
                                    let _ = tx.send(translation);
                                    ctx_clone.request_repaint();
                                });
                            }
                        } else if self.is_translating {
                            ui.spinner();
                            ui.label("Translating...");
                        }
                    });

                    ui.add_space(4.0);
                });
            });

        // Request repaint ถ้ากำลังแปล
        if self.is_translating {
            ctx.request_repaint();
        }
    }
}

#[derive(Debug)]
struct LongdoResult {
    translations: Vec<Translation>,
    examples: Vec<Example>,
}

#[derive(Debug)]
struct Translation {
    word: String,
    pos: String,
    translation: String,
}

#[derive(Debug)]
struct Example {
    en: String,
    th: String,
}

// ตรวจสอบว่าเป็นคำเดี่ยวหรือประโยค
fn is_single_word(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.contains(' ') && !trimmed.contains('\n') && trimmed.len() < 50
}

// แปลภาษาโดยใช้ Longdo หรือ Google Translate
async fn translate_text(text: &str) -> String {
    let trimmed = text.trim();

    if is_single_word(trimmed) {
        // ใช้ Longdo สำหรับคำเดี่ยว
        match fetch_longdo_translation(trimmed).await {
            Ok(result) => format_longdo_result(&result),
            Err(e) => format!("Translation error: {}", e),
        }
    } else {
        // ใช้ Google Translate สำหรับประโยค
        match google_translate(trimmed, "th", "en").await {
            Ok(translation) => translation,
            Err(e) => format!("Translation error: {}", e),
        }
    }
}

// Fetch Longdo translation
async fn fetch_longdo_translation(word: &str) -> Result<LongdoResult, Box<dyn std::error::Error>> {
    let url = format!("https://dict.longdo.com/mobile.php?search={}", word);
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    let html = response.text().await?;
    parse_longdo_html(&html)
}

// Parse Longdo HTML
fn parse_longdo_html(html: &str) -> Result<LongdoResult, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html);
    let mut result = LongdoResult {
        translations: Vec::new(),
        examples: Vec::new(),
    };

    let target_dicts = vec!["NECTEC Lexitron Dictionary EN-TH", "Nontri Dictionary"];

    // Parse translations
    for dict_name in target_dicts {
        let b_selector = Selector::parse("b").unwrap();

        for b_element in document.select(&b_selector) {
            let text = b_element.text().collect::<String>();
            if text.contains(dict_name) {
                // Find next sibling table with class="result-table"
                let mut next = b_element.next_sibling();
                while let Some(node) = next {
                    if let Some(elem) = scraper::ElementRef::wrap(node) {
                        if elem.value().name() == "table" {
                            if let Some(class) = elem.value().attr("class") {
                                if class.contains("result-table") {
                                    parse_translation_table(&elem, &mut result);
                                    break;
                                }
                            }
                        }
                    }
                    next = node.next_sibling();
                }
            }
        }
    }

    // Parse examples
    parse_examples(&document, &mut result);

    Ok(result)
}

fn parse_translation_table(table: &scraper::ElementRef, result: &mut LongdoResult) {
    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();

    for row in table.select(&tr_selector) {
        let cells: Vec<_> = row.select(&td_selector).collect();
        if cells.len() == 2 {
            let word = cells[0].text().collect::<String>().trim().to_string();
            let definition = cells[1].text().collect::<String>().trim().to_string();

            let (pos, translation) = parse_definition(&definition);

            result.translations.push(Translation {
                word,
                pos,
                translation,
            });
        }
    }
}

fn parse_examples(document: &Html, result: &mut LongdoResult) {
    let b_selector = Selector::parse("b").unwrap();

    for b_element in document.select(&b_selector) {
        let text = b_element.text().collect::<String>();
        if text.contains("ตัวอย่างประโยคจาก Open Subtitles") {
            // Find next sibling table
            let mut next = b_element.next_sibling();
            while let Some(node) = next {
                if let Some(elem) = scraper::ElementRef::wrap(node) {
                    if elem.value().name() == "table" {
                        if let Some(class) = elem.value().attr("class") {
                            if class.contains("result-table") {
                                parse_example_table(&elem, result);
                                return;
                            }
                        }
                    }
                }
                next = node.next_sibling();
            }
        }
    }
}

fn parse_example_table(table: &scraper::ElementRef, result: &mut LongdoResult) {
    let tr_selector = Selector::parse("tr").unwrap();
    let font_selector = Selector::parse("font[color='black']").unwrap();

    for row in table.select(&tr_selector) {
        let fonts: Vec<_> = row.select(&font_selector).collect();
        if fonts.len() == 2 {
            let en = fonts[0].text().collect::<String>().trim().to_string();
            let th = fonts[1].text().collect::<String>().trim().to_string();
            result.examples.push(Example { en, th });
        }
    }
}

fn parse_definition(definition: &str) -> (String, String) {
    let re = Regex::new(r"^\s*\((.*?)\)\s*(.*)").unwrap();

    if let Some(caps) = re.captures(definition) {
        let pos = caps.get(1).map_or("N/A", |m| m.as_str()).trim().to_string();
        let translation_text = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();

        // Try to extract POS from translation
        let pos_re = Regex::new(r"^(pron|adj|det|n|v|adv|int|conj)\.?\s*(.*)").unwrap();
        if let Some(caps2) = pos_re.captures(&translation_text) {
            let extracted_pos = caps2
                .get(1)
                .map_or("", |m| m.as_str())
                .trim_end_matches('.');
            let final_translation = caps2.get(2).map_or("", |m| m.as_str()).trim().to_string();

            // Fix common OCR/scraping errors
            let fixed_translation = final_translation
                .replace("your self", "yourself")
                .replace("your selves", "yourselves");

            return (extracted_pos.to_string(), fixed_translation);
        }

        // Fix common errors in translation_text
        let fixed_translation = translation_text
            .replace("your self", "yourself")
            .replace("your selves", "yourselves");

        (pos, fixed_translation)
    } else {
        ("N/A".to_string(), definition.to_string())
    }
}

fn format_longdo_result(result: &LongdoResult) -> String {
    let mut output = String::new();

    if result.translations.is_empty() {
        return "No translation found".to_string();
    }

    for trans in &result.translations {
        output.push_str(&format!(
            "📚 {} ({}): {}\n",
            trans.word, trans.pos, trans.translation
        ));
    }

    if !result.examples.is_empty() {
        output.push_str("\n--- ตัวอย่างประโยค ---\n");
        for (i, example) in result.examples.iter().enumerate().take(3) {
            output.push_str(&format!("\n{}. {}\n   {}\n", i + 1, example.en, example.th));
        }
    }

    output
}

// Google Translate (simplified version using Google Translate API)
async fn google_translate(
    text: &str,
    target_lang: &str,
    source_lang: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        source_lang,
        target_lang,
        urlencoding::encode(text)
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let json: serde_json::Value = response.json().await?;

    if let Some(translations) = json.get(0).and_then(|v| v.as_array()) {
        let mut result = String::new();
        for item in translations {
            if let Some(text) = item.get(0).and_then(|v| v.as_str()) {
                result.push_str(text);
            }
        }
        Ok(result)
    } else {
        Err("Failed to parse translation".into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Phase 1: Capture and OCR (Async) ---
    let ocr_text = capture_and_ocr().await?;

    // สร้าง channel สำหรับส่งคำแปล
    let (tx, rx) = channel();

    // --- Phase 2: Show Results in UI (Sync) ---
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, 400.0])
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
                translation: None,
                has_gained_focus: false,
                clipboard,
                is_translating: false,
                translation_rx: rx,
                translation_tx: tx,
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
