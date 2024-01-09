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
    generics.params.push(syn::parse_quote!(A));

    let where_clause = generics.make_where_clause();
    where_clause
        .predicates
        .push(syn::parse_quote!(A: ::ludi::Address));
    where_clause
        .predicates
        .push(syn::parse_quote!(<A as ::ludi::Address>::Message: ::ludi::Dispatch<#actor_ident #actor_ty_generics>));

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = if input.generics.params.is_empty() {
        quote!(addr: A)
    } else {
        quote!(addr: A, _pd: std::marker::PhantomData #actor_ty_generics)
    };

    let from_fields = if input.generics.params.is_empty() {
        quote!(addr)
    } else {
        quote!(addr, _pd: std::marker::PhantomData)
    };

    quote!(
        #[derive(Debug, Clone)]
        #vis struct #ctrl_ident #ty_generics {
            #fields
        }

        impl #actor_impl_generics #actor_ident #actor_ty_generics #actor_where_clause {
            pub fn controller<A>(addr: A) -> #ctrl_ident #ty_generics
            where
                A: ::ludi::Address,
                <A as ::ludi::Address>::Message: ::ludi::Dispatch<Self>,
            {
                #ctrl_ident ::from(addr)
            }
        }

        impl #impl_generics From<A> for #ctrl_ident #ty_generics #actor_where_clause
        {
            fn from(addr: A) -> Self {
                Self {
                    #from_fields
                }
            }
        }

        impl #ty_generics ::ludi::Controller for #ctrl_ident #ty_generics #where_clause
        {
            type Actor = #actor_ident #actor_ty_generics;
            type Address = A;
            type Message = <A as ::ludi::Address>::Message;

            fn address(&self) -> &Self::Address {
                &self.addr
            }
        }
    )
}
