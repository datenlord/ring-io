use crate::cq::CompletionQueue;
use crate::register::Registrar;
use crate::sq::SubmissionQueue;
use crate::{sys, utils};

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;
use std::ptr::NonNull;
use std::{fmt, io, mem};

#[repr(transparent)]
pub(crate) struct RawRing(UnsafeCell<sys::io_uring>);

impl RawRing {
    pub unsafe fn new(entries: u32, params: &mut sys::io_uring_params) -> io::Result<Self> {
        let mut ring: MaybeUninit<UnsafeCell<sys::io_uring>> = MaybeUninit::uninit();

        let ring_ptr: *mut sys::io_uring = ring.as_mut_ptr().cast();

        let ret = sys::io_uring_queue_init_params(entries, ring_ptr, params);
        let _ = utils::resultify(ret)?;

        Ok(Self(ring.assume_init()))
    }

    pub fn ring_fd(&self) -> RawFd {
        unsafe { (*self.0.get()).ring_fd }
    }

    pub fn get_mut_ptr(&self) -> *mut sys::io_uring {
        self.0.get()
    }
}

pub(crate) struct RawRingPtr<'r>(NonNull<RawRing>, PhantomData<&'r mut RawRing>);

impl RawRingPtr<'_> {
    pub unsafe fn new_unchecked(ptr: *mut RawRing) -> Self {
        Self(NonNull::new_unchecked(ptr), PhantomData)
    }

    pub fn get_ref(&self) -> &RawRing {
        unsafe { self.0.as_ref() }
    }

    pub fn get_mut_ptr(&self) -> *mut sys::io_uring {
        self.0.as_ptr().cast()
    }
}

pub struct Ring {
    ring: RawRing,
}

unsafe impl Send for Ring {}
unsafe impl Sync for Ring {}

pub struct RingBuilder {
    entries: u32,
    params: sys::io_uring_params,
}

unsafe impl Send for RingBuilder {}
unsafe impl Sync for RingBuilder {}

impl RingBuilder {
    pub fn new(entries: u32) -> Self {
        Self {
            entries,
            params: unsafe { mem::zeroed() },
        }
    }

    pub fn build(mut self) -> io::Result<Ring> {
        unsafe {
            let ring = RawRing::new(self.entries, &mut self.params)?;
            Ok(Ring { ring })
        }
    }
}

impl fmt::Debug for RingBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RingBuilder")
            .field("entries", &self.entries) // TODO: more info
            .finish()
    }
}

impl Ring {
    // --- methods ---

    pub fn sq(&mut self) -> SubmissionQueue<'_> {
        unsafe { SubmissionQueue::new_unchecked(&mut self.ring) }
    }

    pub fn cq(&mut self) -> CompletionQueue<'_> {
        unsafe { CompletionQueue::new_unchecked(&mut self.ring) }
    }

    pub fn registrar(&mut self) -> Registrar<'_> {
        unsafe { Registrar::new_unchecked(&mut self.ring) }
    }

    pub fn split(&mut self) -> (SubmissionQueue<'_>, CompletionQueue<'_>, Registrar<'_>) {
        let ring: *mut RawRing = &mut self.ring;
        unsafe {
            let sq = SubmissionQueue::new_unchecked(ring);
            let cq = CompletionQueue::new_unchecked(ring);
            let reg = Registrar::new_unchecked(ring);
            (sq, cq, reg)
        }
    }
}

impl Drop for Ring {
    fn drop(&mut self) {
        // FIXME: Can we exit before all IO operations have been completed or cancelled?
        let ring_ptr = self.ring.get_mut_ptr();
        unsafe { sys::io_uring_queue_exit(ring_ptr) }
    }
}

impl fmt::Debug for Ring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fd = self.ring.ring_fd();
        f.debug_struct("Ring").field("ring_fd", &fd).finish() // TODO: more info
    }
}
