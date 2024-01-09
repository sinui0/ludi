use proc_macro2::Span;
use syn::parse_quote;

/// Returns the identifier of the controller for the given actor.
pub(crate) fn ctrl_ident(actor_ident: &syn::Ident) -> syn::Ident {
    syn::Ident::new(
        &format!("{}Ctrl", actor_ident.to_string()),
        Span::call_site(),
    )
}

/// Extracts the output of an async function, returns `None` if the function is not async.
pub(crate) fn extract_output(sig: &syn::Signature) -> Option<syn::Type> {
    if sig.asyncness.is_some() {
        let return_ty = match sig.output.clone() {
            syn::ReturnType::Default => parse_quote!(()),
            syn::ReturnType::Type(_, ty) => *ty,
        };

        Some(return_ty)
    } else {
        let syn::ReturnType::Type(_, ty) = sig.output.clone() else {
            return None;
        };
        let ty = *ty;

        match ty {
            syn::Type::ImplTrait(ty) => {
                return ty.bounds.iter().find_map(|bound| {
                    if let syn::TypeParamBound::Trait(bound) = bound {
                        extract_fut_output(bound)
                    } else {
                        None
                    }
                });
            }
            syn::Type::Path(_) => {
                // TODO: Support boxed futures
                return None;
            }
            _ => return None,
        }
    }
}

fn extract_fut_output(bound: &syn::TraitBound) -> Option<syn::Type> {
    let segment = bound.path.segments.last()?;
    if segment.ident != "Future" {
        return None;
    }

    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };

    args.args.iter().find_map(|arg| {
        if let syn::GenericArgument::AssocType(assoc_ty) = arg {
            if assoc_ty.ident == "Output" {
                return Some(assoc_ty.ty.clone());
            }
        }

        None
    })
}

pub(crate) fn is_ludi_attr(attr: &syn::Attribute) -> bool {
    let Some(ident) = attr.meta.path().get_ident() else {
        return false;
    };

    match ident.to_string().as_str() {
        "msg" | "ctrl" => true,
        _ => false,
    }
}
