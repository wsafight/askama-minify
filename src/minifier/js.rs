use std::borrow::Cow;

#[cfg(feature = "js-minify")]
use super::template::contains_askama_template;
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
    String::from_utf8(output).ok().map(escape_script_end_tags)
}

#[cfg(feature = "js-minify")]
fn escape_script_end_tags(value: String) -> String {
    let Some(first_match) = find_script_end_tag(value.as_bytes(), 0) else {
        return value;
    };

    let mut result = String::with_capacity(value.len());
    let mut cursor = 0;
    let mut match_start = first_match;
    loop {
        let slash = match_start + 1;
        result.push_str(&value[cursor..slash]);
        result.push_str("\\/");
        cursor = slash + 1;
        let Some(next_match) = find_script_end_tag(value.as_bytes(), cursor) else {
            break;
        };
        match_start = next_match;
    }
    result.push_str(&value[cursor..]);
    result
}

#[cfg(feature = "js-minify")]
fn find_script_end_tag(value: &[u8], start: usize) -> Option<usize> {
    const SCRIPT_END: &[u8] = b"</script";

    value[start..]
        .windows(SCRIPT_END.len())
        .position(|window| window.eq_ignore_ascii_case(SCRIPT_END))
        .map(|offset| start + offset)
}
