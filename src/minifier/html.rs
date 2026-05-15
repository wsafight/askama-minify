use super::css::minify_css;
use super::js::minify_js;
use super::template::try_push_askama_template;
use super::util::trim_trailing_whitespace;

pub(crate) fn minify_html(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
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

                if try_push_askama_template(ch, &mut chars, target).is_some() {
                    last_was_space = false;
                    continue;
                }

                target.push(ch);
                last_was_space = false;
                continue;
            }
        }

        if try_push_askama_template(ch, &mut chars, &mut result).is_some() {
            last_was_space = false;
            continue;
        }

        if !in_script && !in_style && ch == '<' && starts_with_html_comment(&chars) {
            chars.next();
            chars.next();
            chars.next();

            let mut comment_end = String::new();
            for c in chars.by_ref() {
                comment_end.push(c);
                if comment_end.ends_with("-->") {
                    break;
                }

                if comment_end.len() > 3 {
                    let keep_from = comment_end.len() - 3;
                    comment_end.drain(..keep_from);
                }
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
                last_was_space = false;
                continue;
            }

            if ch.is_whitespace() {
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
            result.push(ch);
            last_was_space = false;

            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_whitespace() || next_ch == '>' {
                    break;
                }
                if next_ch == '/' && !tag_name.is_empty() {
                    break;
                }
                tag_name.push(chars.next().unwrap());
            }

            if tag_name.eq_ignore_ascii_case("/script") {
                result.pop();
                if !script_content.is_empty() {
                    let minified = minify_js(&script_content);
                    result.push_str(&minified);
                }
                script_content.clear();
                in_script = false;
                result.push('<');
            } else if tag_name.eq_ignore_ascii_case("/style") {
                result.pop();
                if !style_content.is_empty() {
                    let minified = minify_css(&style_content);
                    result.push_str(&minified);
                }
                style_content.clear();
                in_style = false;
                result.push('<');
            } else if tag_name.eq_ignore_ascii_case("/pre") {
                in_pre = false;
            } else if tag_name.eq_ignore_ascii_case("/textarea") {
                in_textarea = false;
            }

            result.push_str(&tag_name);

            if tag_name.eq_ignore_ascii_case("script") {
                in_script = true;
                script_content.clear();
            } else if tag_name.eq_ignore_ascii_case("style") {
                in_style = true;
                style_content.clear();
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
        } else if ch.is_whitespace() {
            if !last_was_space && !result.is_empty() {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    trim_trailing_whitespace(&mut result);
    result
}

fn starts_with_html_comment(chars: &std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut lookahead = chars.clone();

    matches!(
        (lookahead.next(), lookahead.next(), lookahead.next()),
        (Some('!'), Some('-'), Some('-'))
    )
}

fn starts_with_closing_tag(chars: &std::iter::Peekable<std::str::Chars<'_>>, tag: &str) -> bool {
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

    matches!(lookahead.peek(), Some(ch) if ch.is_whitespace() || *ch == '>')
}
