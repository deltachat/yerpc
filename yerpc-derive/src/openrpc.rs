use crate::{util::extract_result_ty, Inputs, RpcInfo, parse::{Input, RemoteProcedure}};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;

fn generate_param(input: &Input, i: usize) -> TokenStream {
    let name = input
        .ident
        .map_or_else(|| format!("arg{}", i + 1), ToString::to_string)
        .to_case(Case::Camel);
    let ty = input.ty;
    quote! {
        ::yerpc::openrpc::Param {
            name: #name.to_string(),
            description: None,
            schema: ::yerpc::openrpc::generate_schema::<#ty>(),
        }

    }
}

fn generate_method(method: &RemoteProcedure) -> TokenStream {
    let (params, param_structure) = match &method.input {
        Inputs::Positional(ref inputs) => {
            let params = inputs.iter().enumerate().map(|(i, input)| generate_param(&input, i)).collect::<Vec<_>>();
            let params = quote!(vec![#(#params),*]);
            let structure = quote!(::yerpc::openrpc::ParamStructure::ByPosition);
            (params, structure)
        },
        Inputs::Structured(Some(input)) => {
            let ty = &input.ty;
            let params = quote!(::yerpc::openrpc::object_schema_to_params::<#ty>().expect("Invalid parameter structure"));
            let structure = quote!(::yerpc::openrpc::ParamStructure::ByName);
            (params, structure)
        },
        Inputs::Structured(None) => {
            let params = quote!(vec![]);
            let structure = quote!(::yerpc::openrpc::ParamStructure::ByPosition);
            (params, structure)
        }
    };
    let name = &method.name;
    // TODO: Support notifications.
    let _is_notification = method.is_notification;
    let docs = if let Some(docs) = &method.docs {
        quote!(Some(#docs.to_string()))
    } else {
        quote!(None)
    };
    let output_ty = method.output.map(extract_result_ty).map(|ty| quote!(#ty)).unwrap_or(quote!(()));
    let output_name = format!("{}Result", name).to_case(Case::UpperCamel);
    let result = quote! {
        ::yerpc::openrpc::Param {
            name: #output_name.to_string(),
            description: None,
            schema: ::yerpc::openrpc::generate_schema::<#output_ty>(),
        }
    };
    quote! {
        ::yerpc::openrpc::Method {
            name: #name.to_string(),
            summary: None,
            description: #docs,
            param_structure: #param_structure,
            params: #params,
            result: #result
        }
    }
}

fn generate_doc(info: &RpcInfo) -> TokenStream {
    let methods = &info.methods.iter().map(generate_method).collect::<Vec<_>>();
    let title = format!("{}", &info.self_ty.to_token_stream());
    let info = quote! {
        ::yerpc::openrpc::Info {
            version: "1.0.0".to_string(),
            title: #title.to_string()
        }
    };
    quote! {
        ::yerpc::openrpc::Doc {
            openrpc: "1.0.0".to_string(),
            info: #info,
            methods: vec![#(#methods),*],
        }
    }
}

pub(crate) fn generate_openrpc_generator(info: &RpcInfo) -> TokenStream {
    let outdir_path = info
        .attr_args
        .openrpc_path
        .clone()
        .unwrap_or_else(|| "openrpc".to_string());
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let outdir = std::path::PathBuf::from(&manifest_dir).join(&outdir_path);
    let outdir = outdir.to_str().unwrap();

    let doc_spec = generate_doc(&info);

    quote! {
        /// Generate typescript bindings for the JSON-RPC API.
        #[cfg(test)]
        #[test]
        fn generate_openrpc_document() {
            let doc = #doc_spec;
            let outdir = ::std::path::Path::new(#outdir);
            let json = ::serde_json::to_string_pretty(&doc).expect("Failed to serialize OpenRPC document into JSON.");
            ::std::fs::create_dir_all(&outdir).expect(&format!("Failed to create directory `{}`", outdir.display()));
            ::std::fs::write(&outdir.join("openrpc.json"), &json).expect("Failed to write OpenRPC document");
        }
    }
}
