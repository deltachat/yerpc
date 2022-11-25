use super::MethodAttrArgs;
use darling::FromAttributes;
// use proc_macro2::Ident;
use syn::{FnArg, Generics, Ident, ImplItem, ImplItemMethod, ItemImpl, Pat, ReturnType, Type};

use crate::RootAttrArgs;

#[derive(Debug)]
pub(crate) struct RpcInfo<'s> {
    pub self_ty: &'s Type,
    pub _attr_args: &'s RootAttrArgs,
    pub methods: Vec<RemoteProcedure<'s>>,
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

#[derive(Debug)]
pub(crate) struct RemoteProcedure<'s> {
    pub ident: &'s Ident,
    pub name: String,
    pub input: Inputs<'s>,
    pub output: Option<&'s Type>,
    pub is_notification: bool,
    pub docs: Option<String>,
}

#[derive(Debug)]
pub(crate) enum Inputs<'s> {
    Positional(Vec<Input<'s>>),
    Structured(Option<Input<'s>>),
}

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
