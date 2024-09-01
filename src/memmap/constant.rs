//! Constant for memory map.
//!
//! # Host physical address
//! | start         | end           | region                            |
//! |---------------|---------------|-----------------------------------|
//! | `0x0200_0000` | `0x0210_0000` | QEMU CLINT                        |
//! | `0x0c00_0000` | `0x0c60_0000` | QEMU PLIC                         |
//! | `0x1000_0000` | `0x1000_0100` | QEMU UART                         |
//! | `0x1000_1000` | `0x1000_8000` | QEMU `VirtIO`                     |
//! |               |               |                                   |
//! | `0x8000_0000` | `0x8020_0000` | text and read only data of hikami |
//! | `0x8020_0000` |      ...      | bottom of stack                   |
//! |      ...      | `0x8080_0000` | machine stack                     |
//!
//! # Guest physical address
//! | start         | end           | region               |
//! |---------------|---------------|----------------------|
//! | `0x0200_0000` | `0x0210_0000` | QEMU CLINT           |
//! | `0x0c00_0000` | `0x0c60_0000` | QEMU PLIC            |
//! | `0x1000_0000` | `0x1000_0100` | QEMU UART            |
//! | `0x1000_1000` | `0x1000_8000` | QEMU VirtIO          |
//! |               |               |                      |
//! | `0x8000_0000` | `0x9000_0000` | text data of guest 1 |

/// Max number of HART
pub const MAX_HART_NUM: usize = 8;
/// Base address of dram.
pub const DRAM_BASE: usize = 0x8000_0000;
/// Stack size for each HART.
pub const STACK_SIZE_PER_HART: usize = 0x1_0000;

pub mod device {
    //! Device memory map

    use crate::memmap::HostPhysicalAddress;

    /// Uart address
    /// For println macro.
    pub const UART_ADDR: HostPhysicalAddress = HostPhysicalAddress(0x1000_0000);

    /// CLINT address
    /// For trap `SupervisorSoftware` interrupt
    pub const CLINT_ADDR: HostPhysicalAddress = HostPhysicalAddress(0x200_0000);

    /// mtimecmp CSRs address
    pub const MTIMECMP_ADDR: HostPhysicalAddress = HostPhysicalAddress(0x200_4000);
}

pub mod machine {
    //! Machine memory region (`0x8020_0000` - `0x8080_0000`)

    use crate::memmap::HostPhysicalAddress;

    /// Base address of machine stack.
    pub const STACK_BASE: HostPhysicalAddress = HostPhysicalAddress(0x8080_0000);
}

pub mod guest_memory {
    //! Guest memory region on Guest Physical Address

    use crate::memmap::GuestPhysicalAddress;

    /// Dram base address
    pub const DRAM_BASE: GuestPhysicalAddress = GuestPhysicalAddress(0x8000_0000);
    /// Dram memory space per HART.
    pub const DRAM_SIZE_PER_GUEST: usize = 256 * 1024 * 1024; // 256 MB
}
