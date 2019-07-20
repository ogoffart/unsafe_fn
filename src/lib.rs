extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};
use quote::quote;

#[proc_macro_attribute]
pub fn unsafe_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn{ attrs, vis, constness, asyncness, unsafety, abi, ident, decl, block}
        = parse_macro_input!(item as ItemFn);
    assert!(unsafety.is_none());
    let syn::FnDecl { fn_token, generics,  paren_token, inputs, variadic, output} = *decl;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let call = inputs.pairs().fold(quote!(), |x, y| {
        let (it, sep) = y.into_tuple();
        if let syn::FnArg::Captured(syn::ArgCaptured{
            pat: syn::Pat::Ident(name), ..
        }) = it {
            quote!(#x #name #sep)
        } else {
            unimplemented!();
        }
    });

    let r = quote!{
        #(#attrs)* #vis #constness #asyncness unsafe #abi
        #fn_token #ident #impl_generics (#inputs #variadic) #output #where_clause  {
            #[inline] fn __unsafe_fn #impl_generics (#inputs #variadic) #output #where_clause {
                #block
            }
            __unsafe_fn(#call)
        }
    };
    println!("{}", r);
    r.into()
}


