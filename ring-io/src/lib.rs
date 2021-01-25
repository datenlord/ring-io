#![deny(single_use_lifetimes, missing_debug_implementations, clippy::all)]

mod sys;

#[macro_use]
mod utils;

pub mod cq;
pub mod cqe;
pub mod register;
pub mod ring;
pub mod sq;
pub mod sqe;

pub use self::cq::CompletionQueue;
pub use self::cqe::CQE;
pub use self::ring::{Ring, RingBuilder};
pub use self::sq::SubmissionQueue;
pub use self::sqe::SQE;
