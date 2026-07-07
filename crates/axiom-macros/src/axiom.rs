use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

pub fn impl_axiom(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let _name_str = name.to_string();
    let reg_fn = syn::Ident::new(&format!("__axiom_fn_{}", name), proc_macro2::Span::call_site());
    let reg_static =
        syn::Ident::new(&format!("__AXIOM_REG_{}", name), proc_macro2::Span::call_site());

    let expanded = quote! {
            #[derive(Debug)]
            #input

            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            pub static #reg_static: #name = #name;

            #[linkme::distributed_slice(::axiom_kernel::registry::AXIOM_REGISTRY)]
    #[linkme(crate = linkme)]
            #[doc(hidden)]
            pub static #reg_fn: &'static dyn ::axiom_kernel::axiom::DynAxiom = &#reg_static;
        };
    TokenStream::from(expanded)
}
