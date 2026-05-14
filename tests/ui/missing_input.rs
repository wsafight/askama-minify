use askama::Template;
use askama_minify::template_minify;

#[template_minify]
#[derive(Template)]
struct MissingInput;

fn main() {}
