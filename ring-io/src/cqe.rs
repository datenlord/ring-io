use crate::{sys, utils};

use std::{fmt, io, ptr};

use bitflags::bitflags;

#[repr(transparent)]
pub struct CQE {
    cqe: sys::io_uring_cqe,
}

unsafe impl Send for CQE {}
unsafe impl Sync for CQE {}

bitflags! {
    pub struct CompletionFlags: u32 {
        const BUFFER_SHIFT    = sys::IORING_CQE_BUFFER_SHIFT;
    }
}

impl CQE {
    // --- constructor ---

    pub fn new(user_data: u64, res: i32, flags: CompletionFlags) -> Self {
        let flags = flags.bits();
        Self {
            cqe: sys::io_uring_cqe {
                user_data,
                res,
                flags,
            },
        }
    }

    // --- getters ---

    pub fn user_data(&self) -> u64 {
        self.cqe.user_data
    }

    pub fn flags(&self) -> CompletionFlags {
        unsafe { CompletionFlags::from_bits_unchecked(self.cqe.flags) }
    }

    pub fn raw_result(&self) -> i32 {
        self.cqe.res
    }

    // --- methods ---

    pub fn io_result(&self) -> io::Result<u32> {
        utils::resultify(self.cqe.res)
    }

    pub fn is_err(&self) -> bool {
        self.cqe.res < 0
    }
}

impl Clone for CQE {
    fn clone(&self) -> Self {
        Self {
            cqe: unsafe { ptr::read(&self.cqe) },
        }
    }
}

impl fmt::Debug for CQE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CQE")
            .field("user_data", &self.user_data())
            .field("res", &self.raw_result())
            .field("Flags", &self.flags())
            .finish()
    }
}
