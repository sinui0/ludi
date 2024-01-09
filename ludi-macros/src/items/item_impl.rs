use darling::usage::{GenericsExt, IdentSet};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::{abort, emit_error};
use quote::quote;
use syn::{parse_quote, spanned::Spanned};

use crate::{
    items::method::Method,
    options::{CtrlOptions, MsgOptions, WrapOptions},
    utils::{ctrl_ident, is_ludi_attr},
};

pub(crate) struct ItemImpl {
    attrs: Vec<syn::Attribute>,
    msg_options: Option<MsgOptions>,
    ctrl_options: Option<CtrlOptions>,
    impl_trait: Option<ImplTrait>,
    impl_generics: syn::Generics,
    actor_ident: syn::Ident,
    actor_path: syn::Path,
    actor_generic_args: Option<syn::AngleBracketedGenericArguments>,
    methods: Vec<Method>,
}

pub(crate) struct ImplTrait {
    trait_ident: syn::Ident,
    trait_path: syn::Path,
}

impl ItemImpl {
    pub(crate) fn from_item_impl(
        item: &syn::ItemImpl,
        mut msg_options: Option<MsgOptions>,
        mut ctrl_options: Option<CtrlOptions>,
    ) -> Self {
        let mut attrs = item.attrs.clone();

        let item_msg_options = MsgOptions::maybe_from_attributes(&attrs);
        if let Some(item_msg_options) = item_msg_options {
            if let Some(msg_options) = msg_options.as_mut() {
                msg_options.merge(&item_msg_options);
            } else {
                msg_options = Some(item_msg_options);
            }
        }

        let item_ctrl_options = CtrlOptions::maybe_from_attributes(&attrs);
        if let Some(item_ctrl_options) = item_ctrl_options {
            if let Some(ctrl_options) = ctrl_options.as_mut() {
                ctrl_options.merge(&item_ctrl_options);
            } else {
                ctrl_options = Some(item_ctrl_options);
            }
        }

        attrs.retain(|attr| !is_ludi_attr(attr));

        let syn::Type::Path(syn::TypePath {
            path: actor_path, ..
        }) = *(item.self_ty).clone()
        else {
            abort!(item.self_ty, "expected path to actor type");
        };

        let actor_segment = actor_path.segments.last().expect("actor path is non-empty");
        let actor_ident = actor_segment.ident.clone();
        let actor_generic_args = match &actor_segment.arguments {
            syn::PathArguments::None => None,
            syn::PathArguments::AngleBracketed(args) => Some(args.clone()),
            syn::PathArguments::Parenthesized(_) => {
                abort!(actor_segment.arguments, "unexpected parenthesis arguments")
            }
        };

        let impl_trait = if let Some((_, trait_path, _)) = item.trait_.clone() {
            let trait_segment = trait_path.segments.last().expect("trait path is non-empty");
            let trait_ident = trait_segment.ident.clone();
            Some(ImplTrait {
                trait_ident,
                trait_path,
            })
        } else {
            None
        };

        let parent_ident = if let Some(impl_trait) = &impl_trait {
            &impl_trait.trait_ident
        } else {
            &actor_ident
        };

        let type_params = item.generics.declared_type_params();

        let methods = item
            .items
            .iter()
            .filter_map(|item| {
                match item {
                    syn::ImplItem::Fn(f) => return Some(f),
                    syn::ImplItem::Const(_) => {
                        // TODO: support associated consts
                        emit_error!(item, "const items are not supported");
                    }
                    syn::ImplItem::Type(_) => {
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
                    parent_ident,
                    &type_params,
                    msg_options.clone(),
                    ctrl_options.clone(),
                    method.span(),
                    method.attrs.clone(),
                    method.vis.clone(),
                    method.sig.clone(),
                    Some(method.block.clone()),
                )
            })
            .collect::<Vec<_>>();

        Self {
            attrs,
            msg_options,
            ctrl_options,
            impl_trait,
            impl_generics: item.generics.clone(),
            actor_ident,
            actor_path,
            actor_generic_args,
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
            if let Some(impl_trait) = &self.impl_trait {
                syn::Ident::new(
                    &format!(
                        "{}{}Msg",
                        self.actor_ident.to_string(),
                        impl_trait.trait_ident.to_string()
                    ),
                    Span::call_site(),
                )
            } else {
                syn::Ident::new(
                    &format!("{}Msg", self.actor_ident.to_string()),
                    Span::call_site(),
                )
            }
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

        quote!(
            #[derive(::ludi::Wrap)]
            #(#[#attrs])*
            pub enum #wrap_ident<#(#wrap_type_params),*> {
                #(#variants),*
            }
        )
    }

    fn expand_handlers(&self) -> TokenStream {
        let mut tokens = TokenStream::new();
        for method in &self.methods {
            tokens.extend(method.expand_handler(&self.actor_path, &self.impl_generics));
        }
        tokens
    }

    fn expand_ctrl(&self) -> TokenStream {
        let Self {
            attrs,
            ctrl_options,
            actor_path,
            ..
        } = self;

        if self
            .methods
            .iter()
            .all(|method| method.ctrl_options.is_none())
        {
            return TokenStream::new();
        }

        let mut ctrl_path = ctrl_options
            .as_ref()
            .map(|opts| opts.path.clone())
            .flatten()
            .unwrap_or_else(|| syn::Path::from(ctrl_ident(&self.actor_ident)));

        if let Some(generic_args) = &self.actor_generic_args {
            let mut generic_args = generic_args.clone();
            generic_args.args.push(parse_quote!(A));
            ctrl_path.segments.last_mut().unwrap().arguments =
                syn::PathArguments::AngleBracketed(generic_args);
        } else {
            ctrl_path.segments.last_mut().unwrap().arguments =
                syn::PathArguments::AngleBracketed(parse_quote!(<A,>));
        }

        let mut generics = self.impl_generics.clone();
        generics.params.push(parse_quote!(A));

        let where_clause = generics.make_where_clause();
        where_clause
            .predicates
            .push(parse_quote!(A: Send + Sync + 'static));
        where_clause
            .predicates
            .push(parse_quote!(Self: ::ludi::Controller<Actor = #actor_path>));

        if let Some(ImplTrait { trait_path, .. }) = &self.impl_trait {
            self.methods.iter().for_each(|method| {
                let struct_path = &method.struct_path;
                where_clause.predicates.push(
                    parse_quote!(<Self as ::ludi::Controller>::Message: ::ludi::Wrap<#struct_path>),
                );
            });

            let methods = self.methods.iter().map(|method| method.expand_ctrl(true));
            let (impl_generics, _, where_clause) = generics.split_for_impl();
            quote!(
                #(#attrs)*
                impl #impl_generics #trait_path for #ctrl_path #where_clause {
                    #(#methods)*
                }
            )
        } else {
            let impl_blocks = self.methods.iter().map(|method| {
                let struct_path = &method.struct_path;
                let impl_method = method.expand_ctrl(false);
                let mut generics = generics.clone();

                let where_clause = generics.make_where_clause();
                where_clause.predicates.push(
                    parse_quote!(<Self as ::ludi::Controller>::Message: ::ludi::Wrap<#struct_path>),
                );
                let (impl_generics, _, where_clause) = generics.split_for_impl();

                quote!(
                    #(#attrs)*
                    impl #impl_generics #ctrl_path #where_clause {
                        #impl_method
                    }
                )
            });

            quote!(
                #(#impl_blocks)*
            )
        }
    }

    pub(crate) fn expand(&self) -> TokenStream {
        let mut tokens = TokenStream::new();

        if self.impl_trait.is_none()
            || self
                .msg_options
                .as_ref()
                .map(|opts| opts.foreign.is_present())
                .unwrap_or(false)
        {
            tokens.extend(self.expand_messages());
            tokens.extend(self.expand_wrap());
        }

        tokens.extend(self.expand_handlers());
        tokens.extend(self.expand_ctrl());

        tokens
    }
}
