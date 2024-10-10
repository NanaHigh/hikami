//! Emulation Zicfiss (Shadow Stack)
//! Ref: [https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf](https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf)

use super::{pseudo_vs_exception, CsrData, EmulateExtension};
use crate::memmap::{
    page_table::{g_stage_trans_addr, vs_stage_trans_addr},
    GuestVirtualAddress,
};
use crate::HYPERVISOR_DATA;

use core::cell::OnceCell;
use raki::{Instruction, OpcodeKind, ZicfissOpcode, ZicsrOpcode};
use spin::Mutex;

/// Singleton for Zicfiss.
/// TODO: change `OnceCell` to `LazyCell`.
pub static mut ZICFISS_DATA: Mutex<OnceCell<Zicfiss>> = Mutex::new(OnceCell::new());

/// Software-check exception. (cause value)
const SOFTWARE_CHECK_EXCEPTION: usize = 18;
/// Shadow stack fault. (tval value)
const SHADOW_STACK_FAULT: usize = 3;

/// Singleton for Zicfiss extension
pub struct Zicfiss {
    /// Shadow stack pointer
    pub ssp: CsrData,
    /// Shadow Stack Enable in henvcfg (for VS-mode)
    pub henv_sse: bool,
    /// Shadow Stack Enable in senvcfg (for VU-mode)
    pub senv_sse: bool,
}

impl Zicfiss {
    pub fn new() -> Self {
        Zicfiss {
            ssp: CsrData(0),
            henv_sse: false,
            senv_sse: false,
        }
    }

    /// Return host physical shadow stack pointer as `*mut usize`.
    fn ssp_hp_ptr(&self) -> *mut usize {
        let gpa = vs_stage_trans_addr(GuestVirtualAddress(self.ssp.0 as usize));
        let hpa = g_stage_trans_addr(gpa);
        hpa.0 as *mut usize
    }

    /// Push value to shadow stack
    pub fn ss_push(&mut self, value: usize) {
        unsafe {
            self.ssp = CsrData(
                (self.ssp.0 as *const usize).byte_sub(core::mem::size_of::<usize>()) as u64,
            );
            self.ssp_hp_ptr().write_volatile(value);
        }
    }

    /// Pop value from shadow stack
    pub fn ss_pop(&mut self) -> usize {
        unsafe {
            let pop_value = self.ssp_hp_ptr().read_volatile();
            self.ssp = CsrData(
                (self.ssp.0 as *const usize).byte_add(core::mem::size_of::<usize>()) as u64,
            );

            pop_value
        }
    }

    fn is_ss_enable(&self, sstatus: usize) -> bool {
        let spp = sstatus >> 8 & 0x1;
        if spp == 0 {
            self.senv_sse
        } else {
            self.henv_sse
        }
    }
}

impl EmulateExtension for Zicfiss {
    /// Emulate Zicfiss instruction.
    fn instruction(&mut self, inst: Instruction) {
        let hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
        let mut context = hypervisor_data.get().unwrap().guest().context;
        let sstatus = context.sstatus();

        match inst.opc {
            OpcodeKind::Zicfiss(ZicfissOpcode::SSPUSH) => {
                if self.is_ss_enable(sstatus) {
                    let push_value = context.xreg(inst.rs2.unwrap());
                    self.ss_push(push_value as usize);
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::C_SSPUSH) => {
                if self.is_ss_enable(sstatus) {
                    let push_value = context.xreg(inst.rd.unwrap());
                    self.ss_push(push_value as usize);
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::SSPOPCHK) => {
                if self.is_ss_enable(sstatus) {
                    let pop_value = self.ss_pop();
                    let expected_value = context.xreg(inst.rs1.unwrap()) as usize;
                    if pop_value != expected_value {
                        drop(hypervisor_data);
                        pseudo_vs_exception(SOFTWARE_CHECK_EXCEPTION, SHADOW_STACK_FAULT)
                    }
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::C_SSPOPCHK) => {
                if self.is_ss_enable(sstatus) {
                    let pop_value = self.ss_pop();
                    let expected_value = context.xreg(inst.rd.unwrap()) as usize;
                    if pop_value != expected_value {
                        drop(hypervisor_data);
                        pseudo_vs_exception(SOFTWARE_CHECK_EXCEPTION, SHADOW_STACK_FAULT)
                    }
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::SSRDP) => {
                if self.is_ss_enable(sstatus) {
                    context.set_xreg(inst.rd.unwrap(), self.ssp.0 as u64);
                } else {
                    context.set_xreg(inst.rd.unwrap(), 0);
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::SSAMOSWAP_W | ZicfissOpcode::SSAMOSWAP_D) => todo!(),
            _ => todo!(),
        }
    }

    /// Emulate Zicfiss CSRs access.
    fn csr(&mut self, inst: Instruction) {
        const CSR_SSP: usize = 0x11;

        let hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
        let mut context = hypervisor_data.get().unwrap().guest().context;

        let csr_num = inst.rs2.unwrap();
        match csr_num {
            CSR_SSP => match inst.opc {
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRW) => {
                    let rs1 = context.xreg(inst.rs1.unwrap());
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.write(rs1);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRS) => {
                    let rs1 = context.xreg(inst.rs1.unwrap());
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.set(rs1);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRC) => {
                    let rs1 = context.xreg(inst.rs1.unwrap());
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.clear(rs1);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRWI) => {
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.write(inst.rs1.unwrap() as u64);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRSI) => {
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.set(inst.rs1.unwrap() as u64);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRCI) => {
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.clear(inst.rs1.unwrap() as u64);
                }
                _ => unreachable!(),
            },
            unsupported_csr_num => {
                unimplemented!("unsupported CSRs: {unsupported_csr_num:#x}")
            }
        }
    }

    /// Emulate CSR field that already exists.
    fn csr_field(&mut self, inst: &Instruction, write_to_csr_value: u64, read_csr_value: &mut u64) {
        const CSR_SENVCFG: usize = 0x10a;

        let csr_num = inst.rs2.unwrap();
        match csr_num {
            CSR_SENVCFG => {
                // overwritten emulated csr field
                *read_csr_value |= (self.senv_sse as u64) << 3;

                // update emulated csr field
                match inst.opc {
                    OpcodeKind::Zicsr(ZicsrOpcode::CSRRW) => {
                        if write_to_csr_value >> 3 & 0x1 == 1 {
                            self.senv_sse = true;
                        }
                    }
                    OpcodeKind::Zicsr(ZicsrOpcode::CSRRS) => {
                        if write_to_csr_value >> 3 & 0x1 == 1 {
                            self.senv_sse = true;
                        }
                    }
                    OpcodeKind::Zicsr(ZicsrOpcode::CSRRC) => {
                        if write_to_csr_value >> 3 & 0x1 == 1 {
                            self.senv_sse = false;
                        }
                    }
                    OpcodeKind::Zicsr(ZicsrOpcode::CSRRWI) => {
                        if write_to_csr_value >> 3 & 0x1 == 1 {
                            self.senv_sse = true;
                        }
                    }
                    OpcodeKind::Zicsr(ZicsrOpcode::CSRRSI) => {
                        if write_to_csr_value >> 3 & 0x1 == 1 {
                            self.senv_sse = true;
                        }
                    }
                    OpcodeKind::Zicsr(ZicsrOpcode::CSRRCI) => {
                        if write_to_csr_value >> 3 & 0x1 == 1 {
                            self.senv_sse = false;
                        }
                    }
                    _ => unreachable!(),
                }
            }
            _ => (),
        }
    }
}
