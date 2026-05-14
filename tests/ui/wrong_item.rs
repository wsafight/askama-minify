use askama_minify::template_minify;

#[template_minify(source = "<p></p>", ext = "html")]
fn not_a_template_item() {}

fn main() {}
