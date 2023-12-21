use proc_macro::TokenStream;

// When the builder feature is disabled, just remove the entire thing.
pub fn recipe(_: TokenStream, _: TokenStream) -> TokenStream {
    TokenStream::new()
}

pub fn callback_helpers(item: TokenStream) -> TokenStream {
    item
}
