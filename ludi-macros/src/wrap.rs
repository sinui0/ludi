use std::collections::HashSet;

use darling::{
    ast::{Data, Fields},
    FromDeriveInput, FromField, FromVariant, Error, error::Accumulator,
};
use quote::{quote, ToTokens};
use proc_macro2::TokenStream;
use syn::{DeriveInput, parse_quote};

pub(crate) fn impl_wrap(input: DeriveInput) -> TokenStream {
    let wrap = match Wrap::from_derive_input(&input) {
        Ok(msg) => msg,
        Err(e) => return e.with_span(&input).write_errors(),
    };

    quote!(#wrap)
}


#[derive(Debug, FromDeriveInput)]
#[darling(attributes(ludi), supports(enum_newtype), and_then = "Wrap::validate")]
pub(crate) struct Wrap {
    ident: syn::Ident,
    vis: syn::Visibility,
    generics: syn::Generics,
    data: Data<Variant, darling::util::Ignored>,
    #[darling(skip)]
    variants: Vec<Variant>,
}

impl Wrap {
    fn validate(mut self) -> Result<Self, Error> {
        let mut err = Accumulator::default();
        
        if self.generics.lifetimes().count() > 0 {
            err.push(
                Error::custom("wrapper can not be generic over lifetimes")
                    .with_span(&self.generics),
            );
        }

        if self.generics.const_params().count() > 0 {
            err.push(
                Error::custom("wrapper can not be generic over const parameters")
                    .with_span(&self.generics),
            );
        }

        self.variants = match &mut self.data {
            Data::Enum(variants) => std::mem::take(variants),
            Data::Struct(_) => panic!("expected darling to validate that the wrapper is an enum"),
        };

        let variant_tys = self.variants.iter().map(|variant| &variant.fields.fields[0].ty).collect::<HashSet<_>>();
        if variant_tys.len() != self.variants.len() {
            err.push(
                Error::custom("wrapper can not have duplicate variant types")
                    .with_span(&self),
            );
        }

        let type_params = self.generics.type_params().map(|param| &param.ident).collect::<HashSet<_>>();
        variant_tys.iter().for_each(|ty| {
            if let syn::Type::Path(path) = ty {
                if path.path.segments.len() == 1 {
                    let ident = &path.path.segments[0].ident;
                    if type_params.contains(ident) {
                        err.push(
                            Error::custom("wrapper can not have generic variants")
                                .with_span(&ty),
                        );
                    }
                }
            }
        });

        err.finish()?;

        Ok(self)
    }
}

impl ToTokens for Wrap {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            ident,
            vis,
            generics,
            variants,
            ..
        } = self;

        let mut generics = generics.clone();
        let type_params = generics.type_params().map(|param| param.ident.clone()).collect::<Vec<_>>();
        
        let where_clause = generics.make_where_clause();
        for param in type_params {
            where_clause
                .predicates
                .push(parse_quote!(#param: Send + 'static));
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    
        let return_ident = syn::Ident::new(&format!("{}Return", ident), ident.span());
        let (variant_idents, variant_tys) = variants.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut idents, mut tys), variant| {
                idents.push(variant.ident.clone());
                tys.push(variant.fields.fields[0].ty.clone());
    
                (idents, tys)
            },
        );
    
        tokens.extend(quote!(
            impl #impl_generics ::ludi::Message for #ident #ty_generics #where_clause {
                type Return = #return_ident #ty_generics;
            }
    
            #vis enum #return_ident #ty_generics #where_clause {
                #(
                    #variant_idents ( <#variant_tys as ::ludi::Message>::Return ),
                )*
            }
    
            #(
                impl #impl_generics From<#variant_tys> for #ident #ty_generics #where_clause {
                    fn from(value: #variant_tys) -> Self {
                        Self :: #variant_idents (value)
                    }
                }
            )*
    
            #(
                impl #impl_generics ::ludi::Wrap<#variant_tys> for #ident #ty_generics #where_clause {
                    fn unwrap_return(ret: Self::Return) -> Result<<#variant_tys as ::ludi::Message>::Return, ::ludi::MessageError> {
                        match ret {
                            Self::Return :: #variant_idents (value) => Ok(value),
                            _ => Err(::ludi::MessageError::Wrapper),
                        }
                    }
                }
            )*
        ));

        let mut generics = generics.clone();
        generics.params.push(parse_quote!(A));
        let where_clause = generics.make_where_clause();
        where_clause
            .predicates
            .push(parse_quote!(A: ::ludi::Actor));

        for variant_ty in variant_tys {
            where_clause
                .predicates
                .push(parse_quote!(#variant_ty: ::ludi::Dispatch<A>));
        }

        let (impl_generics, _, where_clause) = generics.split_for_impl();

        tokens.extend(quote!(
            impl #impl_generics ::ludi::Dispatch<A> for #ident #ty_generics #where_clause
            {
                async fn dispatch<R: FnOnce(Self::Return) + Send>(
                    self,
                    actor: &mut A,
                    ctx: &mut ::ludi::Context<A>,
                    ret: R,
                ) {
                    match self {
                        #(
                            #ident :: #variant_idents (msg) => {
                                msg.dispatch(actor, ctx, |value| ret(Self::Return :: #variant_idents (value))).await;
                            }
                        ),*
                    }
                }
            }
        ));
    }
}

#[derive(Debug, FromVariant)]
pub(crate) struct Variant {
    pub ident: syn::Ident,
    pub fields: Fields<Field>,
}

#[derive(Debug, FromField)]
pub(crate) struct Field {
    pub ty: syn::Type,
}
