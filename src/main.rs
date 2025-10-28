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
}

impl eframe::App for OcrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("OCR Result");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Use a mutable reference to the text for the TextEdit
                ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::multiline(&mut self.text).font(egui::TextStyle::Monospace),
                );
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
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OCR Result",
        options,
        Box::new(|_cc| Ok(Box::new(OcrApp { text: ocr_text }))),
    )?;

    Ok(())
}

async fn capture_and_ocr() -> Result<String, Box<dyn std::error::Error>> {
    // (The existing capture_and_ocr function remains unchanged)
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

    println!("Requesting screenshot via the portal...");
    println!("Please select a region in the UI that appears.");

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
    println!("Screenshot captured! URI: {}", uri_str);

    let path_str = uri_str.strip_prefix("file://").unwrap();
    let decoded_path = urlencoding::decode(path_str)?.into_owned();
    let source_path = std::path::PathBuf::from(decoded_path);

    let image_data = fs::read(&source_path)?;

    fs::remove_file(&source_path)?;

    println!(
        "Successfully loaded screenshot into memory ({} bytes).",
        image_data.len()
    );

    println!("\nPerforming OCR on the captured image...");
    let ocr_text = tesseract::Tesseract::new(None, Some("eng"))?
        .set_image_from_mem(&image_data)?
        .get_text()?;

    Ok(ocr_text)
}
