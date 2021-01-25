# ring-io

> An easy-to-use interface for io_uring

This project is inspired by [iou](https://crates.io/crates/iou) and [io-uring](https://crates.io/crates/io-uring).

**ring-io is in the early development stage. The API is subject to change without notice.**

You are strongly recommended not to deploy the code under the current version. Tests, bug reports, user feedback, and other experiments are all welcome at this stage.

ring-io is currently a wrapper around the [liburing](https://git.kernel.dk/cgit/liburing/) library. But it may replace the underlying bindings ([uring-sys](https://crates.io/crates/uring-sys)) with a pure rust implementation in the future.

## Safety

ring-io provides both safe and unsafe APIs.

Preparing IO operations is completely unsafe. Users must ensure that the buffers and file descriptors are regarded as borrowed or taken by the kernel during the lifetime of the IO.

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/Nugine/ring-io/blob/main/LICENSE
