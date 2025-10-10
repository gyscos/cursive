use proc_macro::TokenStream;

// When the builder feature is disabled, just remove the entire thing.
pub fn blueprint(_: TokenStream, _: TokenStream) -> TokenStream {
    TokenStream::new()
}

// Just return the annotated function unchanged.
pub fn callback_helpers(item: TokenStream) -> TokenStream {
    item
}
