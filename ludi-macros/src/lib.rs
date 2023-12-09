mod implement;
mod interface;
pub(crate) mod types;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn interface(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut tokens = item.clone();
    let item_trait = syn::parse_macro_input!(item as syn::ItemTrait);

    tokens.extend(interface::impl_interface(item_trait));

    tokens
}

#[proc_macro_attribute]
pub fn implement(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = syn::parse_macro_input!(item as syn::ItemImpl);

    implement::impl_implement(item_impl)
}
