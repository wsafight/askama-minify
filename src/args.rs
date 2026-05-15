use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Lit, LitStr, Meta, MetaNameValue, Token};

pub(crate) struct MacroArgs {
    pub(crate) input: TemplateInput,
    pub(crate) ext: Option<LitStr>,
    pub(crate) passthrough: Vec<Meta>,
}

pub(crate) enum TemplateInput {
    Path(LitStr),
    Source(LitStr),
}

impl MacroArgs {
    pub(crate) fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
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
