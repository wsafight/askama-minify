use askama::Template;
use askama_minify::template_minify;

#[template_minify(source = "<p>{{ value }}</p>", ext = "html")]
#[derive(Template)]
#[template(source = "<p>{{ value }}</p>", ext = "html")]
struct ExistingTemplateAttr<'a> {
    value: &'a str,
}

fn main() {}
