use serde::Deserialize;
use tauri::{AppHandle, Listener, Runtime};

const SUPPORTED_EXTERNAL_URL_PREFIXES: [&str; 2] = ["http://", "https://"];
pub const OPEN_EXTERNAL_URL_EVENT: &str = "waifudex://open-external-url";

#[derive(Deserialize)]
struct OpenExternalUrlPayload {
    url: String,
}

fn is_supported_external_url(url: &str) -> bool {
    SUPPORTED_EXTERNAL_URL_PREFIXES
        .iter()
        .any(|prefix| url.starts_with(prefix))
}

fn open_external_url(url: &str) -> Result<(), String> {
    if !is_supported_external_url(&url) {
        return Err("only http and https URLs are supported".to_string());
    }

    tauri_plugin_opener::open_url(url, Option::<&str>::None)
        .map_err(|error| error.to_string())
}

fn parse_open_external_url_payload(payload: &str) -> Option<String> {
    let payload = serde_json::from_str::<OpenExternalUrlPayload>(payload).ok()?;
    is_supported_external_url(&payload.url).then_some(payload.url)
}

pub fn register_open_external_url_listener<R: Runtime>(app: &AppHandle<R>) {
    app.listen_any(OPEN_EXTERNAL_URL_EVENT, |event| {
        let Some(url) = parse_open_external_url_payload(event.payload()) else {
            eprintln!("failed to parse open external url payload: {}", event.payload());
            return;
        };

        if let Err(error) = open_external_url(&url) {
            eprintln!("failed to open external url `{url}`: {error}");
        }
    });
}
