use darling::util::Override;
use darling::{ast::NestedMeta, Error, FromMeta};
use proc_macro::TokenStream;

use crate::items::ItemImpl;
use crate::options::{CtrlOptions, MsgOptions};

#[derive(FromMeta)]
pub(crate) struct ImplementAttr {
    pub msg: Option<Override<MsgOptions>>,
    pub ctrl: Option<Override<CtrlOptions>>,
}

pub(crate) fn impl_implement(attr: TokenStream, item: syn::ItemImpl) -> proc_macro2::TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => {
            return Error::from(e).write_errors();
        }
    };

    let ImplementAttr { msg, ctrl } = match ImplementAttr::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };

    let msg_options = msg.map(|msg_options| msg_options.unwrap_or_default());
    let ctrl_options = ctrl.map(|ctrl_options| ctrl_options.unwrap_or_default());

    let item = ItemImpl::from_item_impl(&item, msg_options, ctrl_options);

    item.expand()
}
