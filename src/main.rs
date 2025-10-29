mod app;
mod ocr;
mod translation;

use app::OcrApp;
use eframe::egui;
use regex::Regex;
use std::sync::mpsc::channel;
use translation::{is_single_word, CombinedTranslationData};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Phase 1: Capture and OCR (Async) ---
    let mut ocr_text = ocr::capture_and_ocr().await?;
    if is_single_word(&ocr_text) {
        // For single words, remove any special characters that OCR might have picked up.
        let re = Regex::new(r"[^a-zA-Z0-9]").unwrap();
        ocr_text = re.replace_all(&ocr_text, "").to_string();
    }

    // Create a channel for sending the complex translation data
    let (tx, rx) = channel::<CombinedTranslationData>();

    // Start translating immediately in a background thread
    let text_clone = ocr_text.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let translation_data = rt.block_on(translation::translate_text(&text_clone));
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
            let clipboard = arboard::Clipboard::new().ok();
            Ok(Box::new(OcrApp {
                text: ocr_text,
                translation_data: None, // Will be updated to use Option<CombinedTranslationData>
                has_gained_focus: false,
                clipboard,
                is_translating: true,
                translation_rx: rx, // Receiver for the new data structure
                translation_started: true,
            }))
        }),
    )?;

    Ok(())
}
