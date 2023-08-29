#![no_main]
#![no_std]

extern crate panic_halt;
mod init;

use riscv_rt::entry;

/// entry point
#[entry]
#[allow(clippy::empty_loop)]
fn main() -> ! {
    init::init();

    let uart = 0x1000_0000 as *mut u32;
    for c in b"Hello from Rust!\n".iter() {
        unsafe {
            while (uart.read_volatile() as i32) < 0 {}
            uart.write_volatile(*c as u32);
        }
    }

    loop {}
}
