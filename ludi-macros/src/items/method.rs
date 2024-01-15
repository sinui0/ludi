use std::collections::HashSet;

use darling::usage::{IdentSet, Purpose, UsesTypeParams};
use heck::ToUpperCamelCase;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::emit_error;
use quote::quote;
use syn::parse_quote;

use crate::{
    options::{CtrlOptions, ErrorStrategy, MsgOptions},
    utils::{extract_output, is_ludi_attr},
};

/// An item method.
pub(crate) struct Method {
    /// Attributes directly on the method. If these are present on an impl block they will
    /// be forwarded to the handler impl if applicable.
    pub(crate) attrs: Vec<syn::Attribute>,
    /// Doc attributes are forwarded to the controller impl if applicable.
    pub(crate) doc_attrs: Vec<syn::Attribute>,

    /// Message options
    pub(crate) msg_options: Option<MsgOptions>,
    /// Controller options
    pub(crate) ctrl_options: Option<CtrlOptions>,

    /// Method visibility
    pub(crate) vis: syn::Visibility,
    /// Method signature
    pub(crate) sig: syn::Signature,
    /// Method body, if present.
    pub(crate) body: Option<syn::Block>,

    /// Method arguments
    pub(crate) args: Vec<(syn::Ident, syn::Type)>,
    /// Method return type
    pub(crate) return_ty: syn::Type,
    /// Type params from the parent item which are present in the method signature
    pub(crate) type_params: IdentSet,

    /// The struct ident, eg. Foo
    pub(crate) struct_ident: syn::Ident,
    /// The path to the struct, eg. foo::bar::Foo
    pub(crate) struct_path: syn::Path,
}

impl Method {
    pub(crate) fn new(
        parent_ident: &syn::Ident,
        parent_type_params: &IdentSet,
        mut msg_options: Option<MsgOptions>,
        mut ctrl_options: Option<CtrlOptions>,
        span: Span,
        attrs: Vec<syn::Attribute>,
        vis: syn::Visibility,
        sig: syn::Signature,
        body: Option<syn::Block>,
    ) -> Self {
        Method::check_signature(&span, &sig);

        let (args, return_ty) = Self::extract_args(&sig);
        let type_params = Self::extract_type_params(
            &parent_type_params,
            args.iter().map(|(_, ty)| ty),
            &return_ty,
        );

        let method_msg_options = MsgOptions::maybe_from_attributes(&attrs);
        if let Some(method_msg_options) = method_msg_options {
            if let Some(msg_options) = msg_options.as_mut() {
                msg_options.merge(&method_msg_options);
            } else {
                msg_options = Some(method_msg_options);
            }
        }

        let method_ctrl_options = CtrlOptions::maybe_from_attributes(&attrs);
        if let Some(method_ctrl_options) = method_ctrl_options {
            if let Some(ctrl_options) = ctrl_options.as_mut() {
                ctrl_options.merge(&method_ctrl_options);
            } else {
                ctrl_options = Some(method_ctrl_options);
            }
        }

        let struct_ident =
            if let Some(struct_name) = msg_options.as_ref().and_then(|opts| opts.name.clone()) {
                syn::Ident::new(
                    &struct_name
                        .replace("{item}", &parent_ident.to_string())
                        .replace("{name}", &sig.ident.to_string().to_upper_camel_case()),
                    Span::call_site(),
                )
            } else {
                syn::Ident::new(
                    &format!(
                        "{}Msg{}",
                        parent_ident.to_string(),
                        sig.ident.to_string().to_upper_camel_case()
                    ),
                    Span::call_site(),
                )
            };

        if struct_ident.to_string() == parent_ident.to_string() {
            emit_error!(
                sig.ident,
                "message struct name must not be the same as the parent item"
            );
        }

        let struct_path =
            if let Some(path) = msg_options.as_ref().and_then(|opts| opts.path.as_ref()) {
                parse_quote!(#path :: #struct_ident)
            } else {
                parse_quote!(#struct_ident)
            };

        let (mut attrs, doc_attrs): (Vec<_>, Vec<_>) = attrs
            .into_iter()
            .partition(|attr| !attr.meta.path().is_ident("doc"));
        attrs.retain(|attr| !is_ludi_attr(attr));

        Self {
            attrs,
            doc_attrs,
            msg_options,
            ctrl_options,
            vis,
            sig,
            body,
            args,
            return_ty,
            type_params,
            struct_ident,
            struct_path,
        }
    }

    fn check_signature(span: &Span, sig: &syn::Signature) {
        if !sig.generics.params.is_empty() {
            emit_error!(
                sig.generics.params,
                "type parameters in methods are not supported"
            );
        }

        if !sig.constness.is_none() {
            emit_error!(span, "const methods are not supported");
        }
    }

    /// Extracts the method arguments and return type.
    fn extract_args(sig: &syn::Signature) -> (Vec<(syn::Ident, syn::Type)>, syn::Type) {
        let args = sig
            .inputs
            .clone()
            .into_iter()
            .filter_map(|arg| {
                let syn::FnArg::Typed(arg_ty) = arg else {
                    // Skip receiver arg, ie. `&mut self`
                    return None;
                };

                let syn::Pat::Ident(pat_ty) = *arg_ty.pat else {
                    emit_error!(arg_ty, "expected named argument");
                    return None;
                };

                let ty = *arg_ty.ty;
                // TODO: better enforce that arg type is Sized + Send + 'static
                match ty {
                    syn::Type::Reference(_) | syn::Type::Slice(_) | syn::Type::TraitObject(_) => {
                        emit_error!(ty, "arguments must be Sized + Send + 'static");
                    }
                    _ => {}
                }

                let mut ident = pat_ty.ident.clone();
                ident.set_span(Span::call_site());

                Some((ident, ty))
            })
            .collect::<Vec<_>>();

        let return_ty = if let Some(ty) = extract_output(sig) {
            ty
        } else {
            emit_error!(sig, "method must be async or return a future.");
            parse_quote!(())
        };

        // TODO: better enforce that return type is Sized + Send + 'static
        match return_ty {
            syn::Type::Reference(_) | syn::Type::Slice(_) | syn::Type::TraitObject(_) => {
                emit_error!(return_ty, "return type must be Sized + Send + 'static");
            }
            _ => {}
        }

        (args, return_ty)
    }

    /// Extracts the type params from the parent item which are present in the method signature.
    fn extract_type_params<'a>(
        parent_type_params: &IdentSet,
        arg_tys: impl Iterator<Item = &'a syn::Type>,
        return_ty: &syn::Type,
    ) -> IdentSet {
        let arg_type_params = arg_tys
            .map(|ty| ty.uses_type_params_cloned(&Purpose::Declare.into(), parent_type_params))
            .fold(HashSet::default(), |mut acc, set| {
                acc.extend(set);
                acc
            });

        let return_type_params =
            return_ty.uses_type_params_cloned(&Purpose::Declare.into(), parent_type_params);

        // TODO: we could support this, but for now, no. It would require
        // storing a PhantomData in the message struct which is kind of
        // annoying.
        if !return_type_params.is_empty() && !arg_type_params.is_superset(&return_type_params) {
            emit_error!(
                return_ty,
                "generic param present in return type must also be present in argument types"
            );
        }

        arg_type_params
    }

    pub(crate) fn expand_message(&self) -> TokenStream {
        let Self {
            msg_options,
            vis,
            args,
            return_ty,
            type_params,
            struct_ident,
            ..
        } = self;

        if msg_options
            .as_ref()
            .map(|opts| opts.path.is_some() || opts.skip.is_present())
            .unwrap_or(false)
        {
            return TokenStream::new();
        }

        let type_params = type_params.iter().cloned().collect::<Vec<_>>();
        let arg_idents = args.iter().map(|(ident, _)| ident);
        let arg_tys = args.iter().map(|(_, ty)| ty);

        let vis = msg_options
            .as_ref()
            .map(|opts| opts.vis.clone())
            .flatten()
            .unwrap_or_else(|| vis.clone());

        let msg_attrs = msg_options
            .as_ref()
            .map(|opts| opts.attrs.as_ref().map(|attrs| attrs.clone().to_vec()))
            .flatten()
            .unwrap_or_default();

        let struct_body = if args.is_empty() {
            quote!(;)
        } else {
            quote!({ #( pub #arg_idents: #arg_tys ),* })
        };

        quote!(
            #( #[#msg_attrs] )*
            #vis struct #struct_ident<#(#type_params),*> #struct_body

            impl<#(#type_params),*> ::ludi::Message for #struct_ident<#(#type_params),*>
            where
                #(#type_params: Send + 'static),*
            {
                type Return = #return_ty;
            }

            impl<A, #(#type_params),*> ::ludi::Dispatch<A> for #struct_ident<#(#type_params),*>
            where
                A: ::ludi::Actor + ::ludi::Handler<#struct_ident<#(#type_params),*>>,
                #(#type_params: Send + 'static),*
            {
                async fn dispatch<R: FnOnce(#return_ty) + Send>(
                    self,
                    actor: &mut A,
                    ctx: &mut ::ludi::Context<A>,
                    ret: R,
                ) {
                    ::ludi::Handler::<#struct_ident<#(#type_params),*>>::process(
                        actor,
                        self,
                        ctx,
                        ret
                    ).await;
                }
            }
        )
    }

    pub fn expand_handler(&self, actor_path: &syn::Path, generics: &syn::Generics) -> TokenStream {
        let Self {
            attrs,
            msg_options,
            args,
            type_params,
            struct_path,
            body,
            ..
        } = self;

        if msg_options
            .as_ref()
            .map(|opts| opts.skip_handler.is_present())
            .unwrap_or(false)
        {
            return TokenStream::new();
        }

        let Some(body) = body else {
            panic!("expected method to have a body");
        };

        let arg_idents = args.iter().map(|(ident, _)| ident).collect::<Vec<_>>();
        let type_params = type_params.iter().cloned().collect::<Vec<_>>();

        let destructure = if arg_idents.is_empty() {
            quote!()
        } else {
            quote!(let #struct_path { #(#arg_idents),* } = msg;)
        };

        let (impl_generics, _, where_clause) = generics.split_for_impl();

        quote!(
            impl #impl_generics ::ludi::Handler<#struct_path<#(#type_params),*>> for #actor_path #where_clause {
                #(#attrs)*
                async fn handle(
                    &mut self,
                    msg: #struct_path<#(#type_params),*>,
                    ctx: &mut ::ludi::Context<Self>
                ) -> <#struct_path<#(#type_params),*> as ::ludi::Message>::Return {
                    #destructure
                    #body
                }
            }
        )
    }

    pub fn expand_ctrl(&self, is_trait: bool) -> TokenStream {
        let Self {
            doc_attrs,
            ctrl_options,
            struct_path,
            args,
            vis,
            sig,
            ..
        } = self;

        if ctrl_options.is_none() {
            return TokenStream::new();
        }

        let mut ctrl_sig = sig.clone();
        if let Some(syn::FnArg::Receiver(receiver)) = ctrl_sig.inputs.first_mut() {
            if receiver.reference.is_some() && !is_trait {
                *receiver = syn::parse_quote!(&self);
            }
        }

        let arg_idents = args.iter().map(|(ident, _)| ident);

        let struct_arg = if args.is_empty() {
            quote!(#struct_path)
        } else {
            quote!(#struct_path { #(#arg_idents),* })
        };

        let attrs = ctrl_options
            .as_ref()
            .map(|opts| opts.attrs.clone().map(|attrs| attrs.to_vec()))
            .flatten()
            .unwrap_or_default();

        let err_strategy = ctrl_options
            .as_ref()
            .map(|opts| opts.error_strategy())
            .unwrap_or_default();

        let err_handler = match err_strategy {
            ErrorStrategy::Panic => quote!(.expect("message should be handled to completion")),
            ErrorStrategy::Try => quote!(?),
            ErrorStrategy::Map(expr) => quote!(.map_err(#expr)?),
        };

        quote!(
            #(#doc_attrs)*
            #(#[#attrs])*
            #vis #ctrl_sig {
                self.addr.send(#struct_arg).await #err_handler
            }
        )
    }
}
