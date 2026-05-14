use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use std::fs;
use std::path::{Path, PathBuf};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Expr, ExprLit, Item, Lit, LitStr, Meta, MetaNameValue, Token, parse_macro_input,
    parse_quote,
};

mod minifier;

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
    let item = parse_macro_input!(item as Item);

    match expand_template_minify(args, item) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

struct MacroArgs {
    input: TemplateInput,
    ext: Option<LitStr>,
    passthrough: Vec<Meta>,
}

enum TemplateInput {
    Path(LitStr),
    Source(LitStr),
}

struct LoadedTemplate {
    source: String,
    ext: String,
    include_path: Option<PathBuf>,
}

impl MacroArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        let mut path = None;
        let mut source = None;
        let mut ext = None;
        let mut passthrough = Vec::new();

        for meta in metas {
            if let Some(value) = string_name_value(&meta, "path")? {
                set_once(&mut path, value, "duplicate `path` argument")?;
                continue;
            }

            if let Some(value) = string_name_value(&meta, "source")? {
                set_once(&mut source, value, "duplicate `source` argument")?;
                continue;
            }

            if let Some(value) = string_name_value(&meta, "ext")? {
                set_once(&mut ext, value, "duplicate `ext` argument")?;
                continue;
            }

            passthrough.push(meta);
        }

        let input = match (path, source) {
            (Some(path), None) => TemplateInput::Path(path),
            (None, Some(source)) => TemplateInput::Source(source),
            (Some(path), Some(_)) => {
                return Err(syn::Error::new_spanned(
                    path,
                    "`path` and `source` cannot be used together",
                ));
            }
            (None, None) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "expected `path = \"...\"` or `source = \"...\"`",
                ));
            }
        };

        Ok(Self {
            input,
            ext,
            passthrough,
        })
    }
}

fn expand_template_minify(args: MacroArgs, mut item: Item) -> syn::Result<TokenStream2> {
    reject_existing_template_attr(&item)?;
    item_attrs_mut(&mut item)?;

    let template = load_template(&args)?;
    let source = LitStr::new(
        &minify_template_source(&template.source, &template.ext),
        Span::call_site(),
    );
    let ext = LitStr::new(&template.ext, Span::call_site());
    let passthrough = args.passthrough;

    let template_attr: Attribute = parse_quote! {
        #[template(source = #source, ext = #ext #(, #passthrough)*)]
    };
    item_attrs_mut(&mut item)?.push(template_attr);

    let tracking = template.include_path.map(|path| {
        let path = LitStr::new(&path.to_string_lossy(), Span::call_site());
        quote! {
            const _: &str = include_str!(#path);
        }
    });

    Ok(quote! {
        #item
        #tracking
    })
}

fn load_template(args: &MacroArgs) -> syn::Result<LoadedTemplate> {
    match &args.input {
        TemplateInput::Source(source) => {
            let Some(ext) = &args.ext else {
                return Err(syn::Error::new_spanned(
                    source,
                    "`source` templates require `ext = \"...\"`",
                ));
            };

            Ok(LoadedTemplate {
                source: source.value(),
                ext: ext.value(),
                include_path: None,
            })
        }
        TemplateInput::Path(path) => {
            let resolved = resolve_template_path(&path.value())
                .map_err(|message| syn::Error::new_spanned(path, message))?;
            let source = fs::read_to_string(&resolved).map_err(|error| {
                syn::Error::new_spanned(
                    path,
                    format!("failed to read template `{}`: {error}", resolved.display()),
                )
            })?;
            let ext = args
                .ext
                .as_ref()
                .map(LitStr::value)
                .or_else(|| extension_from_path(&resolved))
                .ok_or_else(|| {
                    syn::Error::new_spanned(
                        path,
                        "could not infer template extension; add `ext = \"...\"`",
                    )
                })?;

            Ok(LoadedTemplate {
                source,
                ext,
                include_path: Some(resolved),
            })
        }
    }
}

fn resolve_template_path(path: &str) -> Result<PathBuf, String> {
    let raw = Path::new(path);
    if raw.is_absolute() && raw.is_file() {
        return Ok(raw.to_path_buf());
    }

    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "CARGO_MANIFEST_DIR is not set".to_string())?;
    let candidates = [
        manifest_dir.join(raw),
        manifest_dir.join("templates").join(raw),
    ];

    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned()
        .ok_or_else(|| {
            let tried = candidates
                .iter()
                .map(|candidate| format!("`{}`", candidate.display()))
                .collect::<Vec<_>>()
                .join(", ");
            format!("template `{path}` was not found; tried {tried}")
        })
}

fn minify_template_source(source: &str, ext: &str) -> String {
    if matches!(ext.to_ascii_lowercase().as_str(), "html" | "htm") {
        minifier::minify_html(source)
    } else {
        source.to_owned()
    }
}

fn extension_from_path(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(ToOwned::to_owned)
}

fn item_attrs_mut(item: &mut Item) -> syn::Result<&mut Vec<Attribute>> {
    match item {
        Item::Const(item) => Ok(&mut item.attrs),
        Item::Enum(item) => Ok(&mut item.attrs),
        Item::Struct(item) => Ok(&mut item.attrs),
        Item::Union(item) => Ok(&mut item.attrs),
        _ => Err(syn::Error::new_spanned(
            item,
            "`template_minify` can only be used on an Askama template item",
        )),
    }
}

fn reject_existing_template_attr(item: &Item) -> syn::Result<()> {
    let attrs = match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        _ => return Ok(()),
    };

    for attr in attrs {
        if attr.path().is_ident("template") {
            return Err(syn::Error::new_spanned(
                attr,
                "`template_minify` generates `#[template(...)]`; remove the existing template attribute",
            ));
        }
    }

    Ok(())
}

fn string_name_value(meta: &Meta, name: &str) -> syn::Result<Option<LitStr>> {
    let Meta::NameValue(MetaNameValue { path, value, .. }) = meta else {
        return Ok(None);
    };

    if !path.is_ident(name) {
        return Ok(None);
    }

    match value {
        Expr::Lit(ExprLit {
            lit: Lit::Str(value),
            ..
        }) => Ok(Some(value.clone())),
        _ => Err(syn::Error::new_spanned(
            value,
            format!("`{name}` must be a string literal"),
        )),
    }
}

fn set_once<T>(target: &mut Option<T>, value: T, message: &str) -> syn::Result<()> {
    if target.is_some() {
        return Err(syn::Error::new(Span::call_site(), message));
    }

    *target = Some(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse::Parser;

    #[test]
    fn parses_path_argument() {
        let args = MacroArgs::parse
            .parse2(quote!(path = "index.html"))
            .unwrap();
        match args.input {
            TemplateInput::Path(path) => assert_eq!(path.value(), "index.html"),
            TemplateInput::Source(_) => panic!("expected path input"),
        }
    }

    #[test]
    fn requires_ext_for_source_argument() {
        let args = MacroArgs::parse
            .parse2(quote!(source = "<p>{{ value }}</p>"))
            .unwrap();

        assert!(load_template(&args).is_err());
    }

    #[test]
    fn minifies_html_templates() {
        let result = minify_template_source("<div>   value   </div>", "HTML");

        assert_eq!(result, "<div> value </div>");
    }

    #[test]
    fn leaves_non_html_templates_unchanged() {
        let source = "line 1\n  {{ value }}\nline 3";
        let result = minify_template_source(source, "txt");

        assert_eq!(result, source);
    }
}
