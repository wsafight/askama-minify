pub(super) fn try_push_askama_template(
    ch: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    target: &mut String,
) -> Option<char> {
    if ch != '{' {
        return None;
    }

    let &next_ch = chars.peek()?;

    let end = match next_ch {
        '{' => "}}",
        '%' => "%}",
        '#' => "#}",
        _ => return None,
    };

    target.push(ch);
    let mut last_ch = ch;
    for next_ch in chars.by_ref() {
        target.push(next_ch);
        last_ch = next_ch;
        if target.ends_with(end) {
            break;
        }
    }

    Some(last_ch)
}
