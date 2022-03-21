use super::*;

pub(crate) fn patch(input: TokenStream2) -> TokenStream2 {
    match patch_or_err(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn patch_or_err(input: TokenStream2) -> Result<TokenStream2> {
    use proc_macro2::{Ident, Literal, Span};

    let backend_trait = {
        let mut item_trait: syn::ItemTrait = syn::parse2(input.clone())?;

        item_trait.ident = syn::Ident::new(
            &format!("{}Backend", item_trait.ident),
            item_trait.ident.span(),
        );

        item_trait.items.retain(|i| {
            if let &syn::TraitItem::Type(_) = i {
                false
            } else {
                true
            }
        });

        item_trait.items.push(syn::parse_quote! {
            type Error;
        });

        for item in item_trait.items.iter_mut() {
            if let syn::TraitItem::Method(item_method) = item {
                item_method.attrs.clear();
                item_method.sig.inputs.insert(
                    0,
                    syn::parse_quote! {
                        &self
                    },
                );
                item_method.sig.output = match item_method.sig.output.clone() {
                    syn::ReturnType::Type(_, tp) => {
                        syn::parse_quote! {
                            -> Result<#tp, Self::Error>
                        }
                    }
                    syn::ReturnType::Default => {
                        syn::parse_quote! {
                            -> Result<(), Self::Error>
                        }
                    }
                };
            }
        }

        item_trait
    };

    let extension = ChainExtension::new(Default::default(), input.clone())?;
    let id_pairs: Vec<_> = {
        extension
            .iter_methods()
            .map(|m| {
                let name = m.ident().to_string();
                let id = m.id().into_u32();
                (name, id)
            })
            .collect()
    };

    // Extract all function ids to a sub module
    let func_ids = {
        let mut mod_item: syn::ItemMod = syn::parse_quote! {
            pub mod func_ids {}
        };
        for (name, id) in id_pairs.iter() {
            let name = name.to_uppercase();
            let name = Ident::new(&name, Span::call_site());
            let id = Literal::u32_unsuffixed(*id);
            mod_item
                .content
                .as_mut()
                .unwrap()
                .1
                .push(syn::parse_quote! {
                    pub const #name: u32 = #id;
                });
        }
        mod_item
    };

    // Generate the dispatcher
    let dispatcher: syn::ItemMacro = {
        let (names, ids): (Vec<_>, Vec<_>) = id_pairs
            .into_iter()
            .map(|(name, id)| {
                let name = Ident::new(&name, Span::call_site());
                let id = Literal::u32_unsuffixed(id);
                (name, id)
            })
            .unzip();
        syn::parse_quote! {
            #[macro_export]
            macro_rules! dispatch_ext_call {
                ($func_id: expr, $handler: expr, $env: expr) => {
                    match $func_id {
                        #(
                            #ids => {
                                let input = $env.read_as_unbounded($env.in_len())?;
                                let output = $handler.#names(input)?;
                                let output = output.encode();
                                Some(output)
                            }
                        )*
                        _ => None,
                    }
                };
            }
        }
    };

    // Mock helper functions
    let mock_helpers = {
        let mut mod_item: syn::ItemMod = syn::parse_quote! {
            pub mod mock {
                use super::*;
                use super::test::MockExtension;
            }
        };
        for m in extension.iter_methods() {
            let name = m.ident().to_string();
            let fname = "mock_".to_owned() + &name;
            let fname = Ident::new(&fname, Span::call_site());
            let id = Literal::u32_unsuffixed(m.id().into_u32());
            let input = match m.inputs().next() {
                Some(p0) => *p0.ty.clone(),
                None => syn::parse_quote!(),
            };
            let output = m.sig().output.clone();
            mod_item
                .content
                .as_mut()
                .unwrap()
                .1
                .push(syn::parse_quote! {
                    pub fn #fname(call: impl FnMut(#input) #output + 'static) {
                        ink_env::test::register_chain_extension(
                            MockExtension::<_, _, _, #id>::new(call),
                        );
                    }
                });
        }
        mod_item
    };

    let crate_ink_lang = find_crate_name("ink_lang")?;
    Ok(quote! {
        #[#crate_ink_lang::chain_extension]
        #input

        #backend_trait

        #func_ids

        #dispatcher

        #[cfg(feature = "std")]
        #mock_helpers
    })
}
