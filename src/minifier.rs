use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};

/// 压缩 CSS 内容
pub fn minify_css(css_code: &str) -> String {
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
pub fn minify_js(js_code: &str) -> String {
    let mut result = String::with_capacity(js_code.len());
    let mut chars = js_code.chars().peekable();
    let mut in_string = false;
    let mut in_single_comment = false;
    let mut in_multi_comment = false;
    let mut string_char = '\0';
    let mut last_char = '\0';
    let mut last_was_space = false;

    while let Some(ch) = chars.next() {
        // 处理单行注释
        if !in_string && !in_multi_comment && ch == '/' && chars.peek() == Some(&'/') {
            in_single_comment = true;
            chars.next(); // 跳过第二个 /
            continue;
        }

        if in_single_comment {
            if ch == '\n' {
                in_single_comment = false;
                // 保留换行（某些情况下需要）
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }
            continue;
        }

        // 处理多行注释
        if !in_string && !in_single_comment && ch == '/' && chars.peek() == Some(&'*') {
            in_multi_comment = true;
            chars.next(); // 跳过 *
            continue;
        }

        if in_multi_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                in_multi_comment = false;
                chars.next(); // 跳过 /
                if !last_was_space {
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
            } else if ch == string_char && last_char != '\\' {
                in_string = false;
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
            // 在某些情况下需要保留空格
            let needs_space = match last_char {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$' => {
                    matches!(chars.peek(), Some(&c) if c.is_alphanumeric() || c == '_' || c == '$')
                }
                _ => false
            };

            if needs_space && !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_char = ch;
            last_was_space = false;
        }
    }

    result
}

/// 压缩 HTML 内容，保留 Askama 模板语法
pub fn minify_html(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut in_pre = false;
    let mut in_textarea = false;
    let mut in_template_brace = false; // {{ }}
    let mut in_template_chevron = false; // {% %}
    let mut last_was_space = false;
    let mut tag_name = String::new();
    let mut script_content = String::new();
    let mut style_content = String::new();

    while let Some(ch) = chars.next() {
        // 检测模板语法开始
        if ch == '{' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '{' {
                    in_template_brace = true;
                    result.push(ch);
                    continue;
                } else if next_ch == '%' {
                    in_template_chevron = true;
                    result.push(ch);
                    continue;
                }
            }
        }

        // 在模板语法内，保持原样
        if in_template_brace || in_template_chevron {
            result.push(ch);
            // 检测模板语法结束
            if in_template_brace && ch == '}' && result.ends_with("}}") {
                in_template_brace = false;
            } else if in_template_chevron && ch == '}' && result.ends_with("%}") {
                in_template_chevron = false;
            }
            last_was_space = false;
            continue;
        }

        // HTML 注释处理（只在不在 script/style 内时处理）
        if !in_script && !in_style && ch == '<' && chars.peek() == Some(&'!') {
            let mut comment = String::from("<");
            comment.push(chars.next().unwrap()); // '!'

            if chars.peek() == Some(&'-') {
                comment.push(chars.next().unwrap()); // first '-'
                if chars.peek() == Some(&'-') {
                    comment.push(chars.next().unwrap()); // second '-'
                    // 这是一个注释，跳过直到 -->
                    while let Some(c) = chars.next() {
                        comment.push(c);
                        if comment.ends_with("-->") {
                            break;
                        }
                    }
                    last_was_space = false;
                    continue; // 跳过注释
                }
            }
            result.push_str(&comment);
            continue;
        }

        // 标签处理
        if ch == '<' {
            // 在 script/style 内，只处理结束标签
            if in_script || in_style {
                // 检查是否是结束标签
                if chars.peek() == Some(&'/') {
                    // 可能是 </script> 或 </style>，需要处理
                } else {
                    // 不是结束标签，收集为内容
                    if in_script {
                        script_content.push(ch);
                    } else {
                        style_content.push(ch);
                    }
                    last_was_space = false;
                    continue;
                }
            }
            in_tag = true;
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
                tag_name.push(chars.next().unwrap().to_ascii_lowercase());
            }

            // 检查特殊标签 - 在输出标签名之前处理结束标签的内容
            if tag_name == "/script" {
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
            } else if tag_name == "/style" {
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
            } else if tag_name == "/pre" {
                in_pre = false;
            } else if tag_name == "/textarea" {
                in_textarea = false;
            }

            // 现在输出标签名
            result.push_str(&tag_name);

            // 处理开始标签
            if tag_name == "script" {
                in_script = true;
                script_content.clear();
            } else if tag_name == "style" {
                in_style = true;
                style_content.clear();
            } else if tag_name == "pre" {
                in_pre = true;
            } else if tag_name == "textarea" {
                in_textarea = true;
            }
            continue;
        }

        // 在 script 和 style 内收集内容（必须在 > 检查之前，但要确保不在标签内）
        if !in_tag {
            if in_script {
                script_content.push(ch);
                last_was_space = false;
                continue;
            } else if in_style {
                style_content.push(ch);
                last_was_space = false;
                continue;
            }
        }

        if ch == '>' {
            in_tag = false;
            result.push(ch);
            last_was_space = false;
            continue;
        }

        // 在 pre 和 textarea 内完全保留原样
        if in_pre || in_textarea {
            result.push(ch);
            last_was_space = false;
        }
        // 在标签内压缩空格
        else if in_tag {
            if ch.is_whitespace() {
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            } else {
                result.push(ch);
                last_was_space = false;
            }
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

    result
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
    fn test_minify_html_removes_whitespace() {
        let html = "<div>   <p>   text   </p>   </div>";
        let result = minify_html(html);
        assert_eq!(result, "<div> <p> text </p> </div>");
    }

    #[test]
    fn test_minify_html_preserves_pre() {
        let html = "<pre>  code  \n  block  </pre>";
        let result = minify_html(html);
        assert!(result.contains("  code  \n  block  "));
    }
}
