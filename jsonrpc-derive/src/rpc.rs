use crate::{util::is_result_ty, Inputs, RpcInfo};
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_rpc_impl(info: &RpcInfo) -> TokenStream {
    let mut request_arms = vec![];
    let mut notification_arms = vec![];

    for method in &info.methods[..] {
        let name = &method.name;
        let ident = &method.ident;

        let call = match &method.input {
            // Call with an array of multiple arguments.
            Inputs::Positional(inputs) => {
                let n_inputs = inputs.len();
                let inputs =
                    (0..n_inputs).map(|_| quote!(serde_json::from_value(params.next().unwrap())?));
                quote!(
                    let params: Vec<serde_json::Value> = ::serde_json::from_value(params)?;
                    if params.len() != #n_inputs {
                        return Err(::jsonrpc::Error::invalid_args_len(#n_inputs));
                    }
                    let mut params = params.into_iter();
                    let res = self.#ident(#(#inputs),*).await;
                )
            }
            // Call with a single argument.
            Inputs::Structured(_input) => {
                quote!(
                    let params = ::serde_json::from_value(params)?;
                    let res = self.#ident(params).await;
                )
            }
        };

        let unwrap_output = match &method.output {
            Some(output) if is_result_ty(&output) => quote!(let res = res?;),
            _ => quote!(),
        };

        match method.is_notification {
            false => request_arms.push(quote! {
                #name => {
                    #call
                    #unwrap_output
                    let res = ::serde_json::to_value(&res)?;
                    Ok(res)
                },
            }),
            true => notification_arms.push(quote! {
                #name => {
                    #call
                    #unwrap_output
                    let _ = res;
                    Ok(())
                },
            }),
        }
    }

    let struc = &info.self_ty;
    let crat = quote! { ::jsonrpc };
    let (impl_generics, _ty_generics, where_clause) = &info.generics.split_for_impl();

    // eprintln!("struc {:#?}", struc);
    // eprintln!("generics {:#?}", info.generics.split_for_impl());
    quote! {
        #[automatically_derived]
        #[::jsonrpc::async_trait]
        impl #impl_generics #crat::RpcHandler for #struc #where_clause {
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
