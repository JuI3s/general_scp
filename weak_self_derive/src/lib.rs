use proc_macro::TokenStream;
use quote::quote;
use syn;

use std::sync::{Arc, Mutex, Weak};

#[proc_macro_derive(WeakSelf)]
pub fn weak_self_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    // Build the trait implementation
    impl_weak_self_macro(&ast)
}
fn impl_weak_self_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl WeakSelf for #name {
            fn get_weak_self(&mut self) -> Weak<Mutex<&mut Self>> {
                let strong: Arc<Mutex<&mut Self>> = Arc::from(Mutex::new(self));
                let weak = Arc::downgrade(&strong);
                weak
            }
        }
    };
    gen.into()
}
