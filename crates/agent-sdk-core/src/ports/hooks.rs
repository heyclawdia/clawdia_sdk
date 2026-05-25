//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the hooks
//! portion of that contract.
//!
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::{
    domain::AgentError,
    package_hooks::{HookExecutorRef, HookInput, HookResponse},
};

/// Port or behavior contract for hook executor. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait HookExecutor: Send + Sync {
    /// Returns executor ref for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    fn executor_ref(&self) -> &HookExecutorRef;
    /// Invokes one hook with the already-projected hook input.
    /// Implementations may call hook code or transport, but runtime-owned
    /// ordering, timeout, mutation-right checks, and journal evidence stay
    /// outside this port call.
    fn invoke(&self, input: HookInput) -> Result<HookExecutionOutcome, AgentError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Carries hook execution outcome data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct HookExecutionOutcome {
    /// Response DTO returned by a host adapter or protocol endpoint.
    pub response: HookResponse,
    /// elapsed ms duration in milliseconds.
    pub elapsed_ms: u64,
}

impl HookExecutionOutcome {
    /// Creates a new ports::hooks value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(response: HookResponse, elapsed_ms: u64) -> Self {
        Self {
            response,
            elapsed_ms,
        }
    }
}

/// Port or behavior contract for hook executor registry. Implementors
/// should preserve policy, redaction, idempotency, and replay
/// expectations from the surrounding module. Implementations may
/// perform side effects only as described by the trait methods.
pub trait HookExecutorRegistry: Send + Sync {
    /// Resolves resolve through the configured ports::hooks boundary.
    /// Concrete implementations own any backing-store, filesystem, or network
    /// side effects.
    fn resolve(&self, executor_ref: &HookExecutorRef) -> Option<Arc<dyn HookExecutor>>;

    /// Reads the stored contains without registry or runtime work.
    /// This checks in-memory hook registry membership and does not invoke the hook executor.
    fn contains(&self, executor_ref: &HookExecutorRef) -> bool {
        self.resolve(executor_ref).is_some()
    }
}

#[derive(Clone, Default)]
/// Carries in memory hook executor registry data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct InMemoryHookExecutorRegistry {
    executors: Arc<Mutex<BTreeMap<HookExecutorRef, Arc<dyn HookExecutor>>>>,
}

impl InMemoryHookExecutorRegistry {
    /// Creates a new ports::hooks value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds data to this in-memory ports::hooks collection. It does not
    /// perform external I/O, execute tools, or append journals.
    pub fn register<E>(&self, executor: E) -> Result<(), AgentError>
    where
        E: HookExecutor + 'static,
    {
        self.executors
            .lock()
            .map_err(|_| AgentError::contract_violation("hook executor registry lock poisoned"))?
            .insert(executor.executor_ref().clone(), Arc::new(executor));
        Ok(())
    }
}

impl HookExecutorRegistry for InMemoryHookExecutorRegistry {
    fn resolve(&self, executor_ref: &HookExecutorRef) -> Option<Arc<dyn HookExecutor>> {
        self.executors
            .lock()
            .expect("hook executor registry lock")
            .get(executor_ref)
            .cloned()
    }
}
