use proc_macro::TokenStream;
use quote::quote;

// When the builder feature is disabled, just remove the entire thing.
pub fn recipe(_: TokenStream, _: TokenStream) -> TokenStream {
    quote!().into()
}

pub fn callback_helpers(item: TokenStream) -> TokenStream {
    item
}
