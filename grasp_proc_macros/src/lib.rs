use proc_macro::{self, TokenStream};
use quote::quote;

fn impl_grasp_queue(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl GraspQueue for #name {
            fn get_queue_name(&self) -> String {
                stringify!(#name).to_string()
            }
        }
    };

    gen.into()
}

#[proc_macro_derive(GraspQueue)]
pub fn grasp_queue(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    impl_grasp_queue(&ast)
}
