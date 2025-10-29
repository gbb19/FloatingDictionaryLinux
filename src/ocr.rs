use futures_util::stream::StreamExt;
use rand::Rng;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use zbus::zvariant::{ObjectPath, Str, Value};
use zbus::Connection;

/// The main entry point for capturing and OCR'ing text.
/// It detects the current desktop environment and calls the appropriate
/// screen capture utility.
pub async fn capture_and_ocr(lang: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Detect the current desktop environment.
    let de = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();

    // 2. Call the appropriate capture function, which returns a temporary file path.
    // The `?` operator will propagate any errors, such as the user cancelling the capture.
    let image_path = if de.to_uppercase().contains("KDE") {
        capture_kde().await?
    } else {
        // Default to the Freedesktop portal method for GNOME, etc.
        capture_portal().await?
    };

    // 3. Read the image data from the file.
    let image_data = fs::read(&image_path)?;

    // 4. Clean up the temporary screenshot file immediately after reading.
    let _ = fs::remove_file(&image_path);

    // 5. Perform OCR on the image data in memory.
    let ocr_text = tesseract::Tesseract::new(None, Some(lang))?
        .set_image_from_mem(&image_data)?
        .get_text()?;

    Ok(ocr_text)
}

/// Captures a screen region using KDE's Spectacle tool.
/// This is a command-line approach that is often more reliable on KDE Plasma.
async fn capture_kde() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Generate a random file name for the temporary screenshot.
    let mut rng = rand::rng();
    let temp_file_name: String = (0..12)
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .collect();
    let temp_path = env::temp_dir().join(format!("capture_{}.png", temp_file_name));
    let temp_path_str = temp_path
        .to_str()
        .ok_or("Failed to create a temporary file path.")?;

    // Execute Spectacle in region selection mode.
    // -b: non-GUI, background mode
    // -n: no notification
    // -r: region mode
    // -o: output file
    let output = Command::new("spectacle")
        .args(["-b", "-n", "-r", "-o", temp_path_str])
        .output()?;

    if !output.status.success() {
        // This typically happens if the user presses 'Esc' to cancel the screenshot.
        return Err("Screenshot cancelled by user.".into());
    }

    Ok(temp_path)
}

/// Captures a screen region using the Freedesktop Screenshot portal (DBus).
/// This is the standard method for Wayland and works best on GNOME and other
/// non-KDE environments.
async fn capture_portal() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let connection = Connection::session().await?;

    // Generate a unique token for the portal request.
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz01234d56789";
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

    // Request the screenshot.
    let _ = proxy.call_method("Screenshot", &("", options)).await?;

    // Wait for the portal to respond with the URI of the saved file.
    let request_proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        handle,
        "org.freedesktop.portal.Request",
    )
    .await?;
    let mut signal_stream = request_proxy.receive_signal("Response").await?;
    let response_signal = signal_stream
        .next()
        .await
        .ok_or("Portal did not send a response.")?;
    let body = response_signal.body();
    let (response_code, results): (u32, HashMap<String, Value>) = body.deserialize()?;

    if response_code != 0 {
        return Err("Portal request failed or was cancelled by user.".into());
    }

    // Extract the file path from the response URI.
    let uri_value = results
        .get("uri")
        .ok_or("Portal response did not contain a URI.")?;

    // Bind the Str to a variable so it lives long enough
    let uri_str_obj = uri_value.downcast_ref::<Str>()?;
    let uri_str = uri_str_obj.as_str();

    let path_str = uri_str
        .strip_prefix("file://")
        .ok_or("URI was not a file URI.")?;
    let decoded_path = urlencoding::decode(path_str)?.into_owned();

    Ok(PathBuf::from(decoded_path))
}
