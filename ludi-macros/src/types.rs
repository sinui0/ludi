use syn::{parse_quote, FnArg, Ident, Pat, ReturnType, Signature, Type};

#[derive(Clone)]
pub(crate) struct MethodSig {
    pub ident: Ident,
    pub args: Vec<(Ident, Type)>,
    pub ret: Type,
}

impl From<Signature> for MethodSig {
    fn from(sig: Signature) -> Self {
        let Signature {
            ident,
            generics,
            inputs,
            output,
            ..
        } = sig;

        let ret = match output {
            ReturnType::Default => parse_quote!(()),
            ReturnType::Type(_, ty) => *ty,
        };

        if !generics.params.is_empty() {
            panic!("generic methods are not supported");
        }

        let args = inputs
            .into_iter()
            .filter_map(|arg| {
                let FnArg::Typed(pat) = arg else {
                    return None;
                };

                let ty = *pat.ty;

                let Pat::Ident(pat) = *pat.pat else {
                    panic!("only support named arguments");
                };

                Some((pat.ident, ty))
            })
            .collect();

        Self { ident, args, ret }
    }
}
