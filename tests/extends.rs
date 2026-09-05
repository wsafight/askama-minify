use askama::Template;
use askama_minify::template_minify;

#[template_minify(path = "tests/templates/extends/index.html")]
#[derive(Template)]
struct ExtendTemplate;

#[template_minify(
    source = r#"{% extends "tests/templates/extends/base.html" %}
{% block content %}<p>inline</p>{% endblock %}"#,
    ext = "html"
)]
#[derive(Template)]
struct InlineExtendTemplate;

#[test]
fn minifies_extended_html() {
    let rendered = ExtendTemplate.render().unwrap();

    assert!(!rendered.contains("removed"));
    assert!(rendered.contains("<nav> inherited </nav>"));
    assert!(rendered.contains("<aside>navigation</aside>"));
    assert!(rendered.contains(r#"{% include "partial.html" %}"#));
}

#[test]
fn minifies_extended_html_from_inline_source() {
    let rendered = InlineExtendTemplate.render().unwrap();

    assert!(!rendered.contains("removed"));
    assert!(rendered.contains("<nav> inherited </nav>"));
    assert!(rendered.contains("<aside>navigation</aside>"));
    assert!(rendered.contains("<p>inline</p>"));
}
