use std::collections::HashMap;

use heck::{ToSnakeCase, ToUpperCamelCase};

use proc_macro2::Span;
use quote::quote;
use syn::{parse_str, Ident, ItemTrait, Path, Type};

use crate::types::MethodSig;

pub(crate) fn impl_interface(item: ItemTrait) -> proc_macro::TokenStream {
    let ident = &item.ident;
    let msgs_module: Path =
        parse_str(&format!("{}_msgs", ident.to_string().to_snake_case())).unwrap();

    let sigs = item.items.into_iter().filter_map(|item| match item {
        syn::TraitItem::Method(method) => Some(MethodSig::from(method.sig)),
        _ => None,
    });

    let msg_enum_name = Ident::new(&format!("{}Message", ident), Span::call_site());
    let msg_return_enum_name = Ident::new(&format!("{}MessageReturn", ident), Span::call_site());

    let mut msg_idents = Vec::new();
    let mut msg_arg_idents = Vec::new();
    let mut msg_arg_types = Vec::new();
    let mut msg_rets = Vec::new();
    let mut msgs = Vec::new();
    let mut ret_map: HashMap<Type, Vec<Ident>> = HashMap::new();
    for sig in sigs {
        let MethodSig { ident, args, ret } = sig;

        let ident: Ident = parse_str(&ident.to_string().to_upper_camel_case()).unwrap();

        let msg = if args.is_empty() {
            quote!(
                pub struct #ident;
            )
        } else {
            let arg_idents = args.iter().map(|(ident, _)| ident);
            let arg_types = args.iter().map(|(_, ty)| ty);
            quote!(
                pub struct #ident {
                    #( pub #arg_idents: #arg_types ),*
                }
            )
        };
        msgs.push(msg);

        msg_idents.push(ident.clone());
        for arg in args {
            msg_arg_idents.push(arg.0);
            msg_arg_types.push(arg.1);
        }

        if let Some(variants) = ret_map.get_mut(&ret) {
            variants.push(ident);
        } else {
            ret_map.insert(ret.clone(), vec![ident]);
        }

        msg_rets.push(ret);
    }

    let ret_into = ret_map
        .iter()
        .map(|(ty, variants)| {
            quote! {
                impl Into<#ty> for #msg_return_enum_name {
                    fn into(self) -> #ty {
                        match self {
                            #( #msg_return_enum_name :: #variants (value) => value, )*
                            _ => unreachable!("handler returned unexpected type, this indicates the `Message` implementation is incorrect"),
                        }
                    }
                }
            }
        });

    quote! {
        use #msgs_module :: #msg_enum_name;
        
        pub mod #msgs_module {
            pub enum #msg_enum_name {
                #( #msg_idents ( #msg_idents ) ),*
            }

            pub enum #msg_return_enum_name {
                #( #msg_idents ( #msg_rets ) ),*
            }

            #(
                #msgs
            )*

            #(
                impl From<#msg_idents> for #msg_enum_name {
                    fn from(value: #msg_idents) -> Self {
                        #msg_enum_name :: #msg_idents (value)
                    }
                }
            )*
    
            #(
                #ret_into
            )*

            impl<A> ::ludi::Message<A> for #msg_enum_name where
                A: ::ludi::Actor,
                #( A: ::ludi::Handler<#msg_idents, Return = #msg_rets>, )*
            {
                type Return = #msg_return_enum_name;

                async fn handle<M: ::ludi::Mailbox<A>, R: FnOnce(Self::Return)>(
                    self,
                    actor: &mut A,
                    ctx: &mut ::ludi::Context<'_, A, M>,
                    ret: R,
                ) {
                    match self {
                        #(
                            #msg_enum_name :: #msg_idents (msg) => {
                                let value = #msg_return_enum_name :: #msg_idents (::ludi::Handler::<#msg_idents>::handle(actor, msg, ctx).await);
                                ret(value);
                                ::ludi::Handler::<#msg_idents>::after(actor, ctx).await;
                            }
                        ),*
                    };
                }
            }
        }
    }.into()
}
