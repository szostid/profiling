extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, ImplItem, ItemFn, ItemImpl};

#[proc_macro_attribute]
pub fn function(
    _attr: TokenStream,
    item: TokenStream,
) -> TokenStream {
    let mut function = parse_macro_input!(item as ItemFn);
    let instrumented_function_name = function.sig.ident.to_string();

    impl_block(
        &mut function.block,
        &mut function.attrs,
        &instrumented_function_name,
    );

    (quote! {
        #function
    })
    .into()
}

#[proc_macro_attribute]
pub fn skip(
    _attr: TokenStream,
    item: TokenStream,
) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn all_functions(
    _attr: TokenStream,
    item: TokenStream,
) -> TokenStream {
    let mut content = parse_macro_input!(item as ItemImpl);
    let struct_name = content.self_ty.to_token_stream().to_string();

    'func_loop: for block in &mut content.items {
        // Currently, we only care about the function impl part.
        // In the future, expand the code to following if we are interested in other parts
        //
        // match block {
        //     ImplItem::Fn(ref mut func) => {
        //         for func_attr in &func.attrs {
        //             if let syn::Meta::Path(ref func_attr_info) = func_attr.meta {
        //                 let attr_seg = func_attr_info.segments.last().unwrap();
        //                 if attr_seg.ident.to_string() == "skip".to_string() {
        //                     continue 'func_loop;
        //                 }
        //             }
        //         }
        //         let prev_block = &func.block;
        //         let func_name = func.sig.ident.to_string();
        //         func.block = impl_block(prev_block, &func_name);
        //     }
        //     ImplItem::Macro(_) => { // some code... },
        //     ImplItem::Type(_) => { // some code... },
        //     _ => {}
        // }
        let ImplItem::Fn(ref mut func) = block else {
            continue;
        };

        for func_attr in &func.attrs {
            let func_attr_info = func_attr.path();
            if func_attr_info.segments.is_empty() {
                continue;
            }
            if func_attr_info.segments.first().unwrap().ident != "profiling" {
                continue;
            }
            if func_attr_info.segments.last().unwrap().ident == "skip" {
                continue 'func_loop;
            }
        }
        let calling_info = format!("{}: {}", struct_name, func.sig.ident);
        impl_block(&mut func.block, &mut func.attrs, &calling_info);
    }

    (quote!(
        #content
    ))
    .into()
}

#[cfg(not(any(
    feature = "profile-with-puffin",
    feature = "profile-with-optick",
    feature = "profile-with-superluminal",
    feature = "profile-with-tracing",
    feature = "profile-with-tracy"
)))]
fn impl_block(
    _body: &mut syn::Block,
    _attrs: &mut Vec<syn::Attribute>,
    _instrumented_function_name: &str,
) {
}

#[cfg(any(
    feature = "profile-with-puffin",
    feature = "profile-with-optick",
    feature = "profile-with-superluminal",
    feature = "profile-with-tracy"
))]
fn impl_block(
    body: &mut syn::Block,
    _attrs: &mut Vec<syn::Attribute>,
    _instrumented_function_name: &str,
) {
    *body = syn::parse_quote! {
        {
            profiling::function_scope!();

            #body
        }
    }
}

#[cfg(feature = "profile-with-tracing")]
fn impl_block(
    _body: &mut syn::Block,
    attrs: &mut Vec<syn::Attribute>,
    instrumented_function_name: &str,
) {
    attrs.push(syn::parse_quote! {
        #[profiling::tracing::instrument(
            skip_all,
            name = #instrumented_function_name,
            level = profiling::tracing::Level::INFO,
        )]
    });
}
