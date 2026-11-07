use js_sys::{Array, Function, JsString, Reflect};

// Copies text with `navigator.clipboard.writeText`. The call is made without waiting for the
// browser's answer, so a refused copy is not reported. It uses js-sys only, the Clipboard
// type of web-sys is an unstable API that needs a build flag.
pub fn copy_text(text: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(navigator) = Reflect::get(&window, &JsString::from("navigator")) else {
        return;
    };
    let Ok(clipboard) = Reflect::get(&navigator, &JsString::from("clipboard")) else {
        return;
    };
    let Ok(write_text) = Reflect::get(&clipboard, &JsString::from("writeText")) else {
        return;
    };

    let write_text = Function::from(write_text);
    let _ = Reflect::apply(&write_text, &clipboard, &Array::of1(&JsString::from(text)));
}
