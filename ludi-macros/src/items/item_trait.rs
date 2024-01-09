use darling::usage::{GenericsExt, IdentSet};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::emit_error;
use quote::quote;
use syn::spanned::Spanned;

use crate::items::method::Method;
use crate::options::{MsgOptions, WrapOptions};

pub(crate) struct ItemTrait {
    msg_options: Option<MsgOptions>,
    ident: syn::Ident,
    vis: syn::Visibility,
    methods: Vec<Method>,
}

impl ItemTrait {
    pub(crate) fn from_item_trait(item: &syn::ItemTrait, msg_options: Option<MsgOptions>) -> Self {
        if item.generics.lifetimes().count() > 0 {
            emit_error!(item.generics, "trait can not be generic over lifetimes");
        }

        let type_params = item.generics.declared_type_params();
        let methods = item
            .items
            .iter()
            .filter_map(|item| {
                match item {
                    syn::TraitItem::Fn(f) => return Some(f),
                    syn::TraitItem::Const(_) => {
                        // TODO: support associated consts
                        emit_error!(item, "const items are not supported");
                    }
                    syn::TraitItem::Type(_) => {
                        // TODO: support associated types
                        emit_error!(item, "associated types are not supported");
                    }
                    _ => {
                        emit_error!(item, "only methods are supported");
                    }
                };
                None
            })
            .map(|method| {
                Method::new(
                    &item.ident,
                    &type_params,
                    msg_options.clone(),
                    None,
                    method.span(),
                    method.attrs.clone(),
                    item.vis.clone(),
                    method.sig.clone(),
                    None,
                )
            })
            .collect::<Vec<_>>();

        Self {
            msg_options,
            ident: item.ident.clone(),
            vis: item.vis.clone(),
            methods,
        }
    }

    fn expand_messages(&self) -> TokenStream {
        let mut tokens = TokenStream::new();
        for method in &self.methods {
            tokens.extend(method.expand_message());
        }
        tokens
    }

    fn expand_wrap(&self) -> TokenStream {
        let Some(WrapOptions { attrs, name }) = self
            .msg_options
            .as_ref()
            .map(|opts| opts.wrap.clone().map(|opts| opts.unwrap_or_default()))
            .flatten()
        else {
            return TokenStream::new();
        };

        let attrs = attrs
            .as_ref()
            .map(|attrs| attrs.clone().to_vec())
            .unwrap_or_default();

        let wrap_ident = name.clone().unwrap_or_else(|| {
            syn::Ident::new(&format!("{}Msg", self.ident.to_string()), Span::call_site())
        });

        let mut wrap_type_params = IdentSet::default();
        let mut variants = Vec::with_capacity(self.methods.len());
        for method in &self.methods {
            wrap_type_params.extend(method.type_params.clone());

            let variant_ident = &method.struct_ident;
            let struct_ident = &method.struct_ident;
            let type_params = method.type_params.iter();

            variants.push(quote!(
                #variant_ident (#struct_ident<#(#type_params),*>)
            ));
        }

        let wrap_type_params = wrap_type_params.iter();
        let vis = &self.vis;

        quote!(
            #[derive(::ludi::Wrap)]
            #(#[#attrs])*
            #vis enum #wrap_ident<#(#wrap_type_params),*> {
                #(#variants),*
            }
        )
    }

    pub(crate) fn expand(&self) -> TokenStream {
        let mut tokens = TokenStream::new();
        tokens.extend(self.expand_messages());
        tokens.extend(self.expand_wrap());
        tokens
    }
}
