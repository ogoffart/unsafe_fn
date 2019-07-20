//! Attribute macro to mark a function as unsafe without its body
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
//! For consistency, it is also possible to use the `unsafe_fn` on traits
//! to declare an unsafe trait
//! ```rust
//! # use unsafe_fn::unsafe_fn;
//! // Equivalent to `unsafe trait UnsafeMarker {}`
//! #[unsafe_fn] trait UnsafeMarker {}
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
//! ## Limitations
//!
//! Associated functions of a generic type that reference neither `self` nor `Self`
//! cannot reference any of the generic type.
//!
//! ```ignore
//! # use unsafe_fn::unsafe_fn;
//! struct X<T>(T);
//! impl<T> X<T> {
//!     #[unsafe_fn] // ok: reference self
//!     fn get(&self) -> &T { &self.0 }
//!
//!     // Error! no refernces to 'self' or 'Self', T cannot be used
//!     #[unsafe_fn]
//!     fn identity(x : &T) -> &T { x }
//! }
//! ```

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{ext::IdentExt, fold::Fold, punctuated::Punctuated, spanned::Spanned, visit::Visit, *};

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

struct HasSelfType(bool);
impl<'ast> Visit<'ast> for HasSelfType {
    fn visit_ident(&mut self, i: &'ast Ident) {
        if i == "Self" {
            self.0 = true;
        }
    }

    fn visit_item(&mut self, _: &'ast Item) {
        // Do not recurse in other items
    }
}

enum Kind {
    UnsafeFn,
    SafeBody,
}

/// Mark a function as unsafe without its body being in an unsafe block
///
/// See [crate documentation](index.html)
#[proc_macro_attribute]
pub fn unsafe_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    match item {
        Item::Fn(f) => unsafe_fn_impl(f, Kind::UnsafeFn),
        Item::Trait(t) => quote!(unsafe #t).into(),
        _ => Error::new(
            item.span(),
            "#[unsafe_fn] can only be applied to functions or traits",
        )
        .to_compile_error()
        .into(),
    }
}

/// Make the body of an unsafe function not allowed to call unsafe code without
/// adding unsafe blocks
///
/// This macro can be applied to a unsafe function so that its body is not
/// considered as an unsafe block
///
/// ```rust
/// use unsafe_fn::safe_body;
///
/// #[safe_body]
/// unsafe fn add_to_ptr(a_ptr: *const i32, b: i32) -> i32 {
///     let a = unsafe { *a_ptr }; // dereference in a unsafe block
///     a + b // safe code outside of the unsafe block
/// }
/// ```
#[proc_macro_attribute]
pub fn safe_body(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    unsafe_fn_impl(item, Kind::SafeBody)
}

fn unsafe_fn_impl(
    ItemFn {
        attrs,
        vis,
        constness,
        asyncness,
        unsafety,
        abi,
        ident,
        decl,
        block,
    }: ItemFn,
    k: Kind,
) -> TokenStream {
    let unsafety = match (k, unsafety) {
        (Kind::UnsafeFn, None) => <Token![unsafe]>::default(),
        (Kind::SafeBody, Some(u)) => u,
        (Kind::UnsafeFn, Some(u)) => {
            return Error::new(u.span(), "#[unsafe_fn] already marked unsafe")
                .to_compile_error()
                .into()
        }
        (Kind::SafeBody, None) => {
            return Error::new(
                proc_macro::Span::call_site().into(),
                "#[safe_body] function must be marked as unsafe",
            )
            .to_compile_error()
            .into()
        }
    };

    let FnDecl {
        fn_token,
        generics,
        paren_token: _paren_token,
        inputs,
        variadic,
        output,
    } = &*decl;
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let mut main_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_args = Punctuated::<Ident, Token!(,)>::new();
    let mut wrap_self = false;

    for it in inputs.iter() {
        match it {
            FnArg::SelfRef(_) | FnArg::SelfValue(_) => {
                sub_param.push(it.clone());
                main_param.push(RemoveMut.fold_fn_arg(it.clone()));
                wrap_self = true;
            }
            FnArg::Captured(ArgCaptured {
                pat,
                colon_token,
                ty,
            }) => {
                if let Pat::Ident(i) = pat {
                    main_param.push(RemoveMut.fold_fn_arg(it.clone()));
                    sub_param.push(it.clone());
                    if i.ident == "self" {
                        wrap_self = true;
                    } else {
                        sub_args.push(i.ident.clone());
                    }
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

    let unsafe_fn_name = Ident::new(
        &format!("__unsafe_fn_{}", ident.unraw().to_string()),
        ident.span(),
    );

    let fun = quote! {
        #[doc(hide)]
        #[inline]
        #constness #asyncness #fn_token #unsafe_fn_name #impl_generics (#sub_param #variadic) #output #where_clause {
            #block
        }
    };

    let fdecl = quote! {
        #(#attrs)* #vis #constness #asyncness #unsafety #abi
        #fn_token #ident #impl_generics (#main_param #variadic) #output #where_clause
    };

    let type_params: Vec<_> = generics.type_params().map(|x| &x.ident).collect();
    let turbo = if type_params.is_empty() {
        quote!()
    } else {
        quote!(::< #(#type_params),* >)
    };

    let r = if wrap_self {
        quote! {
            #fun
            #fdecl {
                self.#unsafe_fn_name #turbo (#sub_args)
            }
        }
    } else if {
        let mut has_self = HasSelfType(false);
        has_self.visit_fn_decl(&*decl);
        has_self.visit_block(&block);
        has_self.0
    } {
        quote! {
            #fun
            #fdecl {
                Self::#unsafe_fn_name #turbo (#sub_args)
            }
        }
    } else {
        quote!(
            #fdecl {
                #fun
                #unsafe_fn_name #turbo (#sub_args)
            }
        )
    };
    //println!("{}", r);
    r.into()
}
