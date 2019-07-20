//! Attribute macro to mak a function as unsafe without its body
//! being unsafe
//!
//! Marking a function with the `unsafe` keywords does two things:
//!  - The function may only be called from an `unsafe { ... }` block;
//!  - and the body of the function is itself wrapped in a `unsafe`
//!    block, so it can perform unsafe code.
//!
//! In many case however, it is not desirable to have the full body
//! inside an `unsafe` block.
//!
//! [RFC 2585](https://github.com/rust-lang/rfcs/pull/2585) discusses
//! that and suggests to no longer treat the body of a unsafe function
//! as unsafe.
//!
//! In the mean time, this macro allows to declare a unsafe function
//! with a `#[unsafe_fn]` attribute, so that the function is unsafe,
//! but its body is not considered as unsafe.
//!
//! ```rust
//! use unsafe_fn::unsafe_fn;
//!
//! #[unsafe_fn]
//! fn add_to_ptr(a_ptr: *const i32, b: i32) -> i32 {
//!     let a = unsafe { *a_ptr }; // dereference in a unsafe block
//!     a + b   // safe code outside of the unsafe block
//! }
//!
//! let x = &42 as *const i32;
//! // The function is unsafe and must be called in a unsafe block;
//! assert_eq!(unsafe { add_to_ptr(x, 1) }, 43);
//! ```
//!
//! ## Rationale
//!
//! From the motivation section of
//! [RFC 2585](https://github.com/rust-lang/rfcs/pull/2585):
//! > Marking a function as `unsafe` is one of Rust's key protections against
//! > undefined behavior: Even if the programmer does not read the documentation,
//! > calling an `unsafe` function (or performing another unsafe operation)
//! > outside an unsafe block will lead to a compile error, hopefully followed
//! > by reading the documentation.
//! >
//! > However, we currently entirely lose this protection when writing an `unsafe fn`:
//! > If I, say, accidentally call offset instead of wrapping_offset [..] this happens
//! > without any further notice when I am writing an `unsafe fn` because the body of
//! > an `unsafe fn` is treated as an `unsafe` block.
//! >
//! > [...]
//! >
//! > Using some more formal terminology, an `unsafe` block generally comes with a
//! > proof _obligation_: The programmer has to ensure that this code is actually
//! > safe to execute in the current context, because the compiler just trusts the
//! > programmer to get this right. In contrast, `unsafe fn` represents an _assumption_:
//! > As the author of this function, I make some assumptions that I expect my callees
//! > to uphold.
//!
//! In general, using an attribute instead of a keyword to mark unsafe function make
//! sense: the `unsafe` keyword would mean that the code is unsafe and extra care
//! need to be used when reviewing this code. While the attribute `#[unsafe_fn]` merly
//! declare a function as unsafe, but cannot by itself cause undefined behavior.
//!

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{fold::Fold, punctuated::Punctuated, spanned::Spanned, *};

struct RemoveMut;
impl Fold for RemoveMut {
    fn fold_pat_ident(&mut self, mut i: PatIdent) -> PatIdent {
        i.mutability = None;
        i
    }
    fn fold_arg_self(&mut self, mut i: ArgSelf) -> ArgSelf {
        i.mutability = None;
        i
    }
}

/// Mark a function as unsafe without its body being in an unsafe block
///
/// See [crate documentation](index.html)
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
    if unsafety.is_some() {
        return Error::new(unsafety.span(), "#[unsafe_fn] already marked unsafe")
            .to_compile_error()
            .into();
    }
    let FnDecl {
        fn_token,
        generics,
        paren_token: _paren_token,
        inputs,
        variadic,
        output,
    } = *decl;
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let mut main_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_args = Punctuated::<Ident, Token!(,)>::new();
    let mut wrap_self = false;

    for it in inputs.iter() {
        match it {
            FnArg::SelfRef(_) | FnArg::SelfValue(_) => {
                sub_param.push(it.clone());
                main_param.push(fold::fold_fn_arg(&mut RemoveMut, it.clone()));
                wrap_self = true;
            }
            FnArg::Captured(ArgCaptured {
                pat,
                colon_token,
                ty,
            }) => {
                if let Pat::Ident(i) = pat {
                    main_param.push(fold::fold_fn_arg(&mut RemoveMut, it.clone()));
                    sub_param.push(it.clone());
                    sub_args.push(i.ident.clone());
                } else {
                    let name = Ident::new(&format!("__unsafe_fn_arg{}", sub_args.len()), it.span());
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
        #constness #asyncness #fn_token #unsafe_fn_name #impl_generics (#sub_param #variadic) #output #where_clause {
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
    //println!("{}", r);
    r.into()
}
