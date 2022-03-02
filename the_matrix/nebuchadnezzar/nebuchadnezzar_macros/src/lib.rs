#[macro_use]
extern crate syn;

use nebuchadnezzar_core::requests::*;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::parse::{Parse, ParseBuffer, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Token;
use syn::{parse_macro_input, DeriveInput, Error, Type, TypePath};
use syn::{Data, Result};

struct Args {
    super_request: TypePath,
    types: Punctuated<Type, Token![,]>,
}

impl Parse for Args {
    fn parse(input: &ParseBuffer) -> Result<Self> {
        let sr = input.parse()?;
        input.parse::<Token![for]>()?;
        Ok(Args {
            super_request: sr,
            types: Punctuated::parse_separated_nonempty(input)?,
        })
    }
}

#[proc_macro_attribute]
pub fn converter(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as Args);
    let input = parse_macro_input!(input as DeriveInput);
    let kind = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return Error::new(input.data.span(), "expected `struct`")
                .into_compile_error()
                .into()
        }
    };
    let super_request = &args.super_request.path.segments.last().unwrap().ident;
    let mut request_handler = quote!();
    let mut response_handler = quote!();
    match super_request.to_string().as_str() {
        stringify!(CandlesGetRequest) => {
            let mut stream =
            let super_rhs = super_request_rhs(CandlesGetRequest::)
            for field in kind.fields.iter() {
                let mut rhs;
                if "timeframe" == field.ident.unwrap().to_string() {
                    rhs = quote! {super_request.timeframe.try_into()?};
                } else if "symbol" == field.ident.unwrap().to_string() {
                    rhs = quote! {super_request.timeframe.try_into()?};
                }
                quote! {
                    #request_handler
                    #field: rhs,
                }
            }
        }
        _ => {
            return Error::new(
                super_request.span(),
                "Converter is only supported for types in `nebuchadnezzar_core::requests`.",
            )
            .into_compile_error()
            .into()
        }
    }
    // let mut item = parse_macro_input!(input as DeriveInput);
    // (quote!(#item)).into()
    unimplemented!()
}

fn super_request_rhs(super_request: TokenStream) -> HashMap<String, TokenStream> {
    let input = parse_macro_input!(super_request as DeriveInput);
    let mut super_request;
    match input.data {
        Data::Struct(data) => super_request = data,
        _ => unreachable!(),
    }
    let mut map = HashMap::new();
    for field in &super_request.fields {
        let rhs = quote! {};
        let name = field.ident.unwrap().to_string();
        match name.as_str() {
            "timeframe" => quote!(super_request.timeframe.try_into()?),
            _ => {
                let ident = Ident::new("name", Span::def_site());
                quote!(super_request.#ident)
            }
        }
        map.insert(name, rhs);
    }
    map
}