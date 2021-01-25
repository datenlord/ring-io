use crate::cqe::CQE;
use crate::ring::{RawRing, RawRingPtr};
use crate::{sys, utils};

use std::{fmt, io, ptr, slice};

pub struct CompletionQueue<'r> {
    ring: RawRingPtr<'r>,
}

unsafe impl Send for CompletionQueue<'_> {}
unsafe impl Sync for CompletionQueue<'_> {}

impl CompletionQueue<'_> {
    pub(crate) unsafe fn new_unchecked(ptr: *mut RawRing) -> Self {
        Self {
            ring: RawRingPtr::new_unchecked(ptr),
        }
    }

    pub fn peek_cqe(&mut self) -> Option<&CQE> {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            let mut cqe_ptr: *mut sys::io_uring_cqe = ptr::null_mut();

            sys::io_uring_peek_cqe(ring_ptr, &mut cqe_ptr);

            if cqe_ptr.is_null() {
                return None;
            }

            Some(&*cqe_ptr.cast())
        }
    }

    pub fn peek_batch_cqe<'c, 's: 'c>(
        &'s mut self,
        cqes: &'c mut [Option<&'s CQE>],
    ) -> &'c [&'s CQE] {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            let cqes_ptr: *mut *mut sys::io_uring_cqe = cqes.as_mut_ptr().cast();
            let count = cqes.len() as u32; // safe cast: count <= cqes.len()
            let amt = sys::io_uring_peek_batch_cqe(ring_ptr, cqes_ptr, count);

            let len = amt as usize; // safe cast: amt <= count <= cqes.len()
            slice::from_raw_parts(cqes_ptr.cast(), len)
        }
    }

    pub fn advance(&mut self, n: u32) {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            sys::io_uring_cq_advance(ring_ptr, n);
        }
    }

    pub fn ready(&self) -> u32 {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            sys::io_uring_cq_ready(ring_ptr)
        }
    }

    pub fn is_eventfd_enabled(&self) -> bool {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            sys::io_uring_cq_eventfd_enabled(ring_ptr)
        }
    }

    pub fn toggle_eventfd(&mut self, enabled: bool) -> io::Result<()> {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            let ret = sys::io_uring_cq_eventfd_toggle(ring_ptr, enabled);
            utils::resultify(ret)?;
        }
        Ok(())
    }

    pub fn wait_cqes(&mut self, count: u32) -> io::Result<()> {
        unsafe {
            let ring_ptr = self.ring.get_mut_ptr();
            let mut cqe_ptr: *mut sys::io_uring_cqe = ptr::null_mut();
            let ret = sys::io_uring_wait_cqe_nr(ring_ptr, &mut cqe_ptr, count);
            utils::resultify(ret)?;
        }
        Ok(())
    }
}

impl fmt::Debug for CompletionQueue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fd = self.ring.get_ref().ring_fd();
        f.debug_struct(std::any::type_name::<Self>())
            .field("ring_fd", &fd)
            .finish()
    }
}
