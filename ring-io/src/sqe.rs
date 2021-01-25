use crate::sys;

use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;
use std::{fmt, ptr};

use bitflags::bitflags;

// TODO: Safety documentation

#[repr(transparent)]
pub struct SQE {
    sqe: sys::io_uring_sqe,
}

unsafe impl Send for SQE {}
unsafe impl Sync for SQE {}

impl Clone for SQE {
    fn clone(&self) -> Self {
        Self {
            sqe: unsafe { ptr::read(&self.sqe) },
        }
    }
}

impl fmt::Debug for SQE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SQE {{ .. }}")
    }
}

impl SQE {
    pub fn new_uninit() -> MaybeUninit<Self> {
        MaybeUninit::uninit()
    }

    pub fn overwrite_uninit(this: &mut MaybeUninit<SQE>, sqe: SQE) -> &mut Self {
        unsafe {
            ptr::write(this.as_mut_ptr(), sqe);
            &mut *this.as_mut_ptr()
        }
    }

    pub fn set_flags(&mut self, flags: SubmissionFlags) {
        self.sqe.flags = flags.bits()
    }

    pub fn enable_flags(&mut self, flags: SubmissionFlags) {
        self.sqe.flags |= flags.bits()
    }

    pub fn set_user_data(&mut self, user_data: u64) {
        self.sqe.user_data = user_data;
    }
}

bitflags! {
    pub struct SubmissionFlags: u8 {
        const FIXED_FILE    = sys::IOSQE_FIXED_FILE;
        const IO_DRAIN      = sys::IOSQE_IO_DRAIN;
        const IO_LINK       = sys::IOSQE_IO_LINK;
        const IO_HARDLINK   = sys::IOSQE_IO_HARDLINK;
        const ASYNC         = sys::IOSQE_ASYNC;
        const BUFFER_SELECT = sys::IOSQE_BUFFER_SELECT;
    }
}

bitflags! {
    pub struct FsyncFlags: u32 {
        const FSYNC_DATASYNC    = sys::IORING_FSYNC_DATASYNC;
    }
}

impl PrepareSqe for SQE {
    fn as_raw_mut_sqe(&mut self) -> *mut SQE {
        self
    }
}

impl PrepareSqe for MaybeUninit<SQE> {
    fn as_raw_mut_sqe(&mut self) -> *mut SQE {
        self.as_mut_ptr()
    }
}

unsafe fn do_prep(
    this: &mut (impl PrepareSqe + ?Sized),
    f: impl FnOnce(*mut sys::io_uring_sqe),
) -> &mut SQE {
    let sqe = this.as_raw_mut_sqe();
    f(sqe.cast());
    &mut *sqe
}

pub trait PrepareSqe {
    fn as_raw_mut_sqe(&mut self) -> *mut SQE;

    /// # Safety
    /// See [`SQE`]
    unsafe fn prep_nop(&mut self) -> &mut SQE {
        do_prep(self, |sqe| sys::io_uring_prep_nop(sqe))
    }

    /// # Safety
    /// See [`SQE`]
    unsafe fn prep_read(
        &mut self,
        fd: RawFd,
        buf: *mut u8,
        n_bytes: usize,
        offset: isize,
    ) -> &mut SQE {
        do_prep(self, |sqe| {
            sys::io_uring_prep_read(sqe, fd, buf.cast(), n_bytes as u32, offset as libc::off_t)
        })
    }

    /// # Safety
    /// See [`SQE`]
    unsafe fn prep_readv(
        &mut self,
        fd: RawFd,
        iovecs: *const libc::iovec,
        n_vecs: usize,
        offset: isize,
    ) -> &mut SQE {
        do_prep(self, |sqe| {
            sys::io_uring_prep_readv(sqe, fd, iovecs.cast(), n_vecs as u32, offset as libc::off_t)
        })
    }

    /// # Safety
    /// See [`SQE`]
    unsafe fn prep_write(
        &mut self,
        fd: RawFd,
        buf: *const u8,
        n_bytes: usize,
        offset: isize,
    ) -> &mut SQE {
        do_prep(self, |sqe| {
            sys::io_uring_prep_write(sqe, fd, buf.cast(), n_bytes as u32, offset as libc::off_t)
        })
    }

    /// # Safety
    /// See [`SQE`]
    unsafe fn prep_writev(
        &mut self,
        fd: RawFd,
        iovecs: *const libc::iovec,
        n_vecs: usize,
        offset: isize,
    ) -> &mut SQE {
        do_prep(self, |sqe| {
            sys::io_uring_prep_writev(sqe, fd, iovecs, n_vecs as u32, offset as libc::off_t)
        })
    }

    /// # Safety
    /// See [`SQE`]
    unsafe fn prep_fsync(&mut self, fd: RawFd, flags: FsyncFlags) -> &mut SQE {
        do_prep(self, |sqe| sys::io_uring_prep_fsync(sqe, fd, flags.bits()))
    }

    // TODO: impl more prep_* methods
}
