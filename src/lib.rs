extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{*, punctuated::Punctuated, spanned::Spanned};
use quote::quote;

#[proc_macro_attribute]
pub fn unsafe_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn{ attrs, vis, constness, asyncness, unsafety, abi, ident, decl, block}
        = parse_macro_input!(item as ItemFn);
    assert!(unsafety.is_none());
    let FnDecl { fn_token, generics,  paren_token, inputs, variadic, output} = *decl;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut main_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_args = Punctuated::<Ident, Token!(,)>::new();

    for x in inputs.pairs() {
        let (it, sep) = x.into_tuple();
        match it {
            FnArg::SelfRef(a) => {
                unimplemented!();
            }
            FnArg::SelfValue(a) => {
                unimplemented!();
            }
            FnArg::Captured(ArgCaptured{ pat, colon_token, ty }) => {
                if let Pat::Ident(i) = pat {
                    main_param.push(it.clone());
                    sub_param.push(it.clone());
                    sub_args.push(i.ident.clone());
                } else {
                    let name = Ident::new(&format!("unsafe_fn_arg{}", sub_args.len()), sep.span());
                    main_param.push(parse(quote!(#name #colon_token #ty).into()).unwrap());
                    sub_param.push(it.clone());
                    sub_args.push(name);
                }
            }
            FnArg::Inferred(a) => {
                unimplemented!();
            }
            FnArg::Ignored(_) => {
                main_param.push(it.clone());
            }
        }
    }

    let r = quote!{
        #(#attrs)* #vis #constness #asyncness unsafe #abi
        #fn_token #ident #impl_generics (#main_param #variadic) #output #where_clause  {
            #[inline] fn __unsafe_fn #impl_generics (#sub_param #variadic) #output #where_clause {
                #block
            }
            __unsafe_fn(#sub_args)
        }
    };
    println!("{}", r);
    r.into()
}


