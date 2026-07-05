//! Core synchronous runtime for ARIA.

use events::EventBus;
use memory::Memory;
use plugins::PluginManager;
use scheduler::Scheduler;

/// Owns and coordinates ARIA's foundational runtime components.
#[derive(Default)]
pub struct Runtime {
    event_bus: EventBus,
    memory: Memory,
    plugin_manager: PluginManager,
    scheduler: Scheduler,
}

impl Runtime {
    /// Creates a runtime with empty foundational components.
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the runtime synchronously.
    pub fn start(&self) {
        println!(
            "Starting ARIA Runtime...\n\n\
             Loading Event Bus...\n\
             Loading Memory...\n\n\
             Runtime Ready."
        );
    }

    /// Returns the runtime's event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Returns mutable access to the runtime's event bus.
    pub fn event_bus_mut(&mut self) -> &mut EventBus {
        &mut self.event_bus
    }

    /// Returns the runtime's memory store.
    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    /// Returns mutable access to the runtime's memory store.
    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }

    /// Returns the runtime's plugin manager.
    pub fn plugin_manager(&self) -> &PluginManager {
        &self.plugin_manager
    }

    /// Returns mutable access to the runtime's plugin manager.
    pub fn plugin_manager_mut(&mut self) -> &mut PluginManager {
        &mut self.plugin_manager
    }

    /// Returns the runtime's scheduler.
    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    /// Returns mutable access to the runtime's scheduler.
    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }
}
