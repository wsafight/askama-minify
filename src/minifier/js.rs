use super::template::try_push_askama_template;
use super::util::trim_trailing_whitespace;

pub(super) fn minify_js(js_code: &str) -> String {
    let mut result = String::with_capacity(js_code.len());
    let mut chars = js_code.chars().peekable();
    let mut in_string = false;
    let mut in_single_comment = false;
    let mut in_multi_comment = false;
    let mut multi_comment_had_newline = false;
    let mut string_char = '\0';
    let mut string_backslash_run = 0usize;
    let mut last_char = '\0';
    let mut last_was_space = false;

    while let Some(ch) = chars.next() {
        if let Some(last_ch) = try_push_askama_template(ch, &mut chars, &mut result) {
            last_char = last_ch;
            last_was_space = false;
            continue;
        }

        if !in_string && !in_multi_comment && ch == '/' && chars.peek() == Some(&'/') {
            in_single_comment = true;
            chars.next();
            continue;
        }

        if in_single_comment {
            if ch == '\n' {
                in_single_comment = false;
                if !last_was_space && !result.is_empty() {
                    result.push('\n');
                    last_was_space = true;
                }
            }
            continue;
        }

        if !in_string && !in_single_comment && ch == '/' && chars.peek() == Some(&'*') {
            in_multi_comment = true;
            multi_comment_had_newline = false;
            chars.next();
            continue;
        }

        if in_multi_comment {
            if ch == '\n' || ch == '\r' {
                multi_comment_had_newline = true;
            } else if ch == '*' && chars.peek() == Some(&'/') {
                in_multi_comment = false;
                chars.next();
                if multi_comment_had_newline {
                    if !last_was_space && !result.is_empty() {
                        result.push('\n');
                        last_was_space = true;
                    }
                } else if needs_js_space(last_char, chars.peek().copied()) && !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }
            continue;
        }

        if ch == '"' || ch == '\'' || ch == '`' {
            if !in_string {
                in_string = true;
                string_char = ch;
            } else if ch == string_char && string_backslash_run.is_multiple_of(2) {
                in_string = false;
            }
            result.push(ch);
            last_char = ch;
            string_backslash_run = 0;
            last_was_space = false;
            continue;
        }

        if in_string {
            result.push(ch);
            last_char = ch;
            if ch == '\\' {
                string_backslash_run += 1;
            } else {
                string_backslash_run = 0;
            }
            last_was_space = false;
            continue;
        }

        if ch.is_whitespace() {
            if ch == '\n' || ch == '\r' {
                if !last_was_space && !result.is_empty() {
                    result.push('\n');
                    last_was_space = true;
                }
            } else if needs_js_space(last_char, chars.peek().copied()) && !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_char = ch;
            last_was_space = false;
        }
    }

    trim_trailing_whitespace(&mut result);
    result
}

fn needs_js_space(previous: char, next: Option<char>) -> bool {
    matches!(previous, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$')
        && matches!(next, Some(ch) if ch.is_alphanumeric() || ch == '_' || ch == '$')
}
