mod handlers;
mod model;

pub use model::ACCOUNT_TYPES;
pub mod repository;
#[cfg(test)]
mod tests;

pub use handlers::configure;
