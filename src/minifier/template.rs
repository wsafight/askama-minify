pub(super) fn contains_askama_template(value: &str) -> bool {
    next_template_tag(value, 0).is_some()
}

pub(super) fn try_push_askama_template(
    ch: char,
    chars: &mut std::str::Chars<'_>,
    target: &mut String,
) -> Option<char> {
    if ch != '{' {
        return None;
    }

    let remaining = chars.as_str();
    let next_ch = remaining.as_bytes().first()?;
    let mut end = match next_ch {
        b'{' => find_tag_end(remaining, 1, "}}"),
        b'%' => find_tag_end(remaining, 1, "%}"),
        b'#' => find_nested_comment_end(remaining, 1),
        _ => return None,
    }
    .unwrap_or(remaining.len());

    let start = target.len();
    target.push(ch);
    target.push_str(&remaining[..end]);
    if *next_ch == b'%' && block_keyword(&target[start..]) == "raw" {
        let raw_end = find_raw_end(remaining, end).unwrap_or(remaining.len());
        target.push_str(&remaining[end..raw_end]);
        end = raw_end;
    }
    *chars = remaining[end..].chars();
    target.chars().next_back()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TemplateTag {
    Block,
    Expression,
    Comment,
}

pub(crate) fn next_template_tag(source: &str, cursor: usize) -> Option<(usize, TemplateTag)> {
    source.as_bytes()[cursor..]
        .windows(2)
        .enumerate()
        .find_map(|(offset, pair)| {
            if pair[0] != b'{' {
                return None;
            }
            let tag = match pair[1] {
                b'%' => TemplateTag::Block,
                b'{' => TemplateTag::Expression,
                b'#' => TemplateTag::Comment,
                _ => return None,
            };
            Some((cursor + offset, tag))
        })
}

pub(crate) fn find_tag_end(source: &str, start: usize, marker: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (offset, ch) in source[start..].char_indices() {
        let position = start + offset;
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
        } else if matches!(ch, '"' | '\'') {
            quote = Some(ch);
        } else if source[position..].starts_with(marker) {
            return Some(position + marker.len());
        }
    }
    None
}

pub(crate) fn find_comment_end(source: &str, start: usize) -> Option<usize> {
    find_nested_comment_end(source, start + 2)
}

fn find_nested_comment_end(source: &str, mut cursor: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 1usize;
    while cursor + 1 < bytes.len() {
        match &bytes[cursor..cursor + 2] {
            b"{#" => depth += 1,
            b"#}" => {
                depth -= 1;
                if depth == 0 {
                    return Some(cursor + 2);
                }
            }
            _ => {
                cursor += 1;
                continue;
            }
        }
        cursor += 2;
    }
    None
}

pub(crate) fn find_raw_end(source: &str, mut cursor: usize) -> Option<usize> {
    while let Some(offset) = source[cursor..].find("{%") {
        let start = cursor + offset;
        let inner = source[start + 2..].trim_start_matches(template_prefix_whitespace);
        if inner.starts_with("endraw") {
            let end = find_tag_end(source, start + 2, "%}")?;
            if block_keyword(&source[start..end]) == "endraw" {
                return Some(end);
            }
        }
        cursor = start + 2;
    }
    None
}

pub(crate) fn block_keyword(block: &str) -> &str {
    let Some(inner) = block
        .strip_prefix("{%")
        .and_then(|block| block.strip_suffix("%}"))
    else {
        return "";
    };
    let content = inner.trim_start_matches(template_prefix_whitespace);
    let keyword_end = content
        .find(|ch: char| !ch.is_ascii_alphabetic())
        .unwrap_or(content.len());
    &content[..keyword_end]
}

fn template_prefix_whitespace(ch: char) -> bool {
    ch.is_ascii_whitespace() || "-+~".contains(ch)
}
