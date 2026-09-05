use askama::Template;
use askama_minify::template_minify;


#[template_minify(path = "tests/templates/extends/index.html")]
#[derive(Template)]
struct ExtendTemplate;

#[test]
fn minifies_extended_html() {
    let rendered = ExtendTemplate.render().unwrap();
    assert!(!rendered.contains("removed"));
}
