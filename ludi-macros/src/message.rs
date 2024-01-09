use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, DeriveInput};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(ludi))]
struct Message {
    ident: syn::Ident,
    generics: syn::Generics,
    return_ty: Option<syn::Path>,
}

pub(crate) fn impl_message(input: DeriveInput) -> TokenStream {
    let Message {
        ident,
        mut generics,
        return_ty,
    } = match Message::from_derive_input(&input) {
        Ok(msg) => msg,
        Err(e) => return e.with_span(&input).write_errors(),
    };

    let generic_params = generics.params.clone();
    let where_clause = generics.make_where_clause();
    for param in generic_params {
        where_clause
            .predicates
            .push(parse_quote!(#param: Send + 'static));
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut dispatch_generics = generics.clone();
    dispatch_generics.params.push(parse_quote!(A));

    let dispatch_where = dispatch_generics.make_where_clause();
    dispatch_where
        .predicates
        .push(parse_quote!(A: ::ludi::Actor));
    dispatch_where
        .predicates
        .push(parse_quote!(A: ::ludi::Handler<#ident #ty_generics>));

    let (dispatch_generics, _, dispatch_where) = dispatch_generics.split_for_impl();

    let return_ty = if let Some(path) = return_ty {
        quote!(#path)
    } else {
        quote!(())
    };

    quote!(
        impl #impl_generics ::ludi::Message for #ident #ty_generics #where_clause {
            type Return = #return_ty;
        }

        impl #dispatch_generics ::ludi::Dispatch<A> for #ident #ty_generics #dispatch_where
        {
            async fn dispatch<R: FnOnce(Self::Return) + Send>(
                self,
                actor: &mut A,
                ctx: &mut ::ludi::Context<A>,
                ret: R,
            ) {
                actor.process(self, ctx, ret).await;
            }
        }
    )
}
