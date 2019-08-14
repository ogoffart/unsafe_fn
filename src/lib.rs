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
//! Due to a restriction in the way procedural macro works, there are a small limitation:
//!
//!  1. associated functions of a generic type that reference neither `self` nor `Self`
//! cannot reference any of the generic type.
//!
//! ```ignore
//! # use unsafe_fn::unsafe_fn;
//! struct X<T>(T);
//! impl<T> X<T> {
//!     #[unsafe_fn] // ok: reference self
//!     fn get(&self) -> &T { &self.0 }
//!
//!     // Error! no refernces to 'self' or 'Self', so T cannot be used
//!     #[unsafe_fn]
//!     fn identity(x : &T) -> &T { x }
//! // error[E0401]: can't use generic parameters from outer function
//! }
//! ```
//!
//!  2. Within trait implementation this only work if the trait function was also marked
//!  with #[unsafe_fn]
//!
//! ```ignore
//! # use unsafe_fn::unsafe_fn;
//! trait Tr {
//!     #[unsafe_fn] fn fn1(&self);
//!     unsafe fn fn2(&self);
//! }
//! impl Tr for u32 {
//!     #[unsafe_fn] fn fn1(&self) {} // Ok
//!     #[unsafe_fn] fn fn2(&self) {} // Error: fn2 is not declared with #[unsafe_fn]
//! // error[E0407]: method `__unsafe_fn_fn2` is not a member of trait `Tr`
//! }
//! ```

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{fold::Fold, punctuated::Punctuated, spanned::Spanned, visit::Visit, *};

struct RemoveMut;
impl Fold for RemoveMut {
    fn fold_pat_ident(&mut self, mut i: PatIdent) -> PatIdent {
        i.mutability = None;
        i
    }
    fn fold_receiver(&mut self, mut i: Receiver) -> Receiver {
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

struct FnOrMethod {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    block: Option<Block>,
    semi_token: Option<token::Semi>,
}

impl From<ItemFn> for FnOrMethod {
    fn from(itemfn: ItemFn) -> FnOrMethod {
        FnOrMethod {
            attrs: itemfn.attrs,
            vis: itemfn.vis,
            sig: itemfn.sig,
            block: Some(*itemfn.block),
            semi_token: None,
        }
    }
}

impl From<TraitItemMethod> for FnOrMethod {
    fn from(m: TraitItemMethod) -> FnOrMethod {
        FnOrMethod {
            attrs: m.attrs,
            vis: Visibility::Inherited,
            sig: m.sig,
            block: m.default,
            semi_token: m.semi_token,
        }
    }
}

/// Mark a function as unsafe without its body being in an unsafe block
///
/// See [crate documentation](index.html)
#[proc_macro_attribute]
pub fn unsafe_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(m) = parse::<TraitItemMethod>(item.clone()) {
        return unsafe_fn_impl(m.into(), Kind::UnsafeFn);
    }

    let item = parse_macro_input!(item as Item);
    match item {
        Item::Fn(f) => unsafe_fn_impl(f.into(), Kind::UnsafeFn),
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
    if let Ok(m) = parse::<TraitItemMethod>(item.clone()) {
        return unsafe_fn_impl(m.into(), Kind::SafeBody);
    }
    let item = parse_macro_input!(item as ItemFn);
    unsafe_fn_impl(item.into(), Kind::SafeBody)
}

fn unsafe_fn_impl(
    FnOrMethod {
        attrs,
        vis,
        sig,
        block,
        semi_token,
    }: FnOrMethod,
    k: Kind,
) -> TokenStream {
    let Signature {
        constness,
        asyncness,
        unsafety,
        abi,
        fn_token,
        ident,
        generics,
        paren_token: _paren_token,
        inputs,
        variadic,
        output,
    } = &sig;

    let unsafety = match (k, unsafety) {
        (Kind::UnsafeFn, None) => <Token![unsafe]>::default(),
        (Kind::SafeBody, Some(u)) => u.clone(),
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

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let unsafe_fn_name = format_ident!("__unsafe_fn_{}", ident);

    let block = match block {
        None => {
            // Trait method, just mark it as unsafe, but also create a dummy placeholder
            // function next to it so re-implementaiton works
            let inner_where = match &where_clause {
                Some(w) => quote!(#w, Self:Sized),
                None => quote!(where Self:Sized),
            };

            return quote!(
                #(#attrs)* #vis #constness #asyncness #unsafety #abi
                #fn_token #ident #impl_generics (#inputs #variadic) #output #where_clause
                #semi_token

                #[doc(hide)]
                #[inline]
                #constness #asyncness
                #fn_token #unsafe_fn_name #impl_generics (#inputs #variadic) #output #inner_where
                { ::std::panic!("Not to be called"); }
            )
            .into();
        }
        Some(block) => block,
    };

    let mut main_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_param = Punctuated::<FnArg, Token!(,)>::new();
    let mut sub_args = Punctuated::<Ident, Token!(,)>::new();
    let mut wrap_self = false;

    for it in inputs.iter() {
        match it {
            FnArg::Receiver(_) => {
                sub_param.push(it.clone());
                main_param.push(RemoveMut.fold_fn_arg(it.clone()));
                wrap_self = true;
            }
            FnArg::Typed(PatType {
                attrs,
                pat,
                colon_token,
                ty,
            }) => {
                if let Pat::Ident(i) = pat.as_ref() {
                    main_param.push(RemoveMut.fold_fn_arg(it.clone()));
                    sub_param.push(it.clone());
                    if i.ident == "self" {
                        wrap_self = true;
                    } else {
                        sub_args.push(i.ident.clone());
                    }
                } else {
                    let name = format_ident!("__unsafe_fn_arg{}", sub_args.len());
                    main_param
                        .push(parse(quote!(#(#attrs)$* #name #colon_token #ty).into()).unwrap());
                    sub_param.push(it.clone());
                    sub_args.push(name);
                }
            }
        }
    }

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
        has_self.visit_signature(&sig);
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
