use crate::ring::{RawRing, RawRingPtr};
use crate::sqe::{PrepareSqe, SQE};
use crate::{sys, utils};

use std::mem::{self, MaybeUninit};
use std::{fmt, io};

pub struct SubmissionQueue<'r> {
    ring: RawRingPtr<'r>,
}

unsafe impl Send for SubmissionQueue<'_> {}
unsafe impl Sync for SubmissionQueue<'_> {}

impl SubmissionQueue<'_> {
    pub(crate) unsafe fn new_unchecked(ptr: *mut RawRing) -> Self {
        Self {
            ring: RawRingPtr::new_unchecked(ptr),
        }
    }

    pub fn prepared(&self) -> u32 {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            sys::io_uring_sq_ready(ring_ptr)
        }
    }

    pub fn space_left(&self) -> u32 {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            sys::io_uring_sq_space_left(ring_ptr)
        }
    }

    /// # Safety
    pub unsafe fn get_sqe_uninit(&mut self) -> Option<&mut MaybeUninit<SQE>> {
        let ring_ptr = self.ring.get_mut_ptr();
        let ret = sys::io_uring_get_sqe(ring_ptr);
        mem::transmute(ret)
    }

    pub fn get_sqe(&mut self) -> Option<&mut SQE> {
        unsafe { self.get_sqe_uninit().map(|sqe| sqe.prep_nop()) }
    }

    pub fn submit(&mut self) -> io::Result<u32> {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            let ret = sys::io_uring_submit(ring_ptr);
            utils::resultify(ret)
        }
    }

    pub fn submit_and_wait(&mut self, wait_for: u32) -> io::Result<u32> {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            let ret = sys::io_uring_submit_and_wait(ring_ptr, wait_for);
            utils::resultify(ret)
        }
    }
}

impl fmt::Debug for SubmissionQueue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fd = self.ring.get_ref().ring_fd();
        f.debug_struct(std::any::type_name::<Self>())
            .field("ring_fd", &fd)
            .finish()
    }
}
