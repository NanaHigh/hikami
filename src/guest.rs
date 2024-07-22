//! Guest data of each HARTs.

pub mod context;

use crate::memmap::constant::{DRAM_BASE, DRAM_SIZE_PAR_HART, GUEST_TEXT_OFFSET};
use context::Context;
use elf::{endian::AnyEndian, ElfBytes};

/// Guest Information
#[derive(Debug, Default)]
pub struct Guest {
    /// Guest ID
    guest_id: usize,
    /// Guest context data
    pub context: Context,
}

impl Guest {
    pub fn new(hart_id: usize) -> Self {
        Guest {
            guest_id: hart_id,
            context: Context::default(),
        }
    }

    pub fn hart_id(&self) -> usize {
        self.guest_id
    }

    /// Return guest dram space start
    fn dram_base(&self) -> usize {
        DRAM_BASE + DRAM_SIZE_PAR_HART * (self.guest_id + 1)
    }

    /// Load an elf to guest memory space.
    ///
    /// It only load `PT_LOAD` type segments.
    /// Entry address is determined by ... .
    ///
    /// # Arguments
    /// * `guest_elf` - Elf loading guest space.
    /// * `elf_addr` - Elf address.
    pub fn load_guest_elf(&self, elf_addr: *mut u8, guest_initrd_size: usize) -> usize {
        let guest_elf = unsafe {
            ElfBytes::<AnyEndian>::minimal_parse(core::slice::from_raw_parts(
                elf_addr,
                guest_initrd_size,
            ))
            .unwrap()
        };
        let guest_base_addr = self.dram_base() + GUEST_TEXT_OFFSET;
        for prog_header in guest_elf
            .segments()
            .expect("failed to get segments from elf")
            .iter()
        {
            const PT_LOAD: u32 = 1;
            if prog_header.p_type == PT_LOAD && prog_header.p_filesz > 0 {
                unsafe {
                    core::ptr::copy(
                        elf_addr.wrapping_add(usize::try_from(prog_header.p_offset).unwrap()),
                        (guest_base_addr + usize::try_from(prog_header.p_paddr).unwrap())
                            as *mut u8,
                        usize::try_from(prog_header.p_filesz).unwrap(),
                    );
                }
            }
        }

        guest_base_addr
    }
}
