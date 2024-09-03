//! Trap VS-mode exception.

mod sbi;

use super::hstrap_exit;
use crate::guest;
use crate::h_extension::csrs::vstvec;
use crate::HYPERVISOR_DATA;
use core::arch::asm;
use raki::{Decode, Isa::Rv64, OpcodeKind, ZicntrOpcode};
use riscv::register::{
    scause::{self, Exception},
    stval,
};
use sbi::sbi_base_handler;

/// Delegate exception to supervisor mode from VS-mode.
#[no_mangle]
#[inline(always)]
#[allow(clippy::inline_always, clippy::module_name_repetitions)]
pub extern "C" fn hs_forward_exception() {
    unsafe {
        let mut context = HYPERVISOR_DATA.lock().guest().context;
        asm!(
            "csrw vsepc, {sepc}",
            "csrw vscause, {scause}",
            sepc = in(reg) context.sepc(),
            scause = in(reg) scause::read().bits()
        );

        context.set_sepc(vstvec::read().bits());
    }
}

/// Handler for Ecall from VS-mode exception
#[allow(clippy::cast_possible_truncation)]
fn sbi_vs_mode_handler(context: &mut guest::context::Context) {
    let ext_id: usize = context.xreg(17) as usize;
    let func_id: usize = context.xreg(16) as usize;

    let sbiret = match ext_id {
        sbi_spec::base::EID_BASE => sbi_base_handler(func_id),
        _ => panic!(
            "Unsupported SBI call, eid: {:x}, fid: {:x}",
            ext_id, func_id
        ),
    };

    context.set_xreg(10, sbiret.error as u64);
    context.set_xreg(11, sbiret.value as u64);
}

/// Trap `VirtualInstruction` (cause = 22)
fn virtual_instruction_handler(inst_bytes: u32, context: &mut guest::context::Context) {
    let inst = inst_bytes
        .decode(Rv64)
        .expect("virtual instruction decoding failed");

    match inst.opc {
        OpcodeKind::Zicntr(ZicntrOpcode::RDTIME) => {
            let time_val = unsafe {
                let time;
                asm!("csrr {time_val}, time", time_val = out(reg) time);
                time
            };
            context.set_xreg(
                inst.rd.expect("rd register is not found in rdtime"),
                time_val,
            );
        }
        _ => panic!("unsupported instruction"),
    };
}

/// Trap handler for exception
#[allow(clippy::cast_possible_truncation, clippy::module_name_repetitions)]
pub unsafe fn trap_exception(exception_cause: Exception) -> ! {
    let mut context = unsafe { HYPERVISOR_DATA.lock().guest().context };

    match exception_cause {
        Exception::SupervisorEnvCall => panic!("SupervisorEnvCall should be handled by M-mode"),
        // Enum not found in `riscv` crate.
        Exception::Unknown => {
            match scause::read().code() {
                // Ecall from VS-mode
                10 => {
                    sbi_vs_mode_handler(&mut context);
                    context.set_sepc(context.sepc() + 4);
                }
                // Virtual Instruction
                22 => {
                    virtual_instruction_handler(stval::read() as u32, &mut context);
                    context.set_sepc(context.sepc() + 4);
                }
                _ => unreachable!(),
            }
        }
        _ => hs_forward_exception(),
    }

    hstrap_exit();
}