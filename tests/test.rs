#![deny(unused_unsafe)]

use unsafe_fn::unsafe_fn;


#[unsafe_fn]
fn hello(x : u32, foo: String) -> u32 {
    let y : u32 = unsafe { std::mem::zeroed() };
    y + x + foo.len() as u32
}

#[unsafe_fn]
fn plus_one(val : u32, _x : String) -> u32 {
    let y : u32 = unsafe { std::mem::zeroed() };
    y + val + 1
}

fn main() {
    assert_eq!(unsafe{hello(42, "XYZ".into())}, 42 + 3);
    assert_eq!(unsafe{plus_one(42, "XYZ".into())}, 42 + 1);
}
