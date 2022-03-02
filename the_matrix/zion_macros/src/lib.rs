#![feature(extend_one)]

use std::iter;

use derive_syn_parse::Parse;
use proc_macro2::{Ident, Punct, Spacing, TokenTree};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input::ParseMacroInput;
use syn::{parse_macro_input, LitInt, Token};

#[derive(Parse)]
struct Item {
    lit: LitInt,
    _t1: Token![=>],
    ident: Ident,
}

struct RegistryInput {
    items: Vec<Item>,
}

impl Parse for RegistryInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut idents = Vec::new();
        while !input.is_empty() {
            idents.push(input.parse()?);
        }
        Ok(Self { items: idents })
    }
}

#[proc_macro]
pub fn registry(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let items = parse_macro_input!(input as RegistryInput);
    let lit = items
        .items
        .iter()
        .map(|x| x.lit.base10_parse::<u64>().unwrap());
    let ident = items.items.iter().map(|x| &x.ident);
    let output = quote! {
        macro_rules! system_lookup {
            ($id:expr, $object:ident, $method:ident, $err:block) => {
                match $id {
                    #(
                        #lit => {
                            $object.$method::<#ident>();
                        },
                    )*
                    _ => $err,
                }
            }
        }
    };
    let a: std::collections::VecDeque<u8>;
    output.into()
}
