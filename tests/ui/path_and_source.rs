use askama::Template;
use askama_minify::template_minify;

#[template_minify(path = "index.html", source = "<p></p>", ext = "html")]
#[derive(Template)]
struct PathAndSource;

fn main() {}
