#![feature(prelude_import)]
#![no_std]
#[prelude_import]
use core::prelude::rust_2021::*;
#[macro_use]
extern crate core;
#[macro_use]
extern crate compiler_builtins;
extern crate alloc;
use alloc::vec::Vec;
use scale::{Decode, Encode};
pub type ptr = i32;
pub type size = i32;
extern "C" {
    fn sidevm_ocall(
        func_id: u32,
        input: ptr,
        input_len: size,
        output: ptr,
        output_len_ptr: ptr,
    ) -> i32;
}
fn ocall<Input: Encode, Output: Decode>(func_id: u32, input: &Input) -> Output {
    let output_buffer = ::alloc::vec::from_elem(0u8, 1024 * 256);
    let input_buffer = input.encode();
    let input = &input_buffer[0] as *const u8 as ptr;
    let input_len = input_buffer.len() as size;
    let output = &output_buffer[0] as *const u8 as ptr;
    let mut output_len = output_buffer.len() as size;
    let output_len_ptr = &mut output_len as *mut size as ptr;
    let rv = unsafe { sidevm_ocall(func_id, input, input_len, output, output_len_ptr) };
    if rv != 0 {
        ::core::panicking::panic_fmt(::core::fmt::Arguments::new_v1(
            &["sidevm_ocall failed, error: "],
            &[::core::fmt::ArgumentV1::new_display(&rv)],
        ));
    }
    let mut output = &output_buffer[0..output_len as usize];
    Decode::decode(&mut output).expect("Failed to decode ocall output")
}
