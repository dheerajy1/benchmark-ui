use js_sys::{JsString, Reflect};
use wasm_bindgen::JsValue;

use super::client::base_url;

// Where the page is, which build it is and which browser runs it, appended to `Copy details` so
// a pasted report says where it came from. It uses js-sys only, so no web-sys flag is needed.
pub fn environment_text() -> String {
    let page = browser_text(&["location", "href"]);
    let browser = browser_text(&["navigator", "userAgent"]);

    environment_lines(&page, &base_url(), &browser)
}

fn environment_lines(page: &str, api: &str, browser: &str) -> String {
    let api = if api.is_empty() { "not set" } else { api };

    format!(
        "Page: {page}\nUI version: {}\nAPI address: {api}\nBrowser: {browser}",
        env!("CARGO_PKG_VERSION")
    )
}

// A text property of `window`, for example `location.href`, "unknown" when the browser does not
// give it
fn browser_text(path: &[&str]) -> String {
    let Some(window) = web_sys::window() else {
        return "unknown".to_string();
    };

    let mut value = JsValue::from(window);

    for key in path {
        match Reflect::get(&value, &JsString::from(*key)) {
            Ok(next) => value = next,
            Err(_) => return "unknown".to_string(),
        }
    }

    value
        .as_string()
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_page_build_api_and_browser() {
        let text = environment_lines("https://x.test/compare", "https://api.test", "Firefox");

        assert!(text.starts_with("Page: https://x.test/compare\nUI version: "));
        assert!(text.contains("\nAPI address: https://api.test\n"));
        assert!(text.ends_with("Browser: Firefox"));
    }

    #[test]
    fn an_api_address_that_is_not_set_says_so() {
        assert!(environment_lines("p", "", "b").contains("API address: not set"));
    }
}
