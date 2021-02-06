# ring-io

> An easy-to-use interface for io_uring

This project is inspired by [iou](https://crates.io/crates/iou) and [io-uring](https://crates.io/crates/io-uring).

**ring-io is in the early development stage. The API is subject to change without notice.**

You are strongly recommended not to deploy the code under the current version. Tests, bug reports, user feedback, and other experiments are all welcome at this stage.

ring-io is currently a wrapper around the [liburing](https://git.kernel.dk/cgit/liburing/) library. But it may replace the underlying bindings ([uring-sys](https://crates.io/crates/uring-sys)) with a pure rust implementation in the future.

## Safety

In consideration of use cases and performance, ring-io prefers to provide unsafe APIs. Be careful!

Preparing IO operations is completely unsafe. Users must ensure that the buffers and file descriptors are regarded as borrowed or taken by the kernel during the lifetime of the IO.

## Pure

The branch [pure](https://github.com/Nugine/ring-io/tree/pure) is an experimental pure rust implementation. Different with liburing, it provides concurrent queue operations. 

**The correctness of memory orderings have not been verified.** You are welcome to help us.

SubmissionQueue (SQ) and CompletionQueue (CQ) are spsc in liburing. You may have to lock the whole queue in order to share it across threads.

ring-io#pure provides a spmc+mpsc SQ and a spmc CQ.

When popping SQEs, the producer of SQ is kernel and the consumers of SQ are worker threads. When pushing SQEs, the producers of SQ are worker threads and the consumer of SQ is kernel.

The producer of CQ is kernel and the consumers of CQ are reaper threads.

A worker thread pops SQEs from SQ, prepares IO operations and then pushs SQEs into SQ. A reaper thread pops CQEs from CQ and then wakes up tasks. A thread can be both a worker and a reaper.

## Proactor

It may be not a good choice to push SQEs and submit immediately. Because it increases the amount of syscalls and can be blocked by the internal mutex in kernel side. 

Proactors can add a submitter thread/task to determine how many operations can be batched, when to push and when to submit. 

If there are too much inflight operations, proactors had better apply back pressure in order to avoid CQ overflow.

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/Nugine/ring-io/blob/main/LICENSE
