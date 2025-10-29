mod app;
mod ocr;
mod translation;

use app::OcrApp;
use clap::{Parser, ValueEnum};
use eframe::egui;
use include_dir::{include_dir, Dir};
use regex::Regex;
use std::env;
use std::fs;
use std::sync::mpsc::channel;
use translation::{is_single_word, CombinedTranslationData};

// Embed the 'tessdata' directory directly into the binary.
// This requires a `tessdata` folder in the project's root directory.
static TESS_DATA_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/tessdata");

#[derive(Clone, Debug, ValueEnum, PartialEq)]
enum OcrLang {
    Auto,
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
            OcrLang::Auto => "auto", // This isn't sent to Tesseract directly.
            OcrLang::Eng => "eng",
            OcrLang::Rus => "rus",
            OcrLang::Kor => "kor",
            OcrLang::Jpn => "jpn",
            OcrLang::ChiSim => "chi_sim",
            OcrLang::Thai => "tha", // Tesseract uses 'tha' for Thai
        }
    }

    /// Returns a list of all Tesseract-compatible language strings.
    fn all_tesseract_langs() -> Vec<&'static str> {
        vec!["eng", "rus", "kor", "jpn", "chi_sim", "tha"]
    }
}

/// A simple OCR and translation tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Language for OCR (Tesseract). 'auto' uses all available languages except the target language.
    #[arg(long, value_enum, default_value = "auto")]
    ocr_lang: OcrLang,

    /// Target language for translation
    #[arg(short, long, default_value = "th")]
    target: String,
}

/// Sets up the Tesseract data directory.
/// It extracts embedded `.traineddata` files to a user-specific data directory
/// and sets the TESSDATA_PREFIX environment variable so Tesseract can find them.
fn setup_tessdata() -> Result<(), Box<dyn std::error::Error>> {
    // Find a suitable directory to store our data files (e.g., ~/.local/share on Linux).
    let data_dir = dirs::data_dir().ok_or("Could not find a valid data directory.")?;
    let app_data_dir = data_dir.join("floating-dictionary-linux");
    let tessdata_path = app_data_dir.join("tessdata");

    // If the directory doesn't exist, create it and extract the embedded files.
    if !tessdata_path.exists() {
        fs::create_dir_all(&tessdata_path)?;

        for file in TESS_DATA_DIR.files() {
            let file_path = tessdata_path.join(file.path());
            fs::write(file_path, file.contents())?;
        }
    }

    // Tell Tesseract where to find the data files.
    // The TESSDATA_PREFIX variable should point to the directory
    // containing the `.traineddata` files directly.
    env::set_var("TESSDATA_PREFIX", &tessdata_path);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Phase 0: Setup ---
    // Ensure Tesseract data files are available and the environment is configured.
    setup_tessdata()?;

    let args = Args::parse();

    // --- OCR Language Selection Logic ---
    let ocr_lang_str = if args.ocr_lang == OcrLang::Auto {
        let all_langs = OcrLang::all_tesseract_langs();
        let target_tess_lang = match args.target.as_str() {
            "th" => "tha",
            "en" => "eng",
            "ru" => "rus",
            "ko" => "kor",
            "ja" => "jpn",
            "zh-CN" => "chi_sim",
            _ => "",
        };
        let filtered_langs: Vec<&str> = all_langs
            .into_iter()
            .filter(|&lang| lang != target_tess_lang)
            .collect();
        filtered_langs.join("+")
    } else {
        args.ocr_lang.to_tesseract_str().to_owned()
    };

    // --- Phase 1: Capture and OCR (Async) ---
    let mut ocr_text = ocr::capture_and_ocr(&ocr_lang_str).await?;
    if is_single_word(&ocr_text) {
        // For single words, trim any special characters from the start and end.
        let re = Regex::new(r"^[^a-zA-Z0-9\p{L}]+|[^a-zA-Z0-9\p{L}]+$").unwrap();
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
            .with_inner_size([500.0, 200.0])
            .with_min_inner_size([400.0, 150.0])
            .with_max_inner_size([800.0, 800.0])
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
            let font_family_list = vec![
                "noto_sans".to_owned(),
                "noto_sans_thai".to_owned(),
                "noto_sans_jp".to_owned(),
                "noto_sans_kr".to_owned(),
                "noto_sans_sc".to_owned(),
            ];
            fonts
                .families
                .insert(egui::FontFamily::Proportional, font_family_list.clone());
            fonts
                .families
                .insert(egui::FontFamily::Monospace, font_family_list);
            cc.egui_ctx.set_fonts(fonts);
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

            // Use the new constructor for OcrApp
            Ok(Box::new(OcrApp::new(ocr_text, rx)))
        }),
    )?;

    Ok(())
}
