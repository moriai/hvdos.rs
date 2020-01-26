// Copyright (c) 2009-present, the hvdos developers. All Rights Reserved.
// Copyright (c) 2020, Satoshi Moriai.
// Read LICENSE.txt for licensing information.
//
// hvdos.rs - a simple DOS emulator based on the OS X 10.10 Hypervisor.framework

use xhypervisor::*;
use xhypervisor::consts::vmcs::*;
use xhypervisor::consts::vmx_exit;
use xhypervisor::consts::vmx_cap;
use getopts::{Options,ParsingStyle};
use colored::Colorize;
use std::{env,process};
use std::fs::File;
use std::io::prelude::*;
use std::os::raw::{c_char,c_int};
use std::ffi::CString;
pub mod doskernel;
use doskernel::*;
use rustc_tools_util::*;

// desired control word constrained by hardware/hypervisor capabilities
fn cap2ctrl(cap: u64, ctrl: u64) -> u64 {
    (ctrl | (cap & 0xffffffff)) & (cap >> 32)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.parsing_style(ParsingStyle::StopAtFirstFree)
        .optflag("d", "debug", "debug mode")
        .optflag("t", "trace", "tracing mode")
        .optflag("V", "version", "print version info")
        .optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => {
            eprintln!("{}: {}", "error".red().bold(), f);
            process::exit(1);
        }
    };

    if matches.opt_present("V") {
        println!("{}", rustc_tools_util::get_version_info!());
        return;
    }

    if matches.opt_present("h") {
        let brief = format!("Usage: {} [options] COM ...", program);
        print!("{}", opts.usage(&brief));
        return;
    }

    if matches.free.is_empty() {
        eprintln!("{}: No COM file", "error".red().bold());
        process::exit(1);
    }
    let comfile = matches.free[0].clone();
    let trace = matches.opt_present("t");
    let debug = matches.opt_present("d");

    let mut iter = matches.free.into_iter(); iter.next();
    let c_args = iter.map(|arg| CString::new(arg).unwrap() ).collect::<Vec<CString>>();
    let c_argv = c_args.iter().map(|arg| arg.as_ptr()).collect::<Vec<*const c_char>>();
    let c_argc = c_argv.len();

    // create a VM instance for the current task 
    create_vm().unwrap();

    // get hypervisor enforced capabilities of the machine, (see Intel docs)
    let vmx_cap_pinbased = read_vmx_cap(&VMXCap::PINBASED).unwrap();
    let vmx_cap_procbased = read_vmx_cap(&VMXCap::PROCBASED).unwrap();
    let vmx_cap_procbased2 = read_vmx_cap(&VMXCap::PROCBASED2).unwrap();
    let vmx_cap_entry = read_vmx_cap(&VMXCap::ENTRY).unwrap();

    // allocate some guest physical memory
    const VM_MEM_SIZE: usize = 1 * 1024 * 1024;
    let mut vm_mem: Vec<u8> = vec![0u8;VM_MEM_SIZE];
    if debug {
        eprintln!("{}", format!("VM start: {:?}, size: {:#x}", vm_mem.as_ptr(), VM_MEM_SIZE).green());
    }

    // map a segment of guest physical memory into the guest physical address
    // space of the vm (at address 0)
    map_mem(&vm_mem, 0, &MemPerm::ExecAndWrite).unwrap();

    // create a vCPU instance for this thread
    let vcpu = vCPU::new().unwrap();

    // vCPU setup
    // set VMCS control fields
    //   see Intel SDM Vol.3, Tables 24-5, 24-6 & 24-7
    vcpu.write_vmcs(VMCS_CTRL_PIN_BASED, cap2ctrl(vmx_cap_pinbased, 0)).unwrap();
    vcpu.write_vmcs(VMCS_CTRL_CPU_BASED, cap2ctrl(vmx_cap_procbased,
        if trace { vmx_cap::CPU_BASED_MTF } else { 0 } |
        vmx_cap::CPU_BASED_HLT |
        vmx_cap::CPU_BASED_CR8_LOAD |
        vmx_cap::CPU_BASED_CR8_STORE)).unwrap();
    vcpu.write_vmcs(VMCS_CTRL_CPU_BASED2, cap2ctrl(vmx_cap_procbased2, 0)).unwrap();
    //   see Table 24-13
    vcpu.write_vmcs(VMCS_CTRL_VMENTRY_CONTROLS, cap2ctrl(vmx_cap_entry, 0)).unwrap();
    //   see Sec. 24.6.3 & 25.2
    vcpu.write_vmcs(VMCS_CTRL_EXC_BITMAP, 0xffffffff).unwrap();
    vcpu.write_vmcs(VMCS_CTRL_CR0_MASK, 0x60000000).unwrap();
    vcpu.write_vmcs(VMCS_CTRL_CR0_SHADOW, 0).unwrap();
    vcpu.write_vmcs(VMCS_CTRL_CR4_MASK, 0).unwrap();
    vcpu.write_vmcs(VMCS_CTRL_CR4_SHADOW, 0).unwrap();
    // set VMCS guest state fields
    vcpu.write_vmcs(VMCS_GUEST_CS, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_CS_LIMIT, 0xffff).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_CS_AR, 0x9b).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_CS_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_DS, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_DS_LIMIT, 0xffff).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_DS_AR, 0x93).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_DS_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_ES, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_ES_LIMIT, 0xffff).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_ES_AR, 0x93).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_ES_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_FS, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_FS_LIMIT, 0xffff).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_FS_AR, 0x93).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_FS_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_GS, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_GS_LIMIT, 0xffff).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_GS_AR, 0x93).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_GS_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_SS, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_SS_LIMIT, 0xffff).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_SS_AR, 0x93).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_SS_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_LDTR, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_LDTR_LIMIT, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_LDTR_AR, 0x10000).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_LDTR_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_TR, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_TR_LIMIT, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_TR_AR, 0x83).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_TR_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_GDTR_LIMIT, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_GDTR_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_IDTR_LIMIT, 0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_IDTR_BASE, 0).unwrap();

    vcpu.write_vmcs(VMCS_GUEST_CR0, 0x20).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_CR3, 0x0).unwrap();
    vcpu.write_vmcs(VMCS_GUEST_CR4, 0x2000).unwrap();

    let status = {
        // initialize DOS emulation
        let mut kernel = DOSKernel::new(vm_mem.as_ptr(), vcpu.id, c_argc as c_int, c_argv.as_ptr());

        // read COM file at 0x100
        {
            let mut f = File::open(comfile).unwrap();
            f.read(&mut vm_mem[0x100..64*1024]).unwrap();
        }

        // set up GPRs, start at COM file entry point
        vcpu.write_register(&x86Reg::RIP, 0x100).unwrap();
        vcpu.write_register(&x86Reg::RFLAGS, 0x2).unwrap();
        vcpu.write_register(&x86Reg::RSP, 0xfff8).unwrap(); // should be 0xfffe in legacy 16bit mode

        // vCPU run loop
        let mut stop = false;
        while !stop {
            vcpu.run().unwrap();

            // handle VMEXIT
            let exit_reason = vcpu.read_vmcs(VMCS_RO_EXIT_REASON).unwrap();

            match exit_reason {
                vmx_exit::VMX_REASON_EXC_NMI => {
                    let interrupt_number: u8 = vcpu.read_vmcs(VMCS_RO_IDT_VECTOR_INFO).unwrap() as u8;
                    let status = kernel.dispatch(interrupt_number);
                    match status {
                        DOSKernel::STATUS_HANDLED => {
                            vcpu.write_register(&x86Reg::RIP, vcpu.read_register(&x86Reg::RIP).unwrap() + 2).unwrap();
                        }
                        DOSKernel::STATUS_UNSUPPORTED | DOSKernel::STATUS_STOP => {
                            stop = true;
                        }
                        DOSKernel::STATUS_NORETURN => {
                            // The kernel changed the PC.
                        }
                        _ => {
                            stop = true;
                        }
                    }
                }
                vmx_exit::VMX_REASON_MTF => {
                    if trace {
                        eprintln!("{}", format!("Step CS:IP={:#x}:{:#x} SS:SP={:#x}:{:#x}",
                            vcpu.read_register(&x86Reg::CS).unwrap(),
                            vcpu.read_register(&x86Reg::RIP).unwrap(),
                            vcpu.read_register(&x86Reg::SS).unwrap(),
                            vcpu.read_register(&x86Reg::RSP).unwrap()
                        ).green());
                    }
                }
                vmx_exit::VMX_REASON_IRQ => {
                    // VMEXIT due to host interrupt, nothing to do
                    if debug || trace {
                        eprintln!("{}", format!("IRQ CS:IP={:#x}:{:#x} SS:SP={:#x}:{:#x}",
                            vcpu.read_register(&x86Reg::CS).unwrap(),
                            vcpu.read_register(&x86Reg::RIP).unwrap(),
                            vcpu.read_register(&x86Reg::SS).unwrap(),
                            vcpu.read_register(&x86Reg::RSP).unwrap()
                        ).green());
                    }
                }
                vmx_exit::VMX_REASON_HLT => {
                    // guest executed HLT
                    if debug || trace {
                        eprintln!("{}", "HLT".green());
                    }
                    stop = true;
                }
                vmx_exit::VMX_REASON_EPT_VIOLATION => {
                    // disambiguate between EPT cold misses and MMIO
                    // ... handle MMIO ...
                }
                // ... many more exit reasons go here ...
                _ => {
                    eprintln!("{}", format!("Unhandled VMEXIT ({})", exit_reason).yellow());
                    stop = true;
                }
            }
        }

        kernel.exit_status()
    };

    // optional clean-up ...
    vcpu.destroy().unwrap();
    unmap_mem(0, VM_MEM_SIZE).unwrap();
    destroy_vm().unwrap();

    if debug {
        eprintln!("{}", format!("Exit code: {}", status).green());
    }
    process::exit(status);
}
