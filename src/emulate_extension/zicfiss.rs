//! Emulation Zicfiss (Shadow Stack)
//! Ref: [https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf](https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf)

use crate::memmap::page_table::constants::PAGE_SIZE;
use crate::memmap::HostPhysicalAddress;
use crate::PageBlock;
use crate::HYPERVISOR_DATA;

use core::cell::OnceCell;
use raki::{Instruction, OpcodeKind, ZicfissOpcode};
use spin::Mutex;

/// Singleton for Zicfiss.
/// TODO: change `OnceCell` to `LazyCell`.
static mut ZICFISS_DATA: Mutex<OnceCell<Zicfiss>> = Mutex::new(OnceCell::new());

/// Shadow Stack
struct ShadowStack {
    top: *const usize,
    bottom: *const usize,
    stack_pointer: *mut usize,
}

impl ShadowStack {
    /// Allocate memory region for shadow stack.
    pub fn new() -> Self {
        let stack_addr = PageBlock::alloc();
        let base_ptr = stack_addr.0 as *const usize;
        ShadowStack {
            top: unsafe { base_ptr.byte_add(PAGE_SIZE) },
            bottom: base_ptr,
            stack_pointer: unsafe { base_ptr.cast_mut().byte_add(PAGE_SIZE) },
        }
    }

    /// Push value to shadow stack
    pub fn push(&mut self, value: usize) {
        unsafe {
            self.stack_pointer = self.stack_pointer.byte_sub(core::mem::size_of::<usize>());
            if self.stack_pointer.cast_const() < self.bottom {
                panic!("stack smashed!");
            }
            self.stack_pointer.write_volatile(value);
        }
    }

    /// Pop value from shadow stack
    pub fn pop(&mut self) -> usize {
        unsafe {
            self.stack_pointer = self.stack_pointer.byte_add(core::mem::size_of::<usize>());
            if self.stack_pointer.cast_const() > self.top {
                panic!("stack smashed!");
            }
            self.stack_pointer.read_volatile()
        }
    }

    pub fn get_ssp(&self) -> HostPhysicalAddress {
        HostPhysicalAddress(self.stack_pointer as usize)
    }
}

/// Singleton for Zicfiss extension
struct Zicfiss {
    /// Shadow stack
    pub shadow_stack: ShadowStack,
    /// Shadow Stack Enable
    ///
    /// TODO: handle xenvcfg register.
    /// TODO: devide into each priv.
    pub sse: bool,
}

impl Zicfiss {
    pub fn new() -> Self {
        Zicfiss {
            shadow_stack: ShadowStack::new(),
            sse: true,
        }
    }
}

/// Emulate Zicfiss instruction.
pub fn instruction(opc: ZicfissOpcode) {
    match opc {
        ZicfissOpcode::SSPUSH | ZicfissOpcode::C_SSPUSH => todo!(),
        ZicfissOpcode::SSPOPCHK | ZicfissOpcode::C_SSPOPCHK => todo!(),
        ZicfissOpcode::SSRDP => todo!(),
        ZicfissOpcode::SSAMOSWAP_W | ZicfissOpcode::SSAMOSWAP_D => todo!(),
    }
}
