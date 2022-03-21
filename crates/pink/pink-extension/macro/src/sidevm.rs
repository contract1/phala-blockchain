use super::*;

pub(crate) fn patch(input: TokenStream2) -> TokenStream2 {
    match patch_or_err(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn patch_or_err(input: TokenStream2) -> Result<TokenStream2> {
    use proc_macro2::{Ident, Literal, Span};
    let trait_item: syn::ItemTrait = syn::parse2(input.clone())?;
    let impl_itent = Ident::new(&format!("{}Impl", trait_item.ident), Span::call_site());

    // for item in trait_item.items.iter_mut() {
    //     if let syn::TraitItem::Method(item_method) = item {
    //         item_method.sig.inputs.insert(
    //             0,
    //             syn::parse_quote! {
    //                 &self
    //             },
    //         );
    //         item_method.sig.output = match item_method.sig.output.clone() {
    //             syn::ReturnType::Type(_, tp) => {
    //                 syn::parse_quote! {
    //                     -> Result<#tp, Self::Error>
    //                 }
    //             }
    //             syn::ReturnType::Default => {
    //                 syn::parse_quote! {
    //                     -> Result<(), Self::Error>
    //                 }
    //             }
    //         };
    //     }
    // }
    Ok(quote! {
        #input

        pub struct #impl_itent;
        impl #(trait_item.ident) for #impl_itent {
            pub fn new() -> Self {
                Self
            }
        }
    })
}
