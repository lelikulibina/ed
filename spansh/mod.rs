mod client;
mod query;

pub use client::{SpanshClient, SpanshError};
pub use query::{BodySearchQuery, Coordinates, QueryError};
