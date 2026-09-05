use super::css::minify_css;
use super::js::{ScriptMode, minify_js};
use super::template::{block_keyword, contains_askama_template, try_push_askama_template};
use super::util::{is_html_whitespace, trim_trailing_whitespace};

pub(crate) fn minify_html(content: &str) -> String {
    process_html(content, HtmlMode::Document).0
}

pub(crate) fn minify_html_fragment(content: &str) -> String {
    process_html(content, HtmlMode::Fragment).0
}

pub(crate) fn has_sensitive_template_context(content: &str) -> bool {
    process_html(content, HtmlMode::Inspect).1
}

enum HtmlMode {
    Document,
    Fragment,
    Inspect,
}

fn process_html(content: &str, mode: HtmlMode) -> (String, bool) {
    let preserve_edges = !matches!(mode, HtmlMode::Document);
    let inspect = matches!(mode, HtmlMode::Inspect);
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut in_pre = false;
    let mut in_textarea = false;
    let mut attr_quote = None;
    let mut last_was_space = false;
    let mut tag_name = String::new();
    let mut script_content = String::new();
    let mut style_content = String::new();
    let mut script_tag_start = 0;
    let mut style_tag_start = 0;
    let mut script_mode = Some(ScriptMode::Global);
    let mut should_minify_style = true;

    while let Some(ch) = chars.next() {
        if !in_tag && (in_script || in_style) {
            if ch == '<' {
                let is_closing_tag = (in_script && starts_with_closing_tag(&chars, "script"))
                    || (in_style && starts_with_closing_tag(&chars, "style"));

                if !is_closing_tag && in_script {
                    script_content.push(ch);
                    last_was_space = false;
                    continue;
                } else if !is_closing_tag {
                    style_content.push(ch);
                    last_was_space = false;
                    continue;
                }
            } else {
                let target = if in_script {
                    &mut script_content
                } else {
                    &mut style_content
                };

                let template_start = target.len();
                if try_push_askama_template(ch, &mut chars, target).is_some() {
                    if inspect && may_insert_fragment(&target[template_start..]) {
                        return (String::new(), true);
                    }
                    last_was_space = false;
                    continue;
                }

                target.push(ch);
                last_was_space = false;
                continue;
            }
        }

        if in_textarea && ch == '<' && !starts_with_closing_tag(&chars, "textarea") {
            result.push(ch);
            last_was_space = false;
            continue;
        }

        let template_start = result.len();
        if try_push_askama_template(ch, &mut chars, &mut result).is_some() {
            let template = &result[template_start..];
            // Raw markup can change the surrounding HTML state across the block boundary.
            if block_keyword(template) == "raw" && (in_tag || template.contains('<')) {
                return if inspect {
                    (String::new(), true)
                } else {
                    (content.to_owned(), false)
                };
            }
            if inspect && (in_tag || in_pre || in_textarea) && may_insert_fragment(template) {
                return (String::new(), true);
            }
            last_was_space = false;
            continue;
        }

        if !in_tag && !in_script && !in_style && ch == '<' && starts_with_html_comment(&chars) {
            chars.next();
            chars.next();
            chars.next();

            let mut trailing_dashes = 0;
            for c in chars.by_ref() {
                if c == '>' && trailing_dashes >= 2 {
                    break;
                }
                trailing_dashes = if c == '-' { trailing_dashes + 1 } else { 0 };
            }
            last_was_space = result.ends_with(' ');
            continue;
        }

        if in_tag {
            if let Some(quote) = attr_quote {
                result.push(ch);
                if ch == quote {
                    attr_quote = None;
                }
                last_was_space = false;
                continue;
            }

            if ch == '"' || ch == '\'' {
                attr_quote = Some(ch);
                result.push(ch);
                last_was_space = false;
                continue;
            }

            if ch == '>' {
                in_tag = false;
                result.push(ch);
                if tag_name.eq_ignore_ascii_case("script") {
                    script_mode = script_tag_minification_mode(&result[script_tag_start..]);
                } else if tag_name.eq_ignore_ascii_case("style") {
                    should_minify_style = style_tag_should_be_minified(&result[style_tag_start..]);
                }
                last_was_space = false;
                continue;
            }

            if is_html_whitespace(ch) {
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            } else {
                result.push(ch);
                last_was_space = false;
            }
            continue;
        }

        if ch == '<' {
            in_tag = true;
            attr_quote = None;
            tag_name.clear();
            let tag_start = result.len();
            result.push(ch);
            last_was_space = false;

            while let Some(next_ch) = chars.clone().next() {
                if is_html_whitespace(next_ch) || next_ch == '>' {
                    break;
                }
                if next_ch == '/' && !tag_name.is_empty() {
                    break;
                }
                tag_name.push(chars.next().unwrap());
            }

            if tag_name.eq_ignore_ascii_case("/script") {
                if inspect && !in_script {
                    return (String::new(), true);
                }
                result.pop();
                if !script_content.is_empty() {
                    if let Some(mode) = script_mode.filter(|_| !inspect) {
                        result.push_str(&minify_js(&script_content, mode));
                    } else {
                        result.push_str(&script_content);
                    }
                }
                script_content.clear();
                in_script = false;
                result.push('<');
            } else if tag_name.eq_ignore_ascii_case("/style") {
                if inspect && !in_style {
                    return (String::new(), true);
                }
                result.pop();
                if !style_content.is_empty() {
                    if should_minify_style && !inspect {
                        result.push_str(&minify_css(&style_content));
                    } else {
                        result.push_str(&style_content);
                    }
                }
                style_content.clear();
                in_style = false;
                result.push('<');
            } else if tag_name.eq_ignore_ascii_case("/pre") {
                if inspect && !in_pre {
                    return (String::new(), true);
                }
                in_pre = false;
            } else if tag_name.eq_ignore_ascii_case("/textarea") {
                if inspect && !in_textarea {
                    return (String::new(), true);
                }
                in_textarea = false;
            }

            result.push_str(&tag_name);

            if tag_name.eq_ignore_ascii_case("script") {
                in_script = true;
                script_content.clear();
                script_tag_start = tag_start;
                script_mode = Some(ScriptMode::Global);
            } else if tag_name.eq_ignore_ascii_case("style") {
                in_style = true;
                style_content.clear();
                style_tag_start = tag_start;
                should_minify_style = true;
            } else if tag_name.eq_ignore_ascii_case("pre") {
                in_pre = true;
            } else if tag_name.eq_ignore_ascii_case("textarea") {
                in_textarea = true;
            }
            continue;
        }

        if in_pre || in_textarea {
            result.push(ch);
            last_was_space = false;
        } else if is_html_whitespace(ch) {
            if !last_was_space && (preserve_edges || !result.is_empty()) {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    if inspect {
        return (
            String::new(),
            in_tag || in_pre || in_textarea || in_script || in_style,
        );
    }

    if in_script {
        if let Some(mode) = script_mode {
            result.push_str(&minify_js(&script_content, mode));
        } else {
            result.push_str(&script_content);
        }
    } else if in_style {
        if should_minify_style {
            result.push_str(&minify_css(&style_content));
        } else {
            result.push_str(&style_content);
        }
    }

    if !preserve_edges && !in_pre && !in_textarea && !in_script && !in_style && !in_tag {
        trim_trailing_whitespace(&mut result);
    }
    (result, false)
}

fn may_insert_fragment(template: &str) -> bool {
    matches!(block_keyword(template), "include" | "block" | "call")
        || (template.starts_with("{{") && template.contains('('))
}

fn script_tag_minification_mode(tag: &str) -> Option<ScriptMode> {
    if contains_askama_template(tag) {
        return None;
    }
    let Some(script_type) = attribute_value(tag, "type") else {
        return Some(ScriptMode::Global);
    };
    let script_type = script_type.trim_matches(is_html_whitespace);

    if script_type.eq_ignore_ascii_case("module") {
        Some(ScriptMode::Module)
    } else if script_type.is_empty()
        || contains_ignore_ascii_case(script_type, "javascript")
        || contains_ignore_ascii_case(script_type, "ecmascript")
        || script_type.eq_ignore_ascii_case("text/jscript")
        || script_type.eq_ignore_ascii_case("text/livescript")
    {
        Some(ScriptMode::Global)
    } else {
        None
    }
}

fn contains_ignore_ascii_case(value: &str, needle: &str) -> bool {
    value
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn style_tag_should_be_minified(tag: &str) -> bool {
    if contains_askama_template(tag) {
        return false;
    }
    attribute_value(tag, "type").is_none_or(|value| {
        let value = value.trim_matches(is_html_whitespace);
        value.is_empty() || value.eq_ignore_ascii_case("text/css")
    })
}

fn attribute_value<'a>(tag: &'a str, target: &str) -> Option<&'a str> {
    let bytes = tag.as_bytes();
    let mut cursor = 1;

    while cursor < bytes.len()
        && !is_html_whitespace(bytes[cursor] as char)
        && !matches!(bytes[cursor], b'/' | b'>')
    {
        cursor += 1;
    }

    while cursor < bytes.len() {
        while cursor < bytes.len() && is_html_whitespace(bytes[cursor] as char) {
            cursor += 1;
        }
        if cursor >= bytes.len() || matches!(bytes[cursor], b'/' | b'>') {
            break;
        }

        let name_start = cursor;
        while cursor < bytes.len()
            && !is_html_whitespace(bytes[cursor] as char)
            && !matches!(bytes[cursor], b'=' | b'/' | b'>')
        {
            cursor += 1;
        }
        let name = &tag[name_start..cursor];
        while cursor < bytes.len() && is_html_whitespace(bytes[cursor] as char) {
            cursor += 1;
        }

        let (value_start, value_end) = if cursor < bytes.len() && bytes[cursor] == b'=' {
            cursor += 1;
            while cursor < bytes.len() && is_html_whitespace(bytes[cursor] as char) {
                cursor += 1;
            }
            if cursor < bytes.len() && matches!(bytes[cursor], b'"' | b'\'') {
                let quote = bytes[cursor];
                cursor += 1;
                let start = cursor;
                while cursor < bytes.len() && bytes[cursor] != quote {
                    cursor += 1;
                }
                let end = cursor;
                cursor += usize::from(cursor < bytes.len());
                (start, end)
            } else {
                let start = cursor;
                while cursor < bytes.len()
                    && !is_html_whitespace(bytes[cursor] as char)
                    && bytes[cursor] != b'>'
                {
                    cursor += 1;
                }
                (start, cursor)
            }
        } else {
            (cursor, cursor)
        };

        if name.eq_ignore_ascii_case(target) {
            return Some(&tag[value_start..value_end]);
        }
    }
    None
}

fn starts_with_html_comment(chars: &std::str::Chars<'_>) -> bool {
    chars.as_str().starts_with("!--")
}

fn starts_with_closing_tag(chars: &std::str::Chars<'_>, tag: &str) -> bool {
    let mut lookahead = chars.clone();

    if lookahead.next() != Some('/') {
        return false;
    }

    for expected in tag.chars() {
        let Some(actual) = lookahead.next() else {
            return false;
        };

        if !actual.eq_ignore_ascii_case(&expected) {
            return false;
        }
    }

    matches!(lookahead.next(), Some(ch) if is_html_whitespace(ch) || matches!(ch, '/' | '>'))
}
