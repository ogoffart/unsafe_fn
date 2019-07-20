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


License: MIT
