use futures_util::stream::StreamExt;
use rand::Rng;
use std::collections::HashMap;
use std::fs;
use zbus::zvariant::{ObjectPath, Str, Value};
use zbus::Connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect to the D-Bus session bus
    let connection = Connection::session().await?;

    // 2. Generate a unique token for the request handle
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

    // 3. Set up the options dictionary
    let mut options: HashMap<&str, Value> = HashMap::new();
    options.insert("handle_token", Str::from(token).into());
    options.insert("interactive", true.into());

    // 4. Make the D-Bus method call to the Screenshot portal
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
    )
    .await?;

    let _ = proxy.call_method("Screenshot", &("", options)).await?;

    // The actual response comes as a signal. We must listen for it.
    let request_proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        handle,
        "org.freedesktop.portal.Request",
    )
    .await?;

    let mut signal_stream = request_proxy.receive_signal("Response").await?;
    let response_signal = signal_stream.next().await.unwrap();

    // 5. Decode the signal response
    let body = response_signal.body();
    let (response_code, results): (u32, HashMap<String, Value>) = body.deserialize()?;

    if response_code != 0 {
        return Err("Portal request failed".into());
    }

    let uri_value = results.get("uri").unwrap();
    let uri_binding = uri_value.downcast_ref::<Str>().unwrap();
    let uri_str = uri_binding.as_str();
    println!("Screenshot captured! URI: {}", uri_str);

    // 6. Decode the URI and read the image data into memory
    let path_str = uri_str.strip_prefix("file://").unwrap();
    let decoded_path = urlencoding::decode(path_str)?.into_owned();
    let source_path = std::path::PathBuf::from(decoded_path);

    let image_data = fs::read(&source_path)?;

    // 7. Clean up the temporary file created by the portal
    fs::remove_file(&source_path)?;

    println!(
        "Successfully loaded screenshot into memory ({} bytes).",
        image_data.len()
    );

    // 8. Perform OCR on the image data
    println!("\nPerforming OCR on the captured image...");
    let ocr_text = tesseract::Tesseract::new(None, Some("eng"))?
        .set_image_from_mem(&image_data)?
        .get_text()?;

    println!("\n--- OCR Result ---\n{}", ocr_text);

    Ok(())
}
