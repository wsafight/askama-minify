use askama::Template;
use askama_minify::template_minify;

#[template_minify(source = "<p>{{ value }}</p>")]
#[derive(Template)]
struct SourceWithoutExt<'a> {
    value: &'a str,
}

fn main() {}
