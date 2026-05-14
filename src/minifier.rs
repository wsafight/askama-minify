use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};

/// 压缩 CSS 内容
pub(crate) fn minify_css(css_code: &str) -> String {
    let stylesheet = StyleSheet::parse(css_code, ParserOptions::default());

    match stylesheet {
        Ok(mut sheet) => {
            sheet.minify(MinifyOptions::default()).ok();
            let result = sheet.to_css(PrinterOptions {
                minify: true,
                ..PrinterOptions::default()
            });

            match result {
                Ok(output) => output.code,
                Err(_) => css_code.to_string(),
            }
        }
        Err(_) => css_code.to_string(),
    }
}

/// 压缩 JavaScript 内容
/// 使用简单的空白压缩以避免破坏代码逻辑
pub(crate) fn minify_js(js_code: &str) -> String {
    let mut result = String::with_capacity(js_code.len());
    let mut chars = js_code.chars().peekable();
    let mut in_string = false;
    let mut in_single_comment = false;
    let mut in_multi_comment = false;
    let mut multi_comment_had_newline = false;
    let mut string_char = '\0';
    let mut last_char = '\0';
    let mut last_was_space = false;

    while let Some(ch) = chars.next() {
        if try_push_askama_template(ch, &mut chars, &mut result) {
            last_char = result.chars().last().unwrap_or(last_char);
            last_was_space = false;
            continue;
        }

        // 处理单行注释
        if !in_string && !in_multi_comment && ch == '/' && chars.peek() == Some(&'/') {
            in_single_comment = true;
            chars.next(); // 跳过第二个 /
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

        // 处理多行注释
        if !in_string && !in_single_comment && ch == '/' && chars.peek() == Some(&'*') {
            in_multi_comment = true;
            multi_comment_had_newline = false;
            chars.next(); // 跳过 *
            continue;
        }

        if in_multi_comment {
            if ch == '\n' || ch == '\r' {
                multi_comment_had_newline = true;
            } else if ch == '*' && chars.peek() == Some(&'/') {
                in_multi_comment = false;
                chars.next(); // 跳过 /
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

        // 处理字符串
        if ch == '"' || ch == '\'' || ch == '`' {
            if !in_string {
                in_string = true;
                string_char = ch;
            } else if ch == string_char {
                let backslash_count = trailing_backslash_count(&result);
                // 偶数个反斜杠（包括0个）意味着引号没有被转义
                if backslash_count.is_multiple_of(2) {
                    in_string = false;
                }
            }
            result.push(ch);
            last_char = ch;
            last_was_space = false;
            continue;
        }

        if in_string {
            result.push(ch);
            last_char = ch;
            last_was_space = false;
            continue;
        }

        // 压缩空白
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

    result.trim().to_string()
}

/// 压缩 HTML 内容，保留 Askama 模板语法
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
        // 在 script 和 style 内收集内容，只让对应的结束标签回到 HTML 状态机。
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

                if try_push_askama_template(ch, &mut chars, target) {
                    last_was_space = false;
                    continue;
                }

                target.push(ch);
                last_was_space = false;
                continue;
            }
        }

        if try_push_askama_template(ch, &mut chars, &mut result) {
            last_was_space = false;
            continue;
        }

        // HTML 注释处理（只在不在 script/style 内时处理）
        if !in_script && !in_style && ch == '<' && starts_with_html_comment(&chars) {
            chars.next(); // '!'
            chars.next(); // first '-'
            chars.next(); // second '-'

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

        // 标签处理
        if ch == '<' {
            in_tag = true;
            attr_quote = None;
            tag_name.clear();
            result.push(ch);
            last_was_space = false;

            // 读取标签名
            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_whitespace() || next_ch == '>' {
                    break;
                }
                // 允许第一个字符是 / (结束标签)
                if next_ch == '/' && !tag_name.is_empty() {
                    break;
                }
                tag_name.push(chars.next().unwrap());
            }

            let tag_name_lower = tag_name.to_ascii_lowercase();

            // 检查特殊标签 - 在输出标签名之前处理结束标签的内容
            if tag_name_lower == "/script" {
                // 移除之前添加的 '<'
                result.pop();
                // 压缩并输出 script 内容
                if !script_content.trim().is_empty() {
                    let minified = minify_js(&script_content);
                    result.push_str(&minified);
                }
                script_content.clear();
                in_script = false;
                // 重新添加 '<' 开始结束标签
                result.push('<');
            } else if tag_name_lower == "/style" {
                // 移除之前添加的 '<'
                result.pop();
                // 压缩并输出 style 内容
                if !style_content.trim().is_empty() {
                    let minified = minify_css(&style_content);
                    result.push_str(&minified);
                }
                style_content.clear();
                in_style = false;
                // 重新添加 '<' 开始结束标签
                result.push('<');
            } else if tag_name_lower == "/pre" {
                in_pre = false;
            } else if tag_name_lower == "/textarea" {
                in_textarea = false;
            }

            // 现在输出标签名
            result.push_str(&tag_name);

            // 处理开始标签
            if tag_name_lower == "script" {
                in_script = true;
                script_content.clear();
            } else if tag_name_lower == "style" {
                in_style = true;
                style_content.clear();
            } else if tag_name_lower == "pre" {
                in_pre = true;
            } else if tag_name_lower == "textarea" {
                in_textarea = true;
            }
            continue;
        }

        // 在 pre 和 textarea 内完全保留原样
        if in_pre || in_textarea {
            result.push(ch);
            last_was_space = false;
        } else {
            // 标签外的内容
            if ch.is_whitespace() {
                if !last_was_space && !result.is_empty() {
                    result.push(' ');
                    last_was_space = true;
                }
            } else {
                result.push(ch);
                last_was_space = false;
            }
        }
    }

    result.trim().to_string()
}

fn try_push_askama_template(
    ch: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    target: &mut String,
) -> bool {
    if ch != '{' {
        return false;
    }

    let Some(&next_ch) = chars.peek() else {
        return false;
    };

    let end = match next_ch {
        '{' => "}}",
        '%' => "%}",
        '#' => "#}",
        _ => return false,
    };

    target.push(ch);
    for next_ch in chars.by_ref() {
        target.push(next_ch);
        if target.ends_with(end) {
            break;
        }
    }

    true
}

fn starts_with_html_comment(chars: &std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut lookahead = chars.clone();

    matches!(
        (lookahead.next(), lookahead.next(), lookahead.next()),
        (Some('!'), Some('-'), Some('-'))
    )
}

fn trailing_backslash_count(value: &str) -> usize {
    value.chars().rev().take_while(|ch| *ch == '\\').count()
}

fn needs_js_space(previous: char, next: Option<char>) -> bool {
    matches!(previous, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$')
        && matches!(next, Some(ch) if ch.is_alphanumeric() || ch == '_' || ch == '$')
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minify_css_basic() {
        let css = "body { margin: 0; padding: 0; }";
        let result = minify_css(css);
        assert!(result.contains("margin"));
        assert!(result.len() < css.len());
    }

    #[test]
    fn test_minify_js_basic() {
        let js = "function test() { return 42; }";
        let result = minify_js(js);
        assert!(result.contains("function"));
        assert!(result.len() <= js.len());
    }

    #[test]
    fn test_minify_html_preserves_template_syntax() {
        let html = "<div>{{ title }}</div>";
        let result = minify_html(html);
        assert!(result.contains("{{ title }}"));
    }

    #[test]
    fn test_minify_html_preserves_askama_blocks() {
        let html = "{% if enabled %}\n  <span>{{ value }}</span>\n{% endif %}";
        let result = minify_html(html);

        assert_eq!(
            result,
            "{% if enabled %} <span>{{ value }}</span> {% endif %}"
        );
    }

    #[test]
    fn test_minify_html_preserves_askama_comments() {
        let html = "<div>{# keep as askama comment #}{{ value }}</div>";
        let result = minify_html(html);

        assert_eq!(result, "<div>{# keep as askama comment #}{{ value }}</div>");
    }

    #[test]
    fn test_minify_html_preserves_attribute_values() {
        let html = r#"<div title="a   >   b" data-value='x   y'> text </div>"#;
        let result = minify_html(html);

        assert_eq!(
            result,
            r#"<div title="a   >   b" data-value='x   y'> text </div>"#
        );
    }

    #[test]
    fn test_minify_html_removes_whitespace() {
        let html = "<div>   <p>   text   </p>   </div>";
        let result = minify_html(html);
        assert_eq!(result, "<div> <p> text </p> </div>");
    }

    #[test]
    fn test_minify_html_removes_comments_without_extra_spaces() {
        let html = "<div> a <!-- remove me --> b </div>";
        let result = minify_html(html);

        assert_eq!(result, "<div> a b </div>");
    }

    #[test]
    fn test_minify_html_preserves_pre() {
        let html = "<pre>  code  \n  block  </pre>";
        let result = minify_html(html);
        assert!(result.contains("  code  \n  block  "));
    }

    #[test]
    fn test_minify_html_preserves_textarea() {
        let html = "<textarea>  value  \n  next  </textarea>";
        let result = minify_html(html);

        assert!(result.contains("  value  \n  next  "));
    }

    #[test]
    fn test_minify_html_minifies_style_content() {
        let html = "<style>/* removed */ body { margin: 0; color: red; }</style>";
        let result = minify_html(html);

        assert_eq!(result, "<style>body{color:red;margin:0}</style>");
    }

    #[test]
    fn test_minify_html_minifies_script_content() {
        let html = "<script>// removed\nconst value = 1;</script>";
        let result = minify_html(html);

        assert!(!result.contains("removed"));
        assert!(result.contains("const value=1;"));
    }

    #[test]
    fn test_minify_html_preserves_askama_inside_script() {
        let html = r#"<script>const value = {{ value }};</script>"#;
        let result = minify_html(html);

        assert_eq!(result, r#"<script>const value={{ value }};</script>"#);
    }

    #[test]
    fn test_minify_html_preserves_askama_inside_style() {
        let html = r#"<style>.box { color: {{ color }}; }</style>"#;
        let result = minify_html(html);

        assert_eq!(result, r#"<style>.box { color: {{ color }}; }</style>"#);
    }

    #[test]
    fn test_minify_html_keeps_html_like_strings_in_script() {
        let html = r#"<script>const value = "</div>"; const tag = "</style>";</script>"#;
        let result = minify_html(html);

        assert_eq!(
            result,
            r#"<script>const value="</div>";const tag="</style>";</script>"#
        );
    }

    #[test]
    fn test_minify_html_keeps_html_like_strings_in_style() {
        let html = r#"<style>.icon::before { content: "</script>"; }</style>"#;
        let result = minify_html(html);

        assert_eq!(
            result,
            r#"<style>.icon:before{content:"</script>"}</style>"#
        );
    }

    #[test]
    fn test_minify_js_preserves_comment_markers_inside_strings() {
        let js = r#"const a = "// not a comment"; const b = "/* also not */";"#;
        let result = minify_js(js);

        assert!(result.contains(r#""// not a comment""#));
        assert!(result.contains(r#""/* also not */""#));
    }

    #[test]
    fn test_minify_js_preserves_line_terminators() {
        let js = "function value() { return\n1; }\nconst a = b\n++c;";
        let result = minify_js(js);

        assert!(result.contains("return\n1"));
        assert!(result.contains("b\n++c"));
    }

    #[test]
    fn test_minify_js_preserves_line_terminators_from_comments() {
        let js = "function value() { return/* keep line */\n1; }\nconst a = b// keep line\n++c;";
        let result = minify_js(js);

        assert!(result.contains("return\n1"));
        assert!(result.contains("b\n++c"));
    }
}
