use crate::args::MacroArgs;
use crate::item::reject_existing_template_attr;
use crate::loader::{load_template, minify_template_source};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Attribute, DeriveInput, LitStr, parse_quote};

pub(crate) fn expand_template_minify(
    args: MacroArgs,
    mut item: DeriveInput,
) -> syn::Result<TokenStream2> {
    reject_existing_template_attr(&item)?;

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
    item.attrs.push(template_attr);

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
