mod controller;
mod implement;
mod interface;
pub(crate) mod items;
mod message;
pub(crate) mod options;
pub(crate) mod utils;
pub(crate) mod wrap;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::DeriveInput;

#[proc_macro_error]
#[proc_macro_attribute]
pub fn interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_trait = syn::parse_macro_input!(item as syn::ItemTrait);

    interface::impl_interface(attr, item_trait).into()
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn implement(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = syn::parse_macro_input!(item as syn::ItemImpl);

    implement::impl_implement(attr, item_impl).into()
}

#[proc_macro_derive(Message, attributes(ludi))]
pub fn message(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    message::impl_message(input).into()
}

#[proc_macro_error]
#[proc_macro_derive(Wrap, attributes(ludi))]
pub fn wrap(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    wrap::impl_wrap(input).into()
}

#[proc_macro_error]
#[proc_macro_derive(Controller, attributes(ludi))]
pub fn controller(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    controller::impl_controller(input).into()
}
