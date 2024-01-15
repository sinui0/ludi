use proc_macro_error::abort;
use quote::quote;

use crate::utils::ctrl_ident;

pub(crate) fn impl_controller(input: syn::DeriveInput) -> proc_macro2::TokenStream {
    let actor_ident = &input.ident;
    let vis = &input.vis;
    let ctrl_ident = ctrl_ident(actor_ident);

    if let Some(param) = input
        .generics
        .type_params()
        .find(|param| param.ident == "A")
    {
        abort!(
            param.ident,
            "generic type parameter `A` is reserved for the controller's address"
        );
    }

    let (actor_impl_generics, actor_ty_generics, actor_where_clause) =
        input.generics.split_for_impl();

    let mut generics = input.generics.clone();
    generics.params.push(syn::parse_quote!(CtrlMsg));

    let where_clause = generics.make_where_clause();
    where_clause
        .predicates
        .push(syn::parse_quote!(CtrlMsg: ::ludi::Message));

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = if input.generics.params.is_empty() {
        quote!(addr: ludi::Address<CtrlMsg>)
    } else {
        quote!(addr: ludi::Address<CtrlMsg>, _pd: std::marker::PhantomData #actor_ty_generics)
    };

    let from_fields = if input.generics.params.is_empty() {
        quote!(addr)
    } else {
        quote!(addr, _pd: std::marker::PhantomData)
    };

    let ctrl_doc = format!("[`{}`] controller.", actor_ident.to_string());
    let ctrl_fn_doc = format!("Create a new [`{}`] controller.", actor_ident.to_string());

    quote!(
        #[derive(Debug, Clone)]
        #[doc = #ctrl_doc]
        #vis struct #ctrl_ident #ty_generics where CtrlMsg: ::ludi::Message {
            #fields
        }

        impl #actor_impl_generics #actor_ident #actor_ty_generics #actor_where_clause {
            #[doc = #ctrl_fn_doc]
            pub fn controller<CtrlMsg>(addr: ::ludi::Address<CtrlMsg>) -> #ctrl_ident #ty_generics
            where
                CtrlMsg: ::ludi::Message + ::ludi::Dispatch<Self>,
            {
                #ctrl_ident ::from(addr)
            }
        }

        impl #impl_generics From<::ludi::Address<CtrlMsg>> for #ctrl_ident #ty_generics #where_clause
        {
            fn from(addr: ludi::Address<CtrlMsg>) -> Self {
                Self {
                    #from_fields
                }
            }
        }
    )
}
