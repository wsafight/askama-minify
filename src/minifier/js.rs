use std::borrow::Cow;

#[cfg(feature = "js-minify")]
use super::template::contains_askama_template;
#[cfg(feature = "js-minify")]
use super::util::contains_end_tag;
#[cfg(feature = "js-minify")]
use minify_js::{Session, TopLevelMode};

#[derive(Clone, Copy)]
pub(super) enum ScriptMode {
    Global,
    Module,
}

#[cfg(feature = "js-minify")]
pub(super) fn minify_js(js_code: &str, mode: ScriptMode) -> Cow<'_, str> {
    if contains_askama_template(js_code) {
        return Cow::Borrowed(js_code);
    }

    try_minify(js_code, mode)
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(js_code))
}

#[cfg(not(feature = "js-minify"))]
pub(super) fn minify_js(js_code: &str, _mode: ScriptMode) -> Cow<'_, str> {
    Cow::Borrowed(js_code)
}

#[cfg(feature = "js-minify")]
fn try_minify(js_code: &str, mode: ScriptMode) -> Option<String> {
    let session = Session::new();
    let mut output = Vec::with_capacity(js_code.len());
    let mode = match mode {
        ScriptMode::Global => TopLevelMode::Global,
        ScriptMode::Module => TopLevelMode::Module,
    };
    minify_js::minify(&session, mode, js_code.as_bytes(), &mut output).ok()?;
    // Rewriting raw text here cannot distinguish strings from regex delimiters.
    String::from_utf8(output)
        .ok()
        .filter(|value| !contains_end_tag(value, "script"))
}
