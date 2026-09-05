pub(super) fn trim_trailing_space(value: &mut String) {
    if value.ends_with(' ') {
        value.pop();
    }
}

pub(super) fn trim_trailing_whitespace(value: &mut String) {
    while value.chars().next_back().is_some_and(is_html_whitespace) {
        value.pop();
    }
}

pub(super) fn is_html_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

#[cfg(any(feature = "advanced-css", feature = "js-minify"))]
pub(super) fn contains_end_tag(value: &str, name: &str) -> bool {
    value.as_bytes().windows(name.len() + 2).any(|window| {
        window.starts_with(b"</") && window[2..].eq_ignore_ascii_case(name.as_bytes())
    })
}
