# unsafe_fn

Attribute macro to mark a function as unsafe without its body
being unsafe

Marking a function with the `unsafe` keywords does two things:
 - The function may only be called from an `unsafe { ... }` block;
 - and the body of the function is itself wrapped in a `unsafe`
   block, so it can perform unsafe code.

In many case however, it is not desirable to have the full body
inside an `unsafe` block.

[RFC 2585](https://github.com/rust-lang/rfcs/pull/2585) discusses
that and suggests to no longer treat the body of a unsafe function
as unsafe.

In the mean time, this macro allows to declare a unsafe function
with a `#[unsafe_fn]` attribute, so that the function is unsafe,
but its body is not considered as unsafe.

```rust
use unsafe_fn::unsafe_fn;

#[unsafe_fn]
fn add_to_ptr(a_ptr: *const i32, b: i32) -> i32 {
    let a = unsafe { *a_ptr }; // dereference in a unsafe block
    a + b   // safe code outside of the unsafe block
}

let x = &42 as *const i32;
// The function is unsafe and must be called in a unsafe block;
assert_eq!(unsafe { add_to_ptr(x, 1) }, 43);
```

For consistency, it is also possible to use the `unsafe_fn` on traits
to declare an unsafe trait
```rust
// Equivalent to `unsafe trait UnsafeMarker {}`
#[unsafe_fn] trait UnsafeMarker {}
```

### Rationale

From the motivation section of
[RFC 2585](https://github.com/rust-lang/rfcs/pull/2585):
> Marking a function as `unsafe` is one of Rust's key protections against
> undefined behavior: Even if the programmer does not read the documentation,
> calling an `unsafe` function (or performing another unsafe operation)
> outside an unsafe block will lead to a compile error, hopefully followed
> by reading the documentation.
>
> However, we currently entirely lose this protection when writing an `unsafe fn`:
> If I, say, accidentally call offset instead of wrapping_offset [..] this happens
> without any further notice when I am writing an `unsafe fn` because the body of
> an `unsafe fn` is treated as an `unsafe` block.
>
> [...]
>
> Using some more formal terminology, an `unsafe` block generally comes with a
> proof _obligation_: The programmer has to ensure that this code is actually
> safe to execute in the current context, because the compiler just trusts the
> programmer to get this right. In contrast, `unsafe fn` represents an _assumption_:
> As the author of this function, I make some assumptions that I expect my callees
> to uphold.

In general, using an attribute instead of a keyword to mark unsafe function make
sense: the `unsafe` keyword would mean that the code is unsafe and extra care
need to be used when reviewing this code. While the attribute `#[unsafe_fn]` merly
declare a function as unsafe, but cannot by itself cause undefined behavior.

### Limitations

Due to a restriction in the way procedural macro works, there are a small limitation:

 1. associated functions of a generic type that reference neither `self` nor `Self`
cannot reference any of the generic type.

```rust
struct X<T>(T);
impl<T> X<T> {
    #[unsafe_fn] // ok: reference self
    fn get(&self) -> &T { &self.0 }

    // Error! no refernces to 'self' or 'Self', so T cannot be used
    #[unsafe_fn]
    fn identity(x : &T) -> &T { x }
// error[E0401]: can't use generic parameters from outer function
}
```

 2. Within trait implementation this only work if the trait function was also marked
 with #[unsafe_fn]

```rust
trait Tr {
    #[unsafe_fn] fn fn1(&self);
    unsafe fn fn2(&self);
}
impl Tr for u32 {
    #[unsafe_fn] fn fn1(&self) {} // Ok
    #[unsafe_fn] fn fn2(&self) {} // Error: fn2 is not declared with #[unsafe_fn]
// error[E0407]: method `__unsafe_fn_fn2` is not a member of trait `Tr`
}
```

License: MIT
