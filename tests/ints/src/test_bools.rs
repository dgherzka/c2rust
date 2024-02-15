use crate::bools::{rust_A, rust_B, rust_C};

pub fn test_bools() {
    unsafe {
        println!("rust_A = {rust_A}");
        println!("rust_B = {rust_B}");
        println!("rust_C = {rust_C}");
    }
}
