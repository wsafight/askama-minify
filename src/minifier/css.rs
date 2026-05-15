use super::template::try_push_askama_template;
use super::util::trim_trailing_space;

#[cfg(feature = "advanced-css")]
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};

pub(super) fn minify_css(css_code: &str) -> String {
    #[cfg(feature = "advanced-css")]
    {
        if !contains_askama_template(css_code) {
            let stylesheet = StyleSheet::parse(css_code, ParserOptions::default());

            if let Ok(mut sheet) = stylesheet {
                sheet.minify(MinifyOptions::default()).ok();
                let result = sheet.to_css(PrinterOptions {
                    minify: true,
                    ..PrinterOptions::default()
                });

                if let Ok(output) = result {
                    return output.code;
                }
            }
        }
    }

    minify_css_conservative(css_code)
}

fn minify_css_conservative(css_code: &str) -> String {
    let mut result = String::with_capacity(css_code.len());
    let mut chars = css_code.chars().peekable();
    let mut in_string = false;
    let mut string_char = '\0';
    let mut escaped = false;
    let mut last_was_space = false;
    let mut last_significant_char = None;

    while let Some(ch) = chars.next() {
        if let Some(last_ch) = try_push_askama_template(ch, &mut chars, &mut result) {
            last_significant_char = Some(last_ch);
            last_was_space = false;
            continue;
        }

        if in_string {
            result.push(ch);

            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == string_char {
                in_string = false;
            }

            if !ch.is_whitespace() {
                last_significant_char = Some(ch);
            }
            last_was_space = false;
            continue;
        }

        if ch == '"' || ch == '\'' {
            in_string = true;
            string_char = ch;
            result.push(ch);
            last_significant_char = Some(ch);
            last_was_space = false;
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();

            while let Some(comment_ch) = chars.next() {
                if comment_ch == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }

            if !css_space_is_redundant_after(last_significant_char)
                && !last_was_space
                && !result.is_empty()
            {
                result.push(' ');
                last_was_space = true;
            }
            continue;
        }

        if ch.is_whitespace() {
            if !css_space_is_redundant_after(last_significant_char)
                && !last_was_space
                && !result.is_empty()
            {
                result.push(' ');
                last_was_space = true;
            }
            continue;
        }

        if matches!(ch, '{' | '}' | ';' | ',') {
            trim_trailing_space(&mut result);
            if ch == '}' && result.ends_with(';') {
                result.pop();
            }
            result.push(ch);
            last_significant_char = Some(ch);
            last_was_space = false;
            continue;
        }

        result.push(ch);
        last_significant_char = Some(ch);
        last_was_space = false;
    }

    trim_trailing_space(&mut result);
    result
}

#[cfg(feature = "advanced-css")]
fn contains_askama_template(value: &str) -> bool {
    value.contains("{{") || value.contains("{%") || value.contains("{#")
}

fn css_space_is_redundant_after(previous: Option<char>) -> bool {
    matches!(previous, Some('{' | ':' | ';' | ',' | '('))
}
