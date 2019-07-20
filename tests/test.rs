#![deny(unused_unsafe)]

use unsafe_fn::unsafe_fn;

#[derive(Default)]
struct SomeStruct {
    i: u32,
    s: String,
}

#[unsafe_fn]
fn hello(x: u32, foo: String) -> u32 {
    let y: u32 = unsafe { std::mem::zeroed() };
    y + x + foo.len() as u32
}

#[unsafe_fn]
fn plus_one(mut val: u32, _x: String) -> u32 {
    let y: u32 = unsafe { std::mem::zeroed() };
    val += 1;
    y + val
}

#[unsafe_fn]
fn take_struct(
    SomeStruct { mut i, s }: SomeStruct,
    foo @ SomeStruct { .. }: &mut SomeStruct,
) -> SomeStruct {
    let y: u32 = unsafe { std::mem::zeroed() };
    foo.i += 1;
    i += foo.i;
    SomeStruct {
        i: i + y,
        s: s + &foo.s,
    }
}

impl SomeStruct {
    #[unsafe_fn]
    fn i_plus(&self, plus: u32) -> u32 {
        let y: u32 = unsafe { std::mem::zeroed() };
        self.i + plus + y
    }

    #[unsafe_fn]
    fn with_generic<'a, 'b, T: Clone>(&'a self, r: &'b T, _: u32, _: u32) -> (&'b T, T, &'a str)
    where
        (T, Self): Default,
    {
        let _: u32 = unsafe { std::mem::zeroed() };
        (r, r.clone(), &self.s)
    }

    #[unsafe_fn]
    fn take_self(mut self) -> Self {
        let y: u32 = unsafe { std::mem::zeroed() };
        self.i += 2 + y;
        self
    }
}

mod some_module {
    #[unsafe_fn::unsafe_fn]
    pub(crate) fn create_vec<T>() -> Vec<T> {
        let _: u32 = unsafe { std::mem::zeroed() };
        Vec::new()
    }

    #[unsafe_fn::unsafe_fn]
    pub fn size_plus<T>(x: usize) -> usize {
        let y: usize = unsafe { std::mem::zeroed() };
        x + y + std::mem::size_of::<T>()
    }
}

#[unsafe_fn]
#[no_mangle]
extern "C" fn deref_ptr(ptr: *const u32) -> u32 {
    unsafe { *ptr }
}

#[unsafe_fn]
fn with_return(x: &mut u32, cond: bool) {
    *x += 4;
    if cond {
        return;
    }
    *x += 8;
}

#[unsafe_fn]
trait Marker {}

unsafe impl Marker for SomeStruct {}

fn main() {
    assert_eq!(unsafe { hello(42, "XYZ".into()) }, 42 + 3);
    assert_eq!(unsafe { plus_one(42, "XYZ".into()) }, 42 + 1);
    let mut s1 = SomeStruct {
        i: 8,
        s: "DEF".into(),
    };
    let s2 = unsafe {
        take_struct(
            SomeStruct {
                i: 5,
                s: "ABC".into(),
            },
            &mut s1,
        )
    };
    assert_eq!(s1.i, 9);
    assert_eq!(s2.i, 5 + 9);
    assert_eq!(unsafe { s2.i_plus(58) }, 5 + 9 + 58);
    let x = 31;
    assert_eq!(unsafe { s2.with_generic(&x, 5, 8) }, (&x, x, "ABCDEF"));
    assert_eq!(unsafe { s2.take_self() }.i, 5 + 9 + 2);
    let _ = unsafe { some_module::create_vec::<u32>() };
    assert_eq!(unsafe { some_module::size_plus::<u32>(1) }, 4 + 1);
    assert_eq!(unsafe { deref_ptr(&x) }, 31);
    let mut m = 1;
    unsafe { with_return(&mut m, true) };
    assert_eq!(m, 1 + 4);
}
