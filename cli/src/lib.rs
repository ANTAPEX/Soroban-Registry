//! The handful of modules the benches and integration tests reach for. The
//! binary is the real crate; `#[path]` keeps these names stable while the
//! files themselves live in the command tree.

#[path = "commands/cache.rs"]
pub mod cache;
#[path = "support/diagnostic.rs"]
pub mod diagnostic;
#[path = "commands/notification.rs"]
pub mod notification;
#[path = "support/output_format.rs"]
pub mod output_format;
#[path = "support/profiler.rs"]
pub mod profiler;
#[path = "support/search_pagination.rs"]
pub mod search_pagination;
#[path = "support/table_format.rs"]
pub mod table_format;
