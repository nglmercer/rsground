mod async_from;
mod to_stream;

pub use async_from::AsyncInto;
pub use to_stream::ToStream;

use std::sync::Arc;

/// Use `Arc` instead of `String` because **user's id** is a immutable string
/// and is shared across threads.
///
/// https://blocklisted.github.io/blog/arc_str_vs_string_is_it_really_faster/
pub type ArcStr = Arc<str>;
