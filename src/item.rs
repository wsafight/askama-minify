use syn::DeriveInput;
use syn::parse::{Parse, ParseStream};

pub(crate) struct TemplateItem(pub(crate) DeriveInput);

impl Parse for TemplateItem {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        DeriveInput::parse(input).map(Self).map_err(|error| {
            syn::Error::new(
                error.span(),
                "`template_minify` can only be used on an Askama template item",
            )
        })
    }
}

pub(crate) fn reject_existing_template_attr(item: &DeriveInput) -> syn::Result<()> {
    for attr in &item.attrs {
        if attr.path().is_ident("template") {
            return Err(syn::Error::new_spanned(
                attr,
                "`template_minify` generates `#[template(...)]`; remove the existing template attribute",
            ));
        }
    }

    Ok(())
}
