use super::{toast::Toasts, use_toasts};
use dioxus::prelude::*;
use js_sys::encode_uri_component;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CanvasRenderingContext2d, Element, HtmlAnchorElement, HtmlCanvasElement, HtmlImageElement,
    Window, XmlSerializer,
};

// Elements with this attribute (menus, buttons) are left out of the picture
const SKIP_ATTRIBUTE: &str = "data-capture-skip";

// Pixels per CSS pixel of the saved picture, 2 keeps text sharp when it is posted
const SCALE: f64 = 2.0;

// Empty border around the card in CSS pixels
const MARGIN: i32 = 16;

const XHTML: &str = "http://www.w3.org/1999/xhtml";

// Sizes of the picture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PictureSize {
    // Size of the svg in CSS pixels, the card plus its border
    width: i32,
    height: i32,
    // Size of the canvas in device pixels
    canvas_width: u32,
    canvas_height: u32,
}

fn picture_size(card_width: i32, card_height: i32) -> PictureSize {
    let width = card_width.max(1) + MARGIN * 2;
    let height = card_height.max(1) + MARGIN * 2;

    PictureSize {
        width,
        height,
        canvas_width: (f64::from(width) * SCALE).round() as u32,
        canvas_height: (f64::from(height) * SCALE).round() as u32,
    }
}

// The svg document that carries the copied card as xhtml inside a foreignObject
fn svg_document(size: PictureSize, xhtml: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><foreignObject x=\"{MARGIN}\" y=\"{MARGIN}\" width=\"{}\" height=\"{}\">{xhtml}</foreignObject></svg>",
        size.width,
        size.height,
        size.width - MARGIN * 2,
        size.height - MARGIN * 2,
    )
}

// Lower case letters and digits of a title with single dashes between the words
fn slug(title: &str) -> String {
    let mut slug = String::new();

    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }

    slug.trim_end_matches('-').to_string()
}

// File name safe for any system
pub fn file_name(title: &str) -> String {
    let slug = slug(title);

    if slug.is_empty() {
        "benchmark.png".to_string()
    } else {
        format!("benchmark-{slug}.png")
    }
}

// DOM id of a card, built from its title
pub fn card_id(title: &str) -> String {
    format!("card-{}", slug(title))
}

// Message of every step that can fail, the browser's own error says nothing a person can use
fn failed<E>(_: E) -> String {
    "The picture could not be made.".to_string()
}

// Copies every computed style of `source` and its descendants onto `copy` as inline style, so
// the copy looks the same outside of the page's style sheet
fn inline_styles(window: &Window, source: &Element, copy: &Element) -> Result<(), JsValue> {
    if let Some(style) = window.get_computed_style(source)? {
        let mut css = String::new();

        for index in 0..style.length() {
            let name = style.item(index);
            let value = style.get_property_value(&name)?;

            css.push_str(&name);
            css.push(':');
            css.push_str(&value);
            css.push(';');
        }

        // The picture is a still, nothing may animate or start from its first frame
        css.push_str("animation:none;transition:none;");
        copy.set_attribute("style", &css)?;
    }

    let mut source_child = source.first_element_child();
    let mut copy_child = copy.first_element_child();

    while let (Some(source_item), Some(copy_item)) = (source_child, copy_child) {
        source_child = source_item.next_element_sibling();
        copy_child = copy_item.next_element_sibling();

        if source_item.has_attribute(SKIP_ATTRIBUTE) {
            copy_item.remove();
        } else {
            inline_styles(window, &source_item, &copy_item)?;
        }
    }

    Ok(())
}

// Saves the element with this id as a PNG file. The element is copied with its computed
// styles into an svg image, drawn on a canvas and downloaded.
pub async fn save_as_image(element_id: &str, name: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| failed(()))?;
    let document = window.document().ok_or_else(|| failed(()))?;
    let card = document
        .get_element_by_id(element_id)
        .ok_or_else(|| failed(()))?;

    let size = picture_size(card.client_width(), card.client_height());

    let copy: Element = card
        .clone_node_with_deep(true)
        .map_err(failed)?
        .dyn_into()
        .map_err(failed)?;

    inline_styles(&window, &card, &copy).map_err(failed)?;
    copy.set_attribute("xmlns", XHTML).map_err(failed)?;

    let style = copy.get_attribute("style").unwrap_or_default();
    copy.set_attribute("style", &format!("{style}margin:0;"))
        .map_err(failed)?;

    let xhtml = XmlSerializer::new()
        .map_err(failed)?
        .serialize_to_string(&copy)
        .map_err(failed)?;
    let svg = svg_document(size, &xhtml);
    let url = format!(
        "data:image/svg+xml;charset=utf-8,{}",
        String::from(encode_uri_component(&svg))
    );

    let image = HtmlImageElement::new().map_err(failed)?;
    image.set_src(&url);
    JsFuture::from(image.decode()).await.map_err(failed)?;

    let canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .map_err(failed)?
        .dyn_into()
        .map_err(failed)?;
    canvas.set_width(size.canvas_width);
    canvas.set_height(size.canvas_height);

    let context: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(failed)?
        .ok_or_else(|| failed(()))?
        .dyn_into()
        .map_err(failed)?;

    context.set_fill_style_str("#ffffff");
    context.fill_rect(
        0.0,
        0.0,
        f64::from(size.canvas_width),
        f64::from(size.canvas_height),
    );
    context.scale(SCALE, SCALE).map_err(failed)?;
    context
        .draw_image_with_html_image_element(&image, 0.0, 0.0)
        .map_err(failed)?;

    let png = canvas.to_data_url().map_err(failed)?;

    let link: HtmlAnchorElement = document
        .create_element("a")
        .map_err(failed)?
        .dyn_into()
        .map_err(failed)?;
    link.set_href(&png);
    link.set_download(name);
    link.click();

    Ok(())
}

// Saves the picture and tells the person how it went. A failure keeps a `Try again` in the toast.
fn save_with_toast(toasts: Toasts, element_id: String, name: String) {
    spawn(async move {
        match save_as_image(&element_id, &name).await {
            Ok(()) => toasts.show("Image saved"),
            Err(message) => {
                let retry = Callback::new(move |()| {
                    save_with_toast(toasts, element_id.clone(), name.clone());
                });

                toasts.show_error(message, Some(retry));
            }
        }
    });
}

// Handler that saves an element: call it with the element's id and the file name
pub fn use_image_saver() -> Callback<(String, String)> {
    let toasts = use_toasts();

    use_callback(move |(element_id, name): (String, String)| {
        save_with_toast(toasts, element_id, name);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_file_after_the_title() {
        assert_eq!(
            file_name("Latency by percentile"),
            "benchmark-latency-by-percentile.png"
        );
        assert_eq!(
            file_name("  Memory & CPU range! "),
            "benchmark-memory-cpu-range.png"
        );
    }

    #[test]
    fn falls_back_when_the_title_has_no_letters() {
        assert_eq!(file_name("***"), "benchmark.png");
    }

    #[test]
    fn card_id_has_no_extension() {
        assert_eq!(card_id("Shipped size"), "card-shipped-size");
    }

    #[test]
    fn picture_is_the_card_plus_a_border_at_double_resolution() {
        let size = picture_size(400, 200);

        assert_eq!(size.width, 432);
        assert_eq!(size.height, 232);
        assert_eq!(size.canvas_width, 864);
        assert_eq!(size.canvas_height, 464);
    }

    #[test]
    fn svg_places_the_card_inside_the_border() {
        let svg = svg_document(picture_size(100, 50), "<div/>");

        assert!(svg.contains("x=\"16\" y=\"16\" width=\"100\" height=\"50\""));
        assert!(svg.contains("<div/></foreignObject></svg>"));
    }
}
