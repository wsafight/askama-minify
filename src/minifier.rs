mod css;
mod html;
mod js;
pub(crate) mod template;
mod util;

pub(crate) use html::{has_sensitive_template_context, minify_html, minify_html_fragment};
