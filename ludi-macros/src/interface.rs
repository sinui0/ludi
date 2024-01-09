use darling::{ast::NestedMeta, Error, FromMeta};

use proc_macro::TokenStream;
use quote::quote;

use crate::items::ItemTrait;
use crate::options::MsgOptions;

#[derive(FromMeta)]
pub(crate) struct InterfaceAttr {
    pub msg: Option<MsgOptions>,
}

pub(crate) fn impl_interface(
    attr: TokenStream,
    mut item: syn::ItemTrait,
) -> proc_macro2::TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => {
            return Error::from(e).write_errors();
        }
    };

    let InterfaceAttr { msg } = match InterfaceAttr::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };

    let mut expanded_tokens = proc_macro2::TokenStream::new();

    {
        let item = ItemTrait::from_item_trait(&item, msg);

        expanded_tokens.extend(item.expand());
    }

    item.items.iter_mut().for_each(|item| {
        if let syn::TraitItem::Fn(f) = item {
            f.attrs.retain(|attr| !attr.path().is_ident("msg"));
        }
    });

    quote!(
        #item

        #expanded_tokens
    )
}
