// Copyright (c) 2020, Satoshi Moriai.
// Read LICENSE.txt for licensing information.

use std::os::raw::{c_char,c_uchar,c_int,c_uint};

pub enum DOSKernelInst {}

extern {
    pub fn DOSKernel_DOSKernel(memory: *const c_uchar, vcpu: c_uint, argc: c_int, argv: *const *const c_char) -> *mut DOSKernelInst;
    pub fn DOSKernel_DOSKernel_destructor(ki: *mut DOSKernelInst);
    pub fn DOSKernel_dispatch(ki: *mut DOSKernelInst, intno: u8) -> i32;
    pub fn DOSKernel_exitStatus(ki: *mut DOSKernelInst) -> i32;
}

pub struct DOSKernel {
    inst: *mut DOSKernelInst
}

impl DOSKernel {
    pub const STATUS_HANDLED: i32 = 0;
    pub const STATUS_STOP: i32 = 1;
    pub const STATUS_UNHANDLED: i32 = 2;
    pub const STATUS_UNSUPPORTED: i32 = 3;
    pub const STATUS_NORETURN: i32 = 4;

    #[inline]
    pub fn new(memory: *const c_uchar, vcpu: c_uint, argc: c_int, argv: *const *const c_char) -> Self {
        unsafe {
            DOSKernel { inst: DOSKernel_DOSKernel(memory, vcpu, argc, argv) }
        }
    }

    #[inline]
    pub fn dispatch(&mut self, intno: u8) -> i32 {
        unsafe {
            DOSKernel_dispatch(self.inst, intno)
        }
    }

    #[inline]
    pub fn exit_status(&mut self) -> i32 {
        unsafe {
            DOSKernel_exitStatus(self.inst)
        }
    }
}

impl Drop for DOSKernel {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            DOSKernel_DOSKernel_destructor(self.inst)
        }
    }
}
