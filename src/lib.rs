extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, spanned::Spanned, *};

#[proc_macro_attribute]
pub fn unsafe_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        attrs,
        vis,
        constness,
        asyncness,
        unsafety,
        abi,
        ident,
        decl,
        block,
    } = parse_macro_input!(item as ItemFn);
    assert!(unsafety.is_none());
    let FnDecl {
        fn_token,
        generics,
        paren_token: _,
        inputs,
        variadic,
        output,
    } = *decl;
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let mut main_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_args = Punctuated::<Ident, Token!(,)>::new();
    let mut wrap_self = false;

    for x in inputs.pairs() {
        let (it, sep) = x.into_tuple();
        match it {
            FnArg::SelfRef(_) | FnArg::SelfValue(_) => {
                sub_param.push(it.clone());
                main_param.push(it.clone());
                wrap_self = true;
            }
            FnArg::Captured(ArgCaptured {
                pat,
                colon_token,
                ty,
            }) => {
                if let Pat::Ident(i) = pat {
                    main_param.push(it.clone());
                    sub_param.push(it.clone());
                    sub_args.push(i.ident.clone());
                } else {
                    let name =
                        Ident::new(&format!("__unsafe_fn_arg{}", sub_args.len()), sep.span());
                    main_param.push(parse(quote!(#name #colon_token #ty).into()).unwrap());
                    sub_param.push(it.clone());
                    sub_args.push(name);
                }
            }
            FnArg::Inferred(_) => {
                unimplemented!();
            }
            FnArg::Ignored(_) => {
                main_param.push(it.clone());
            }
        }
    }

    let unsafe_fn_name = Ident::new(&format!("__unsafe_fn_{}", ident.to_string()), ident.span());

    let fun = quote! {
        #[doc(hide)]
        #[inline]
        fn #unsafe_fn_name #impl_generics (#sub_param #variadic) #output #where_clause {
            #block
        }
    };

    let type_params: Vec<_> = generics.type_params().map(|x| &x.ident).collect();
    let turbo = if type_params.is_empty() {
        quote!()
    } else {
        quote!(::< #(#type_params),* >)
    };

    let ctn = if wrap_self {
        quote!( self.#unsafe_fn_name #turbo (#sub_args) )
    } else {
        quote!( #unsafe_fn_name #turbo (#sub_args) )
    };

    let r = quote! {
        #fun
        #(#attrs)* #vis #constness #asyncness unsafe #abi
        #fn_token #ident #impl_generics (#main_param #variadic) #output #where_clause  {
            #ctn
        }
    };
    println!("{}", r);
    r.into()
}
