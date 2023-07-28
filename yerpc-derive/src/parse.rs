use super::MethodAttrArgs;
use darling::FromAttributes;
// use proc_macro2::Ident;
use syn::{FnArg, Generics, Ident, ImplItem, ImplItemMethod, ItemImpl, Pat, ReturnType, Type};

use crate::RootAttrArgs;

/// Result of parsing the `impl` of an RPC server.
#[derive(Debug)]
pub(crate) struct RpcInfo<'s> {
    pub self_ty: &'s Type,
    pub _attr_args: &'s RootAttrArgs,

    /// Descriptions of RPC methods.
    pub methods: Vec<RemoteProcedure<'s>>,

    /// Lifetype and type parameters that appear
    /// between the `impl` keyword and the type.
    pub generics: &'s Generics,
}

impl<'s> RpcInfo<'s> {
    pub fn from_impl(attr_args: &'s RootAttrArgs, input: &'s ItemImpl) -> Self {
        let methods = input
            .items
            .iter()
            .filter_map(|item| {
                if let ImplItem::Method(method) = item {
                    Some(RemoteProcedure::from_method(attr_args, method))
                } else {
                    None
                }
            })
            .collect();
        Self {
            _attr_args: attr_args,
            methods,
            self_ty: &input.self_ty,
            generics: &input.generics,
        }
    }
}

/// Description of a single RPC method.
#[derive(Debug)]
pub(crate) struct RemoteProcedure<'s> {
    /// Identifier of the function implementing the method.
    pub ident: &'s Ident,

    /// Method name as should be sent in a JSON-RPC requst.
    ///
    /// By default the same as the function name,
    /// but may be overridden by an attribute.
    pub name: String,

    /// Description of the method parameters.
    pub input: Inputs<'s>,

    /// Output type of the method.
    pub output: Option<&'s Type>,
    pub is_notification: bool,

    /// Documentation extracted from the documentation comment.
    pub docs: Option<String>,
}

/// Description of a single method parameters.
#[derive(Debug)]
pub(crate) enum Inputs<'s> {
    Positional(Vec<Input<'s>>),
    Structured(Option<Input<'s>>),
}

/// Description of a single method parameter.
#[derive(Debug)]
pub(crate) struct Input<'s> {
    pub ident: Option<&'s Ident>,
    pub ty: &'s Type,
}

impl<'s> Input<'s> {
    fn new(ty: &'s Type, ident: Option<&'s Ident>) -> Self {
        Self { ty, ident }
    }
    fn from_arg(arg: &'s FnArg) -> Option<Self> {
        match arg {
            FnArg::Typed(ref arg) => Some(Self::new(arg.ty.as_ref(), ident_from_pat(&arg.pat))),
            FnArg::Receiver(_) => None,
        }
    }
}

fn parse_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let mut parts = vec![];
    for attr in attrs {
        let meta = attr.parse_meta().unwrap();
        if let syn::Meta::NameValue(meta) = meta {
            if let syn::Lit::Str(doc) = meta.lit {
                parts.push(doc.value());
            }
        }
    }
    if !parts.is_empty() {
        Some(parts.join("\n"))
    } else {
        None
    }
}

impl<'s> RemoteProcedure<'s> {
    pub fn from_method(root_attr_args: &RootAttrArgs, method: &'s ImplItemMethod) -> Self {
        let args = MethodAttrArgs::from_attributes(&method.attrs).unwrap_or_default();
        let name = args.name.unwrap_or_else(|| method.sig.ident.to_string());
        let output = match &method.sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, ref ty) => Some(ty.as_ref()),
        };
        let positional = root_attr_args.all_positional || args.positional;
        let mut inputs_iter = method.sig.inputs.iter();
        let input = if positional {
            let inputs = inputs_iter.filter_map(Input::from_arg);
            Inputs::Positional(inputs.collect())
        } else {
            let input = inputs_iter.find_map(Input::from_arg);
            Inputs::Structured(input)
        };
        let docs = parse_doc_comment(&method.attrs);
        Self {
            ident: &method.sig.ident,
            name,
            input,
            output,
            is_notification: args.notification,
            docs,
        }
    }
}

fn ident_from_pat(pat: &Pat) -> Option<&Ident> {
    match pat {
        Pat::Ident(pat_ident) => Some(&pat_ident.ident),
        _ => None,
    }
}
