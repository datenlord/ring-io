use crate::ring::{RawRing, RawRingPtr};

use std::fmt;

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
}

impl fmt::Debug for Registrar<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fd = self.ring.get_ref().ring_fd();
        f.debug_struct(std::any::type_name::<Self>())
            .field("ring_fd", &fd)
            .finish()
    }
}
