extern crate darling;
use darling::FromAttributes;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item};

mod rpc;
mod ts;
use rpc::impl_rpc;
use ts::impl_ts;

#[derive(FromAttributes, Debug, Default)]
#[darling(default, attributes(rpc))]
pub(crate) struct RpcAttrArgs {
    /// Set the name of the RPC method. Defaults to the function name.
    name: Option<String>,
    /// Make this a notification method. Notifications are received like method calls but cannot
    /// return anything.
    #[darling(default)]
    notification: bool,
    /// Positional mode means that the parameters of the RPC call are expected to be a JSON array,
    /// which will be parsed as a tuple of this function's arguments.
    #[darling(default)]
    positional: bool,
    /// Set the path where typescript definitions are written to (relative to the crate root).
    /// Defaults to `ts-bindings`.
    ts_outdir: Option<String>,
}

#[proc_macro_attribute]
pub fn rpc(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut orig_input = input.clone();
    let input = parse_macro_input!(input as Item);

    let impls: TokenStream = match input {
        Item::Impl(input) => {
            let attrs = match RpcAttrArgs::from_attributes(&input.attrs) {
                Ok(args) => args,
                Err(err) => return err.write_errors().into(),
            };
            let rpc = impl_rpc(&attrs, &input);
            let ts = impl_ts(&attrs, &input);
            quote! {
                #rpc
                #ts
            }
        }
        Item::Fn(_) => quote! {},
        _ => syn::Error::new_spanned(
            input,
            "The #[rpc] attribute only works on impl and method items",
        )
        .to_compile_error(),
    }
    .into();
    orig_input.extend(impls);
    orig_input
}
