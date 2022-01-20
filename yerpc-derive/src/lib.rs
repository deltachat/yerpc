extern crate darling;
use darling::{FromAttributes, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Item};

mod parse;
mod rpc;
mod ts;
pub(crate) use parse::*;
pub(crate) use rpc::*;
pub(crate) use ts::*;
pub(crate) mod util;

#[proc_macro_attribute]
pub fn rpc(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let item = parse_macro_input!(tokens as Item);
    match &item {
        Item::Impl(input) => {
            let attr_args = parse_macro_input!(attr as AttributeArgs);
            let attr_args = match RootAttrArgs::from_list(&attr_args) {
                Ok(args) => args,
                Err(err) => return err.write_errors().into(),
            };
            let info = RpcInfo::from_impl(&attr_args, &input);
            let ts_impl = generate_typescript_generator(&info);
            let rpc_impl = generate_rpc_impl(&info);
            quote! {
                #item
                #rpc_impl
                #ts_impl
            }
        }
        Item::Fn(_) => quote!(#item),
        _ => syn::Error::new_spanned(
            item,
            "The #[rpc] attribute only works on impl and method items",
        )
        .to_compile_error(),
    }
    .into()
}

#[derive(FromMeta, Debug, Default)]
#[darling(default)]
pub(crate) struct RootAttrArgs {
    /// Positional mode means that the parameters of the RPC call are expected to be a JSON array,
    /// which will be parsed as a tuple of this function's arguments.
    all_positional: bool,
    /// Set the path where typescript definitions are written to (relative to the crate root).
    /// Defaults to `ts-bindings`.
    ts_outdir: Option<String>,
}

#[derive(FromAttributes, Debug, Default)]
#[darling(default, attributes(rpc))]
pub(crate) struct MethodAttrArgs {
    /// Set the name of the RPC method. Defaults to the function name.
    name: Option<String>,
    /// Make this a notification method. Notifications are received like method calls but cannot
    /// return anything.
    #[darling(default)]
    notification: bool,
    /// Positional mode means that the parameters of the RPC call are expected to be a JSON array,
    /// which will be parsed as a tuple of this function's arguments.
    positional: bool,
}
