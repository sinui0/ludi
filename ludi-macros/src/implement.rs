use heck::{ToSnakeCase, ToUpperCamelCase};
use quote::quote;
use syn::{parse_str, Ident, ItemImpl, Path};

use crate::types::MethodSig;

pub(crate) fn impl_implement(item: ItemImpl) -> proc_macro::TokenStream {
    let self_ty = *item.self_ty;
    let trait_path = item.trait_.expect("expected trait implementation").1;
    let msgs_module: Path = parse_str(&format!(
        "{}_msgs",
        quote!(#trait_path).to_string().to_snake_case()
    ))
    .unwrap();

    let (method_blocks, impl_blocks): (Vec<_>, Vec<_>) = item
        .items
        .into_iter()
        .filter_map(|item| match item {
            syn::ImplItem::Method(method) => Some(method),
            _ => None,
        })
        .map(|method| {
            let full_sig = method.sig.clone();
            let sig = MethodSig::from(method.sig);
            let msg_ident: Ident = parse_str(&sig.ident.to_string().to_upper_camel_case()).unwrap();
            let block = method.block;
            let ret = sig.ret;

            let structure = if sig.args.is_empty() {
                quote!(#msgs_module :: #msg_ident)
            } else {
                let arg_idents = sig.args.iter().map(|(ident, _)| ident);
                quote!(#msgs_module :: #msg_ident { #( #arg_idents ),* })
            };

            let method_block = quote!(
                #full_sig {
                    self.send(#structure).await
                }
            );

            let impl_block = quote!(
                impl ::ludi::Handler<#msgs_module :: #msg_ident> for #self_ty {
                    type Return = #ret;

                    async fn handle<M: ::ludi::Mailbox<Self>>(
                        &mut self,
                        msg: #msgs_module :: #msg_ident,
                        ctx: &mut ::ludi::Context<'_, Self, M>,
                    ) -> Self::Return {
                        let #structure = msg;

                        #block
                    }
                }
            );

            (method_block, impl_block)
        })
        .unzip();

    quote!(
        impl<A: ::ludi::Address<#self_ty>> #trait_path for A {
            #( #method_blocks )*
        }

        #( #impl_blocks )*
    )
    .into()
}
