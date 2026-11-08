mod handlers;
pub mod model;
pub mod repository;
#[cfg(test)]
mod tests;

pub use handlers::configure;
pub(crate) use handlers::require_manager;
