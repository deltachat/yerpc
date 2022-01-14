// extern crate darling;
use darling::FromAttributes;
// use darling::FromDeriveInput;
use super::RpcAttrArgs;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItem, ItemImpl};

pub(crate) fn impl_rpc(input: ItemImpl) -> TokenStream {
    // let _args = match RpcAttrArgs::from_attributes(&input.attrs) {
    //     Ok(args) => args,
    //     Err(err) => return err.write_errors().into(),
    // };
    let mut request_arms = vec![];
    let mut notification_arms = vec![];
    for item in &input.items {
        match item {
            ImplItem::Method(method) => {
                let args = RpcAttrArgs::from_attributes(&method.attrs).unwrap_or_default();
                let ident = method.sig.ident.clone();
                let name = ident.to_string();
                match args.notification {
                    false => request_arms.push(quote! {
                        #name => {
                            let params = serde_json::from_value(params)?;
                            let res = self.#ident(params).await?;
                            let res = ::serde_json::to_value(&res)?;
                            Ok(res)
                        },
                    }),
                    true => notification_arms.push(quote! {
                        #name => {
                            let params = serde_json::from_value(params)?;
                            self.#ident(params).await
                        },
                    }),
                }
            }
            _ => {}
        }
    }
    let crat = quote! { ::jsonrpc };

    quote! {
        #input

        #[::async_trait::async_trait]
        impl #crat::RpcHandler for Session {
            async fn on_request(
                &self,
                method: String,
                params: ::serde_json::Value,
            ) -> Result<::serde_json::Value, #crat::Error> {
                match method.as_str() {
                    #(#request_arms)*
                    _ => Err(#crat::Error::method_not_found())
                }
            }
            async fn on_notification(
                &self,
                method: String,
                params: ::serde_json::Value,
            ) -> Result<(), #crat::Error> {
                match method.as_str() {
                    #(#notification_arms)*
                    _ => Err(#crat::Error::method_not_found())
                }
            }
        }
    }
}
