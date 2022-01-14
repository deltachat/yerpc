use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, DeriveInput, Item, ItemImpl};
extern crate darling;
use darling::{FromAttributes, FromMeta};

mod rpc;
use rpc::impl_rpc;

#[derive(FromAttributes, Debug, Default)]
#[darling(default, attributes(rpc))]
pub(crate) struct RpcAttrArgs {
    #[darling(default)]
    notification: bool,
    // #[darling(default)]
    // param_list: bool,
}

#[proc_macro_attribute]
pub fn rpc(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Item);
    let res: proc_macro2::TokenStream = match input {
        Item::Impl(input) => impl_rpc(input).into(),
        Item::Fn(input) => {
            let res = quote! {
                #input
            };
            res.into()
        }
        _ => syn::Error::new_spanned(
            input,
            "The #[rpc] custom attribute only works on impl and method items",
        )
        .to_compile_error(),
    };
    res.into()
}
