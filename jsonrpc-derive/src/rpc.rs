use super::RpcAttrArgs;
use darling::FromAttributes;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItem, ItemImpl};

pub(crate) fn impl_rpc(_args: &RpcAttrArgs, input: &ItemImpl) -> TokenStream {
    let mut request_arms = vec![];
    let mut notification_arms = vec![];

    let struc = &input.self_ty;

    for item in &input.items {
        match item {
            ImplItem::Method(method) => {
                let args = RpcAttrArgs::from_attributes(&method.attrs).unwrap_or_default();
                let ident = method.sig.ident.clone();
                let name = args.name.unwrap_or_else(|| ident.to_string());

                let n_inputs = method.sig.inputs.len() - 1;
                let inputs = (0..n_inputs).map(|_i| {
                    quote! {
                        serde_json::from_value(params.next().unwrap())?,
                    }
                });

                let call = if args.positional {
                    // Call with an array of multiple arguments.
                    quote!(
                        let params: Vec<serde_json::Value> = ::serde_json::from_value(params)?;
                        if params.len() != #n_inputs {
                            return Err(::jsonrpc::Error::invalid_args_len(#n_inputs));
                        }
                        let mut params = params.into_iter();
                        let res = self.#ident(#(#inputs)*).await;
                    )
                } else {
                    // Call with a single argument.
                    quote!(
                        let params = ::serde_json::from_value(params)?;
                        let res = self.#ident(params).await;
                    )
                };

                match args.notification {
                    false => request_arms.push(quote! {
                        #name => {
                            #call
                            let res = ::serde_json::to_value(&res?)?;
                            Ok(res)
                        },
                    }),
                    true => notification_arms.push(quote! {
                        #name => {
                            #call
                            let _ = res?;
                        },
                    }),
                }
            }
            _ => {}
        }
    }
    let crat = quote! { ::jsonrpc };

    quote! {
        #[::async_trait::async_trait]
        impl #crat::RpcHandler for #struc {
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
            // async fn on_notification(
            //     &self,
            //     method: String,
            //     params: ::serde_json::Value,
            // ) -> Result<(), #crat::Error> {
            //     match method.as_str() {
            //         #(#notification_arms)*
            //         _ => Err(#crat::Error::method_not_found())
            //     }
            // }
        }
    }
}
