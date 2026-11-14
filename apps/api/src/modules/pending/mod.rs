//! The confirm page's API, tronc's [`PendingPage`] on Go's paths: the person's pending agent
//! actions, confirmed or dismissed from their own browser session only.
//!
//! [`PendingPage`]: tronc::mcp::PendingPage

mod router;

pub use router::{Pending, routes};
