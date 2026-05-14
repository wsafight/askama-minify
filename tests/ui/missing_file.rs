use askama::Template;
use askama_minify::template_minify;

#[template_minify(path = "does-not-exist.html")]
#[derive(Template)]
struct MissingFile;

fn main() {}
