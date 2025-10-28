use futures_util::stream::StreamExt;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fs;
use zbus::zvariant::{ObjectPath, Str, Value};
use zbus::Connection;

// Helper function to generate a unique filename
fn generate_filename() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("screenshot-{}.png", now.as_secs())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect to the D-Bus session bus
    let connection = Connection::session().await?;

    // 2. Generate a unique token for the request handle
    let token: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
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

    // 6. Convert URI to a file path and copy the file
    let path_str = uri_str.strip_prefix("file://").unwrap();
    let decoded_path = urlencoding::decode(path_str)?.into_owned();
    let source_path = std::path::PathBuf::from(decoded_path);

    let dest_dir = "target/monitors";
    fs::create_dir_all(dest_dir)?;
    let dest_filename = generate_filename();
    let dest_path = std::path::Path::new(dest_dir).join(dest_filename);

    fs::copy(&source_path, &dest_path)?;

    println!("Screenshot saved to: {}", dest_path.display());

    Ok(())
}
