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
        let re = Regex::new(r"[^a-zA-Z0-9]").unwrap();
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
        Box::new(|_cc| {
            Ok(Box::new(OcrApp {
                text: ocr_text,
                translation_data: None, // Will be updated to use Option<CombinedTranslationData>
                has_gained_focus: false,
                is_translating: true,
                translation_rx: rx, // Receiver for the new data structure
                translation_started: true,
            }))
        }),
    )?;

    Ok(())
}
