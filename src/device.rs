//! Devices data

pub mod clint;
mod initrd;
mod pci;
mod plic;
pub mod uart;
mod virtio;

use crate::memmap::page_table::PteFlag;
use crate::memmap::{page_table, HostPhysicalAddress, MemoryMap};
use crate::HypervisorData;
use alloc::vec::Vec;
use fdt::Fdt;

/// Page table for device
const PTE_FLAGS_FOR_DEVICE: [PteFlag; 6] = [
    PteFlag::Dirty,
    PteFlag::Accessed,
    PteFlag::Write,
    PteFlag::Read,
    PteFlag::User,
    PteFlag::Valid,
];

/// A struct that implement Device trait **must** has `base_addr` and size member.
pub trait Device {
    /// Create self instance.
    /// * `device_tree` - struct Fdt
    /// * `node_path` - node path in fdt
    fn new(device_tree: &Fdt, node_path: &str) -> Self;
    /// Return size of memory region.
    fn size(&self) -> usize;
    /// Return address of physical memory
    fn paddr(&self) -> HostPhysicalAddress;
    /// Return memory map between physical to physical
    fn memmap(&self) -> MemoryMap;
}

/// Manage devices sush as uart, plic, etc...
///
/// `memory_map` has memory region data of each devices.  
/// Each devices **must** be implemented Device trait.
#[derive(Debug)]
pub struct Devices {
    pub uart: uart::Uart,
    pub virtio: Vec<virtio::VirtIO>,
    pub initrd: initrd::Initrd,
    pub plic: plic::Plic,
    pub plic_context: usize,
    pub clint: clint::Clint,
    pub pci: pci::Pci,
}

impl Devices {
    pub fn device_mapping_g_stage(&self, page_table_start: HostPhysicalAddress) {
        let memory_map = self.create_device_map();
        page_table::sv39x4::generate_page_table(page_table_start, &memory_map);
    }

    fn create_device_map(&self) -> Vec<MemoryMap> {
        let mut device_mapping: Vec<MemoryMap> = self
            .virtio
            .iter()
            .flat_map(|virt| [virt.memmap(), virt.memmap()])
            .collect();

        device_mapping.extend_from_slice(&[
            self.uart.memmap(),
            self.initrd.memmap(),
            self.plic.memmap(),
            self.clint.memmap(),
            self.pci.memmap(),
        ]);

        device_mapping
    }
}

impl HypervisorData {
    /// Set device data.
    ///
    /// It replace None (uninit value) to `Some(init_device)`.
    ///
    /// # Panics
    /// It will be panic when parsing device tree failed.
    pub fn register_devices(&mut self, device_tree: Fdt) {
        self.devices.replace(Devices {
            uart: uart::Uart::new(&device_tree, "/soc/serial"),
            virtio: virtio::VirtIO::new_all(&device_tree, "/soc/virtio_mmio"),
            initrd: initrd::Initrd::new(&device_tree, "/chosen"),
            plic: plic::Plic::new(&device_tree, "/soc/plic"),
            plic_context: device_tree
                .find_node("/cpus/cpu")
                .unwrap()
                .children()
                .next() // interrupt-controller
                .unwrap()
                .property("phandle")
                .unwrap()
                .value[0] as usize,
            clint: clint::Clint::new(&device_tree, "/soc/clint"),
            pci: pci::Pci::new(&device_tree, "/soc/pci"),
        });
    }
}