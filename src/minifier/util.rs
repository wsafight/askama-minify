pub(super) fn trim_trailing_space(value: &mut String) {
    if value.ends_with(' ') {
        value.pop();
    }
}

pub(super) fn trim_trailing_whitespace(value: &mut String) {
    while value.chars().next_back().is_some_and(char::is_whitespace) {
        value.pop();
    }
}
