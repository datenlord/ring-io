use crate::ring::RawRing;
use crate::sys;

use std::marker::PhantomData;
use std::os::unix::io::RawFd;
use std::{fmt, io, ptr};

pub struct Registrar<'r> {
    ring_fd: RawFd,
    _marker: PhantomData<&'r mut RawRing>,
}

unsafe impl Send for Registrar<'_> {}
unsafe impl Sync for Registrar<'_> {}

impl Registrar<'_> {
    pub(crate) unsafe fn new_unchecked(ring_fd: RawFd) -> Self {
        Self {
            ring_fd,
            _marker: PhantomData,
        }
    }

    fn syscall_register(&self, f: impl FnOnce(RawFd) -> i32) -> io::Result<()> {
        let ret = f(self.ring_fd);
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// # Safety
    pub unsafe fn register_buffers(
        &self,
        iovecs: *const libc::iovec,
        n_vecs: usize,
    ) -> io::Result<()> {
        self.syscall_register(|fd| {
            sys::pure::io_uring_register(
                fd,
                sys::IORING_REGISTER_BUFFERS,
                iovecs.cast(),
                n_vecs as u32,
            )
        })
    }

    pub fn unregister_buffers(&self) -> io::Result<()> {
        self.syscall_register(|fd| unsafe {
            sys::pure::io_uring_register(fd, sys::IORING_UNREGISTER_BUFFERS, ptr::null(), 0)
        })
    }

    pub fn register_files(&self, files: &[RawFd]) -> io::Result<()> {
        self.syscall_register(|fd| unsafe {
            let files_ptr = files.as_ptr();
            let nr_files = files.len() as u32;
            sys::pure::io_uring_register(fd, sys::IORING_REGISTER_FILES, files_ptr.cast(), nr_files)
        })
    }

    pub fn unregister_files(&self) -> io::Result<()> {
        self.syscall_register(|fd| unsafe {
            sys::pure::io_uring_register(fd, sys::IORING_UNREGISTER_FILES, ptr::null(), 0)
        })
    }
}

impl fmt::Debug for Registrar<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fd = self.ring_fd;
        f.debug_struct(std::any::type_name::<Self>())
            .field("ring_fd", &fd)
            .finish()
    }
}
