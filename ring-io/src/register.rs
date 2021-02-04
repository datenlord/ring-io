use crate::ring::{RawRing, RawRingPtr};
use crate::{sys, utils};

use std::os::unix::io::RawFd;
use std::{fmt, io};

pub struct Registrar<'r> {
    ring: RawRingPtr<'r>,
}

unsafe impl Send for Registrar<'_> {}
unsafe impl Sync for Registrar<'_> {}

impl Registrar<'_> {
    pub(crate) unsafe fn new_unchecked(ptr: *mut RawRing) -> Self {
        Self {
            ring: RawRingPtr::new_unchecked(ptr),
        }
    }

    /// # Safety
    pub unsafe fn register_buffers(
        &self,
        iovecs: *const libc::iovec,
        n_vecs: usize,
    ) -> io::Result<()> {
        let ring_ptr = self.ring.get_mut_ptr();
        let ret = sys::io_uring_register_buffers(ring_ptr, iovecs, n_vecs as u32);
        utils::resultify(ret)?;
        Ok(())
    }

    pub fn unregister_buffers(&self) -> io::Result<()> {
        let ring_ptr = self.ring.get_mut_ptr();
        let ret = unsafe { sys::io_uring_unregister_buffers(ring_ptr) };
        utils::resultify(ret)?;
        Ok(())
    }

    pub fn register_files(&self, files: &[RawFd]) -> io::Result<()> {
        let ring_ptr = self.ring.get_mut_ptr();
        let files_ptr = files.as_ptr();
        let nr_files = files.len() as u32;
        let ret = unsafe { sys::io_uring_register_files(ring_ptr, files_ptr, nr_files) };
        utils::resultify(ret)?;
        Ok(())
    }

    pub fn unregister_files(&self) -> io::Result<()> {
        let ring_ptr = self.ring.get_mut_ptr();
        let ret = unsafe { sys::io_uring_unregister_files(ring_ptr) };
        utils::resultify(ret)?;
        Ok(())
    }
}

impl fmt::Debug for Registrar<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fd = self.ring.get_ref().ring_fd();
        f.debug_struct(std::any::type_name::<Self>())
            .field("ring_fd", &fd)
            .finish()
    }
}
