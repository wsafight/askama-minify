use proc_macro::TokenStream;
use syn::parse_macro_input;

mod args;
mod expand;
mod item;
mod loader;
mod minifier;

use args::MacroArgs;
use expand::expand_template_minify;
use item::TemplateItem;

/// Reads an Askama template at compile time, minifies it, and injects it as
/// `#[template(source = "...", ext = "...")]`.
///
/// Use it with Askama's derive macro:
///
/// ```ignore
/// use askama::Template;
/// use askama_minify::template_minify;
///
/// #[template_minify(path = "index.html")]
/// #[derive(Template)]
/// struct IndexTemplate<'a> {
///     title: &'a str,
/// }
/// ```
///
/// Template paths are resolved relative to `CARGO_MANIFEST_DIR`; if that file
/// does not exist, `templates/<path>` is tried to match Askama's default layout.
#[proc_macro_attribute]
pub fn template_minify(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with MacroArgs::parse);
    let item = parse_macro_input!(item as TemplateItem).0;

    match expand_template_minify(args, item) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
