//! Resource-reader helpers layered over explicit URI resolvers and core content refs.
//! Use these modules when a host wants toolkit tools to read approved resources.
//! Resolver implementations own any backing-store or network side effects.
//!
mod executor;
mod policy;
mod resolver;
mod types;

pub use executor::ResourceReaderExecutor;
pub use policy::memory_read_policy;
pub use resolver::InMemoryResourceResolver;
pub use types::ResourceReaderRequest;
