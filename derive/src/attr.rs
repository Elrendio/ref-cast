//! This module handles parsing of `#[ref_cast(...)]` attributes. The entrypoints
//! is `attr::Container::from_ast`. It returns an instance of the corresponding
//! struct.

use {
    proc_macro2::{Group, Span, TokenStream, TokenTree},
    quote::ToTokens,
    syn::{
        parse::{self, Parse},
        Error,
    },
};

use crate::symbols::{Symbol, CRATE, REF_CAST};

/// Represents container attribute information.
pub struct Container {
    pub crate_ref_cast_path: syn::Path,
}

impl Container {
    /// Extract out the `#[ref_cast(...)]` attributes from an item.
    pub fn from_ast(item: &syn::DeriveInput) -> crate::Result<Self> {
        let mut crate_ref_cast_path = None;
        for meta_item in item.attrs.iter().filter(|attr| attr.path() == REF_CAST) {
            meta_item.parse_nested_meta(|attr| match attr.path.get_ident() {
                // Parse `#[ref_cast(crate = "foo")]`
                Some(ident) if ident == CRATE => {
                    crate_ref_cast_path =
                        Some(parse_lit_into_path(CRATE, &attr.value()?.parse()?)?);
                    Ok(())
                }
                _ => Err(Error::new_spanned(
                    meta_item.path(),
                    format!(
                        "unknown ref_cast container attribute `{}`",
                        attr.path.into_token_stream().to_string().replace(' ', "")
                    ),
                )),
            })?;
        }

        Ok(Self {
            crate_ref_cast_path: crate_ref_cast_path
                .unwrap_or_else(|| syn::parse_str(&format!("::{}", REF_CAST)).unwrap()),
        })
    }
}

fn get_lit_str(attr_name: Symbol, lit: &syn::Lit) -> crate::Result<&syn::LitStr> {
    use syn::Lit;
    match lit {
        Lit::Str(lit) => Ok(lit),
        _ => Err(Error::new_spanned(
            lit,
            format!(
                "expected ref_cast {0} attribute to be a string: `{0} = \"...\"`",
                attr_name
            ),
        )),
    }
}

fn parse_lit_into_path(attr_name: Symbol, lit: &syn::Lit) -> crate::Result<syn::Path> {
    let string = get_lit_str(attr_name, lit)?;
    parse_lit_str(string)
        .map_err(|_| Error::new_spanned(lit, format!("failed to parse path: {:?}", string.value())))
}

/// Parses a string litteral as rust code
fn parse_lit_str<T>(s: &syn::LitStr) -> parse::Result<T>
where
    T: Parse,
{
    let stream = syn::parse_str(&s.value())?;
    syn::parse2(respan_token_stream(stream, s.span()))
}

fn respan_token_stream(stream: TokenStream, span: Span) -> TokenStream {
    stream
        .into_iter()
        .map(|mut token| {
            if let TokenTree::Group(g) = &mut token {
                *g = Group::new(g.delimiter(), respan_token_stream(g.stream(), span));
            }
            token.set_span(span);
            token
        })
        .collect()
}
