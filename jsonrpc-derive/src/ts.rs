use super::RpcAttrArgs;
use convert_case::{Case, Casing};
use darling::FromAttributes;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    FnArg, GenericArgument, ImplItem, ImplItemMethod, ItemImpl, PathArguments, ReturnType, Type,
};

pub(crate) fn impl_ts(args: &RpcAttrArgs, input: &ItemImpl) -> TokenStream {
    let mut ts_methods = vec![];
    for item in &input.items {
        if let ImplItem::Method(method) = item {
            ts_methods.push(TsMethod::from_input(method));
        }
    }
    generate_typescript_generator(ts_methods, &args.ts_outdir)
}

struct TsMethod<'s> {
    attrs: RpcAttrArgs,
    name: String,
    inputs: Vec<&'s Type>,
    output: Option<&'s Type>,
}

impl<'s> TsMethod<'s> {
    pub fn from_input(method: &'s ImplItemMethod) -> Self {
        let args = RpcAttrArgs::from_attributes(&method.attrs).unwrap_or_default();
        let name = args
            .name
            .clone()
            .unwrap_or_else(|| method.sig.ident.to_string());
        let output = match &method.sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, ref ty) => Some(ty.as_ref()),
        };
        let mut inputs = vec![];
        for arg in &method.sig.inputs {
            match arg {
                FnArg::Typed(ref arg) => inputs.push(arg.ty.as_ref()),
                FnArg::Receiver(_) => {}
            }
        }
        Self {
            name,
            inputs,
            output,
            attrs: args,
        }
    }
}

fn generate_typescript_generator(
    methods: Vec<TsMethod>,
    custom_outdir: &Option<String>,
) -> TokenStream {
    let mut types = vec![];
    let mut defs = vec![];
    let mut ts_functions: Vec<TokenStream> = vec![];

    for method in methods {
        // Generate a MethodNameInput struct for the typescript definition.
        let input_newty = ty_ident(&format!("{}Input", method.name));
        let input_tys = method.inputs.iter().map(|ty| quote!(#ty,));
        defs.push(quote!(
            #[derive(::typescript_type_def::TypeDef)]
            struct #input_newty(#(#input_tys)*);
        ));
        types.push(quote!(#input_newty));

        // Generate a MethodNameOutput struct for the typescript definition.
        let output_newty = ty_ident(&format!("{}Output", method.name));
        let output_ty = &method
            .output
            .map(extract_result_ty)
            .map(|ty| quote!(#ty))
            .unwrap_or(quote!(()));
        defs.push(quote!(
            #[derive(::typescript_type_def::TypeDef)]
            struct #output_newty(#output_ty);
        ));
        types.push(quote!(#output_newty));

        let ts_method = method.name.to_case(Case::Camel);
        let ts_ns = "T.";
        let ts_output = if method.attrs.notification {
            "void".to_owned()
        } else {
            format!("Promise<{}{}>", ts_ns, output_newty)
        };
        let (ts_params, ts_call) = if method.attrs.positional {
            let mut param_str = String::new();
            let mut call_str = String::new();
            for (i, input) in method.inputs.iter().enumerate() {
                let input_newty = ty_ident(&format!("{}Input{}", method.name, i + 1));
                let var_name = format!("arg{}", i);
                defs.push(quote!(
                    #[derive(::typescript_type_def::TypeDef)]
                    struct #input_newty(#input);
                ));
                types.push(quote!(#input_newty));
                param_str.push_str(&format!("{}: {}{}, ", var_name, ts_ns, input_newty));
                call_str.push_str(&format!("{}, ", var_name));
            }
            param_str.truncate(param_str.len() - 2);
            call_str.truncate(call_str.len() - 2);
            let call_str = format!("[{}]", call_str);
            (param_str, call_str)
        } else {
            (format!("params: {}{}", ts_ns, input_newty), "params".into())
        };

        let fn_str = if method.attrs.notification {
            format!(
                "    public {}({}): {} {{ return this._notification('{}', {}) }}\n",
                ts_method, ts_params, ts_output, method.name, ts_call
            )
        } else {
            format!(
                "    public async {}({}): {} {{ return this._request('{}', {}) }}\n",
                ts_method, ts_params, ts_output, method.name, ts_call
            )
        };
        ts_functions.push(quote!(
            out.push_str(#fn_str);
        ));
    }

    let outdir_path = custom_outdir.as_deref().unwrap_or("ts-bindings");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let outdir = std::path::PathBuf::from(&manifest_dir).join(&outdir_path);
    let outdir = outdir.to_str().unwrap();

    quote! {
        /// Generate typescript bindings for the JSON-RPC API.
        #[cfg(test)]
        #[test]
        fn generate_ts_bindings() {
            // Generate typescript definitions.
            #(#defs)*
            #[derive(typescript_type_def::TypeDef)]
            struct __AllTyps(#(#types),*, ::jsonrpc::Error, ::jsonrpc::Message);
            let mut bindings = ::std::vec::Vec::new();
            let mut options = ::typescript_type_def::DefinitionFileOptions::default();
            options.root_namespace = None;
            ::typescript_type_def::write_definition_file::<_, __AllTyps>(&mut bindings, options)
                .expect("Failed to generate typescript bindings");

            // Generate a raw client.
            let mut out = String::new();
            let mut out = [
                r#"import * as T from "./types.js""#,
                r#"export abstract class RawClient {"#,
                r#"    abstract _notification(method: string, params?: any): void;"#,
                r#"    abstract _request(method: string, params?: any): Promise<any>;"#,
                r#""#
            ].join("\n");
            #(#ts_functions)*
            out.push_str("}\n");

            // Write the files.
            let outdir = ::std::path::Path::new(#outdir);
            ::std::fs::create_dir_all(&outdir).expect(&format!("Failed to create directory `{}`", outdir.display()));
            ::std::fs::write(&outdir.join("types.ts"), &bindings).expect("Failed to write TS bindings");
            ::std::fs::write(&outdir.join("client.ts"), &out).expect("Failed to write TS bindings");
        }
    }
}

fn extract_result_ty(ty: &Type) -> &Type {
    if let Type::Path(path) = ty {
        if let Some(last) = path.path.segments.last() {
            if last.ident == "Result" {
                if let PathArguments::AngleBracketed(ref generics) = last.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = generics.args.first() {
                        return inner_ty;
                    }
                }
            }
        }
    }
    ty
}

fn ty_ident(name: &str) -> Ident {
    Ident::new(&name.to_case(Case::UpperCamel), Span::call_site())
}
