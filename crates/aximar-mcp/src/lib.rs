// Re-export from aximar-core for backward compatibility
pub use aximar_core::capture;
pub use aximar_core::commands;
pub use aximar_core::log;
pub use aximar_core::notebook;
pub use aximar_core::registry;

mod convert;
pub mod http;
mod params;
pub mod server;
