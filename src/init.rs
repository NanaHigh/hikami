use core::arch::asm;

const DRAM_BASE: u64 = 0x8000_0000;
const PAGE_TABLE_BASE: u64 = 0x8020_0000;
const PAGE_TABLE_SIZE: u64 = 1024;
const STACK_BASE: u64 = 0x8030_0000;
const PA2VA_OFFSET: u64 = 0xffff_ffff_4000_0000;

/// Initialize CSRs, page tables, stack pointer
pub fn init() {
    let hart_id: u64;
    unsafe {
        // get hart id
        asm!("mv {}, a0", out(reg) hart_id);

        // debug output
        let uart = 0x1000_0000 as *mut u32;
        for c in b"hart_id: ".iter() {
            while (uart.read_volatile() as i32) < 0 {}
            uart.write_volatile(*c as u32);
        }
        uart.write_volatile(hart_id as u32 + '0' as u32);
        uart.write_volatile('\n' as u32);
    }

    // init stack pointer
    let stack_pointer = STACK_BASE + PA2VA_OFFSET;
    unsafe {
        asm!("mv sp, {}", in(reg) stack_pointer);
    }

    // init page tables
    let offset_from_dram_base = init as *const fn() as u64 - DRAM_BASE;
    let offset_from_dram_base_masked = (offset_from_dram_base >> 21) << 19;
    let page_table_start = PAGE_TABLE_BASE + offset_from_dram_base + hart_id * PAGE_TABLE_SIZE;
    for pt_num in 511..1024 {
        let pt_offset = (page_table_start + pt_num * 8) as *mut u64;
        unsafe {
            pt_offset.write_volatile(pt_offset.read_volatile() + offset_from_dram_base_masked);
        }
    }
}
