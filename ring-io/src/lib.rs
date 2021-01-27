#![deny(single_use_lifetimes, missing_debug_implementations, clippy::all)]

#[cfg(target_pointer_width = "16")]
compile_error!("ring-io does not support this target");

mod sys;

#[macro_use]
mod utils;

pub mod cq;
pub mod cqe;
pub mod register;
pub mod ring;
pub mod sq;
pub mod sqe;
