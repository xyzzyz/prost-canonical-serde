//! Pre-generated prost output for documentation and tests.

extern crate alloc;

pub mod demo {
    include!("demo.rs");
}

#[expect(
    clippy::doc_markdown,
    clippy::module_inception,
    reason = "Generated prost code uses upstream docs and nested module names."
)]
pub mod kitchen_sink {
    include!("kitchen_sink.rs");

    pub use self::kitchen_sink::Choice;
}

pub use kitchen_sink::*;
