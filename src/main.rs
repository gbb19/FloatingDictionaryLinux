mod app;
mod ocr;
mod translation;

use app::OcrApp;
use clap::{Parser, ValueEnum};
use eframe::egui;
use regex::Regex;
use std::sync::mpsc::channel;
use translation::{is_single_word, CombinedTranslationData};

#[derive(Clone, Debug, ValueEnum)]
enum OcrLang {
    Eng,
    Rus,
    Kor,
    Jpn,
    #[value(name = "chi_sim")]
    ChiSim,
    #[value(name = "thai")]
    Thai,
}

impl OcrLang {
    /// Converts the enum variant to the string representation Tesseract expects.
    fn to_tesseract_str(&self) -> &str {
        match self {
            OcrLang::Eng => "eng",
            OcrLang::Rus => "rus",
            OcrLang::Kor => "kor",
            OcrLang::Jpn => "jpn",
            OcrLang::ChiSim => "chi_sim",
            OcrLang::Thai => "tha", // Tesseract uses 'tha' for Thai
        }
    }
}

/// A simple OCR and translation tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Language for OCR (Tesseract)
    #[arg(long, value_enum, default_value = "eng")]
    ocr_lang: OcrLang,

    /// Target language for translation
    #[arg(short, long, default_value = "th")]
    target: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // --- Phase 1: Capture and OCR (Async) ---
    let mut ocr_text = ocr::capture_and_ocr(args.ocr_lang.to_tesseract_str()).await?;
    if is_single_word(&ocr_text) {
        // For single words, remove any special characters that OCR might have picked up.
        // Keep Unicode letters to support non-latin scripts.
        let re = Regex::new(r"[^a-zA-Z0-9\p{L}]").unwrap();
        ocr_text = re.replace_all(&ocr_text, "").to_string();
    }

    // Create a channel for sending the complex translation data
    let (tx, rx) = channel::<CombinedTranslationData>();

    // Start translating immediately in a background thread
    let text_clone = ocr_text.clone();
    let target_lang = args.target;
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let translation_data = rt
            .block_on(translation::translate_text(
                &text_clone,
                "auto", // Always use auto-detection for the Google Translate source language
                &target_lang,
            ))
            .unwrap(); // Using unwrap here for simplicity, consider proper error handling
        let _ = tx.send(translation_data);
    });

    // --- Phase 2: Show Results in UI (Sync) ---
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 200.0]) // A sensible initial size before content is loaded
            .with_min_inner_size([400.0, 150.0])
            .with_max_inner_size([600.0, 800.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Floating Dictionary",
        options,
        Box::new(|cc| {
            // --- FONT & STYLE SETUP ---
            let mut fonts = egui::FontDefinitions::default();

            // 1. Load all font data from assets
            fonts.font_data.insert(
                "noto_sans".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/NotoSans-Regular.ttf")),
            );
            fonts.font_data.insert(
                "noto_sans_thai".to_owned(),
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/NotoSansThai-Regular.ttf"
                )),
            );
            fonts.font_data.insert(
                "noto_sans_jp".to_owned(),
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/NotoSansJP-Regular.ttf"
                )),
            );
            fonts.font_data.insert(
                "noto_sans_kr".to_owned(),
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/NotoSansKR-Regular.ttf"
                )),
            );
            fonts.font_data.insert(
                "noto_sans_sc".to_owned(),
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/NotoSansSC-Regular.ttf"
                )),
            );

            // 2. Create a list of font names in fallback order
            let font_family_list = vec![
                "noto_sans".to_owned(),
                "noto_sans_thai".to_owned(),
                "noto_sans_jp".to_owned(),
                "noto_sans_kr".to_owned(),
                "noto_sans_sc".to_owned(),
            ];

            // 3. Explicitly overwrite the font families to use our new font list.
            fonts
                .families
                .insert(egui::FontFamily::Proportional, font_family_list.clone());
            fonts
                .families
                .insert(egui::FontFamily::Monospace, font_family_list);

            // 4. Apply the new font configuration
            cc.egui_ctx.set_fonts(fonts);

            // 5. Configure text styles and spacing (moved from app.rs)
            let mut style = (*cc.egui_ctx.style()).clone();
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
                    egui::TextStyle::Small,
                    egui::FontId::new(12.0, egui::FontFamily::Proportional),
                ),
                (
                    egui::TextStyle::Heading,
                    egui::FontId::new(24.0, egui::FontFamily::Proportional),
                ),
            ]
            .into();
            style.spacing.item_spacing = egui::Vec2::new(6.0, 6.0);
            cc.egui_ctx.set_style(style);
            // --- END FONT & STYLE SETUP ---

            Ok(Box::new(OcrApp {
                text: ocr_text,
                translation_data: None,
                has_gained_focus: false,
                is_translating: true,
                translation_rx: rx,
                translation_started: true,
            }))
        }),
    )?;

    Ok(())
}
