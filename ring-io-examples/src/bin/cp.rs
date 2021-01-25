use self::chunk::{DataChunkPtr, DataChunkState};
use ring_io::sqe::{FsyncFlags, PrepareSqe, SubmissionFlags};
use ring_io::{Ring, RingBuilder, CQE};

use std::collections::VecDeque;
use std::convert::TryInto;
use std::fs::{self, File};
use std::mem;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    src: PathBuf,
    dst: PathBuf,
}

fn main() -> Result<()> {
    run(Args::from_args())
}

const RING_ENTRIES: u32 = 32;
const CHUNK_SIZE: usize = 32 * 1024;
const MAX_CHUNKS: usize = 32;

fn run(args: Args) -> Result<()> {
    let mut src_file = {
        let src = &args.src;
        let on_err = || format!("can not open source file: path = {}", src.display());
        File::open(src).with_context(on_err)?
    };

    let src_file_size: u64 = {
        let metadata = src_file.metadata()?;
        if !metadata.is_file() {
            bail!("source file is not a regular file");
        }
        metadata.len()
    };

    let mut dst_file = {
        let dst = &args.dst;
        let on_err = || format!("can not open destination file: path = {}", dst.display());
        File::create(&dst).with_context(on_err)?
    };

    let mut ring = {
        let on_err = || format!("failed to build a ring: entries = {}", RING_ENTRIES);
        RingBuilder::new(RING_ENTRIES)
            .build()
            .with_context(on_err)?
    };

    let ret = copy_file(
        &mut ring,
        &mut src_file,
        &mut dst_file,
        src_file_size,
        CHUNK_SIZE,
    );

    drop(src_file);
    drop(dst_file);
    drop(ring);

    if let Err(err) = ret {
        let _ = fs::remove_file(&args.dst);
        return Err(err.context("failed to run copy_file"));
    }

    Ok(())
}

mod chunk {
    use std::ffi::OsStr;
    use std::ops::Range;
    use std::os::unix::ffi::OsStrExt;
    use std::ptr::NonNull;
    use std::{fmt, mem, slice};

    use aligned_utils::bytes::AlignedBytes;

    #[repr(C)]
    struct DataChunkHeader {
        chunk_size: usize,
        file_offset: usize,
        iov: libc::iovec,
        state: u8,
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(u8)]
    pub enum DataChunkState {
        IDLE = 0,
        READING = 1,
        WRITING = 2,
    }

    pub struct DataChunkPtr(NonNull<DataChunkHeader>);

    impl DataChunkPtr {
        pub fn alloc(file_offset: usize, chunk_size: usize) -> DataChunkPtr {
            let size = chunk_size
                .checked_add(mem::size_of::<DataChunkHeader>())
                .unwrap();
            let align = mem::align_of::<DataChunkHeader>();

            let mem = AlignedBytes::new_zeroed(size, align);
            let mem_ptr: *mut u8 = AlignedBytes::into_raw(mem).0.as_ptr().cast();

            unsafe {
                let header_ptr: *mut DataChunkHeader = mem_ptr.cast();
                let buf_ptr = header_ptr
                    .cast::<u8>()
                    .add(mem::size_of::<DataChunkHeader>());

                let header = &mut *header_ptr;
                header.file_offset = file_offset;
                header.chunk_size = chunk_size;
                header.iov.iov_base = buf_ptr.cast();
                header.iov.iov_len = 0;
                header.state = DataChunkState::IDLE as u8;

                DataChunkPtr(NonNull::new_unchecked(header_ptr))
            }
        }

        pub fn state(&self) -> &DataChunkState {
            unsafe { &*<*const u8>::cast::<DataChunkState>(&self.0.as_ref().state) }
        }

        pub fn state_mut(&mut self) -> &mut DataChunkState {
            unsafe { &mut *<*mut u8>::cast::<DataChunkState>(&mut self.0.as_mut().state) }
        }

        fn buf_ptr(&self) -> *mut u8 {
            unsafe {
                self.0
                    .as_ptr()
                    .cast::<u8>()
                    .add(mem::size_of::<DataChunkHeader>())
            }
        }

        pub fn iovecs_ptr(&self) -> *const libc::iovec {
            unsafe {
                let header = self.0.as_ref();
                &header.iov
            }
        }

        pub fn iovecs_mut_ptr(&mut self) -> *mut libc::iovec {
            unsafe {
                let header = self.0.as_mut();
                &mut header.iov
            }
        }

        pub fn set_full_filled(&mut self) {
            unsafe {
                let chunk_size = self.0.as_ref().chunk_size;
                self.set_data_range_unchecked(0..chunk_size);
            }
        }

        pub unsafe fn set_data_range_unchecked(&mut self, range: Range<usize>) {
            let buf_ptr = self.buf_ptr();
            let header = self.0.as_mut();
            header.iov.iov_base = buf_ptr.add(range.start).cast();
            header.iov.iov_len = range.end - range.start;
        }

        pub fn data(&self) -> &[u8] {
            unsafe {
                let header = self.0.as_ref();
                slice::from_raw_parts(header.iov.iov_base.cast(), header.iov.iov_len)
            }
        }

        pub fn file_offset(&self) -> usize {
            unsafe { self.0.as_ref().file_offset }
        }

        pub fn set_file_offset(&mut self, offset: usize) {
            unsafe { self.0.as_mut().file_offset = offset };
        }

        pub unsafe fn consume_data_unchecked(&mut self, n_bytes: usize) {
            let iov = &mut self.0.as_mut().iov;
            iov.iov_base = iov.iov_base.cast::<u8>().add(n_bytes).cast();
            iov.iov_len -= n_bytes;
        }

        pub unsafe fn from_raw(ptr: *mut ()) -> Self {
            Self(NonNull::new_unchecked(ptr.cast()))
        }

        pub fn into_raw(self) -> *mut () {
            let ptr = self.0.as_ptr();
            mem::forget(self);
            ptr.cast()
        }
    }

    impl Drop for DataChunkPtr {
        fn drop(&mut self) {
            let ptr = self.0.as_ptr();
            unsafe {
                let size = (*ptr).chunk_size + mem::size_of::<DataChunkHeader>();
                let align = mem::align_of::<DataChunkHeader>();
                let buf = NonNull::from(slice::from_raw_parts_mut(ptr.cast::<u8>(), size));
                let mem = AlignedBytes::from_raw(buf, align);
                drop(mem)
            }
        }
    }

    impl fmt::Debug for DataChunkPtr {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let buf_ptr = self.buf_ptr();
            let header = unsafe { self.0.as_ref() };
            let state = self.state();
            let data_start = unsafe { header.iov.iov_base.cast::<u8>().offset_from(buf_ptr) };
            let data: &OsStr = OsStrExt::from_bytes(self.data());
            f.debug_struct("DataChunk")
                .field("file_offset", &header.file_offset)
                .field("chunk_size", &header.chunk_size)
                .field("state", state)
                .field("data_start", &data_start)
                .field("data_len", &header.iov.iov_len)
                .field("data", &data)
                .finish()
        }
    }
}

fn copy_file(
    ring: &mut Ring,
    src_file: &mut File,
    dst_file: &mut File,
    src_file_size: u64,
    chunk_size: usize,
) -> Result<()> {
    let cp_size: isize = src_file_size.try_into()?; // check overflow
    let cp_size = cp_size as usize;

    let chunk_size = chunk_size.min(cp_size);

    let src_fd = src_file.as_raw_fd();
    let dst_fd = dst_file.as_raw_fd();

    let (mut sq, mut cq, _) = ring.split();

    let mut total_n_written: usize = 0;
    let mut current_offset: usize = 0;

    let mut chunk_count: usize = 0;
    let mut recycle_chunks: Vec<DataChunkPtr> = Vec::new();
    let mut data_chunks: VecDeque<DataChunkPtr> = VecDeque::new();

    let mut n_submitted: u32 = 0;
    let mut n_completed: u32 = 0;

    loop {
        // issue `read` SQEs into the kernel pipeline as many as possible
        while current_offset < cp_size {
            if recycle_chunks.is_empty() && chunk_count >= MAX_CHUNKS {
                // there is no more chunks, wait for a CQE
                break;
            }

            let sqe = match unsafe { sq.get_sqe_uninit() } {
                None => break,
                Some(sqe) => sqe,
            };

            let next_chunk_size = (cp_size - current_offset).min(chunk_size);

            let mut chunk = match recycle_chunks.pop() {
                Some(mut c) => {
                    c.set_file_offset(current_offset);
                    c
                }
                None => {
                    let c = DataChunkPtr::alloc(current_offset, next_chunk_size);
                    chunk_count += 1;
                    c
                }
            };

            chunk.set_full_filled();
            *chunk.state_mut() = DataChunkState::READING;

            // dbg!(("before readv", &chunk));
            unsafe {
                let iovecs = chunk.iovecs_mut_ptr();
                let chunk_ptr = chunk.into_raw();

                sqe.prep_readv(src_fd, iovecs, 1, current_offset as isize)
                    .set_user_data(chunk_ptr as u64)
            };

            current_offset += next_chunk_size;
        }

        // issue `write` SQEs into the kernel pipeline as many as possible
        while !data_chunks.is_empty() {
            let sqe = match unsafe { sq.get_sqe_uninit() } {
                None => break,
                Some(sqe) => sqe,
            };

            let mut chunk = data_chunks.pop_front().unwrap();
            *chunk.state_mut() = DataChunkState::WRITING;

            // dbg!(("before writev", &chunk));

            unsafe {
                let iovecs = chunk.iovecs_ptr();
                let offset = chunk.file_offset();
                let chunk_ptr = chunk.into_raw();

                sqe.prep_writev(dst_fd, iovecs, 1, offset as isize)
                    .set_user_data(chunk_ptr as u64)
            }
        }

        // submit all prepared SQEs
        n_submitted += sq.submit()?;

        // wait at least one CQE
        cq.wait_cqes(1)?;

        // reap available CQEs
        let mut cqes_buf: [Option<&CQE>; RING_ENTRIES as usize] = unsafe { mem::zeroed() };
        let cqes = cq.peek_batch_cqe(&mut cqes_buf);
        for &cqe in cqes {
            let mut chunk = unsafe { DataChunkPtr::from_raw(cqe.user_data() as _) };

            // dbg!(("after IO completed", &chunk));

            match chunk.state_mut() {
                DataChunkState::READING => {
                    let n_read = cqe.io_result().context("IO operation failed: op = readv")?;
                    unsafe { chunk.set_data_range_unchecked(0..n_read as usize) };
                    *chunk.state_mut() = DataChunkState::IDLE;

                    // dbg!(("after readv completed", &chunk));
                    data_chunks.push_back(chunk);
                }
                DataChunkState::WRITING => {
                    let n_written = cqe
                        .io_result()
                        .context("IO operation failed: op = writev")?;
                    unsafe { chunk.consume_data_unchecked(n_written as usize) };
                    *chunk.state_mut() = DataChunkState::IDLE;

                    // dbg!(("after writev completed", n_written, &chunk));
                    total_n_written += n_written as usize;

                    if chunk.data().is_empty() {
                        recycle_chunks.push(chunk);
                    } else {
                        data_chunks.push_back(chunk)
                    }
                }
                DataChunkState::IDLE => unreachable!(),
            }
        }
        let n_reaped = cqes.len() as u32;
        unsafe { cq.advance_unchecked(n_reaped) };
        n_completed += n_reaped;

        if total_n_written >= cp_size {
            break;
        }
    }

    {
        // synchronize file finally
        let sqe = sq
            .get_sqe()
            .expect("no available SQE in the submission queue");
        unsafe {
            sqe.prep_fsync(dst_fd, FsyncFlags::empty())
                .enable_flags(SubmissionFlags::IO_DRAIN);
        }

        sq.submit()?;
        cq.wait_cqes(1)?;

        let cqe = cq
            .peek_cqe()
            .expect("no available CQE in the completion queue");

        cqe.io_result()?;
        unsafe { cq.advance_unchecked(1) };
    }

    debug_assert_eq!(sq.prepared(), 0);
    debug_assert_eq!(sq.space_left(), RING_ENTRIES);
    debug_assert_eq!(cq.ready(), 0);
    debug_assert_eq!(n_submitted, n_completed);
    debug_assert!(data_chunks.is_empty());

    Ok(())
}
