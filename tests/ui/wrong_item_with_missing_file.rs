use askama_minify::template_minify;

#[template_minify(path = "does-not-exist.html")]
fn not_a_template_item() {}

fn main() {}
