use crate::{util::extract_result_ty, Inputs, RpcInfo};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
pub(crate) fn generate_typescript_generator(info: &RpcInfo) -> TokenStream {
    let mut gen_types = vec![];
    let mut gen_methods = vec![];
    for method in &info.methods {
        let (is_positional, gen_args) = match &method.input {
            Inputs::Positional(ref inputs) => {
                let mut gen_args = vec![];
                for (i, input) in inputs.iter().enumerate() {
                    let ty = input.ty;
                    let name = input
                        .ident
                        .map_or_else(|| format!("arg{}", i + 1), ToString::to_string);
                    gen_types.push(quote!(#ty));
                    gen_args.push(quote!((#name.to_string(), &<#ty as TypeDef>::INFO)))
                }
                (true, gen_args)
            }
            Inputs::Structured(Some(input)) => {
                let mut gen_args = vec![];
                let ty = input.ty;
                let name = input
                    .ident
                    .map_or_else(|| "params".to_string(), ToString::to_string);
                gen_types.push(quote!(#ty));
                gen_args.push(quote!((#name.to_string(), &<#ty as TypeDef>::INFO)));
                (false, gen_args)
            }
            Inputs::Structured(None) => (false, vec![]),
        };
        let gen_output = match (method.output, method.is_notification) {
            (_, true) | (None, _) => quote!(None),
            (Some(ty), false) => {
                let ty = extract_result_ty(ty);
                gen_types.push(quote!(#ty));
                quote!(Some(&<#ty as TypeDef>::INFO))
            }
        };
        let ts_name = method.name.to_case(Case::Camel);
        let rpc_name = &method.name;
        let is_notification = method.is_notification;
        gen_methods.push(quote!(
                let args = vec![#(#gen_args),*];
                let method = Method::new(#ts_name, #rpc_name, args, #gen_output, #is_notification, #is_positional);
                out.push_str(&method.to_string(&root_namespace));
        ));
    }

    let outdir_path = info
        .attr_args
        .ts_outdir
        .as_deref()
        .unwrap_or("typescript/generated");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let outdir = std::path::PathBuf::from(&manifest_dir).join(&outdir_path);
    let outdir = outdir.to_str().unwrap();

    let ts_base = include_str!("client.ts");

    quote! {
        /// Generate typescript bindings for the JSON-RPC API.
        #[cfg(test)]
        #[test]
        fn generate_ts_bindings() {
            use ::jsonrpc::typescript::type_def::{TypeDef, type_expr::TypeInfo, DefinitionFileOptions};
            use ::jsonrpc::typescript::{typedef_to_expr_string, export_types_to_file, Method};
            use ::std::{fs, path::Path};

            // Create output directory.
            let outdir = Path::new(#outdir);
            fs::create_dir_all(&outdir).expect(&format!("Failed to create directory `{}`", outdir.display()));

            // Create helper type with all exported types.
            // #(#gen_definitions)*
            #[derive(TypeDef)]
            struct __AllTyps(#(#gen_types),*);
            // Write typescript types to file.
            export_types_to_file::<__AllTyps>(&outdir.join("types.ts"), None).expect("Failed to write TS out");
            export_types_to_file::<::jsonrpc::Message>(&outdir.join("jsonrpc.ts"), None).expect("Failed to write TS out");

            // // Generate a raw client.
            let root_namespace = Some("T");
            let mut out = String::new();
            #(#gen_methods)*
            let ts_module = #ts_base.replace("#methods", &out);
            fs::write(&outdir.join("client.ts"), &ts_module).expect("Failed to write TS bindings");
        }
    }
}
