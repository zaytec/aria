//! Core synchronous runtime for ARIA.

use agents::AgentRegistry;
use applications::ApplicationManager;
use audio::AudioEngine;
use commands::CommandManager;
use events::EventBus;
use filesystem::FileSystem;
use memory::Memory;
use plugins::PluginManager;
use scheduler::Scheduler;
use tools::ToolManager;

/// The current lifecycle state of the runtime.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RuntimeState {
    /// The runtime has been constructed but not started.
    #[default]
    Created,
    /// Runtime components are being initialized.
    Starting,
    /// The runtime is initialized and ready.
    Running,
    /// The runtime has been stopped.
    Stopped,
}

/// Published when runtime startup begins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeStarted;

/// Published when the runtime is ready.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeReady;

/// Published when the runtime stops.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeStopped;

/// Owns and coordinates ARIA's foundational runtime components.
#[derive(Default)]
pub struct Runtime {
    agent_registry: AgentRegistry,
    application_manager: ApplicationManager,
    audio_engine: AudioEngine,
    command_manager: CommandManager,
    event_bus: EventBus,
    filesystem: FileSystem,
    memory: Memory,
    plugin_manager: PluginManager,
    scheduler: Scheduler,
    state: RuntimeState,
    tool_manager: ToolManager,
}

impl Runtime {
    /// Creates a runtime with empty foundational components.
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the runtime synchronously.
    pub fn start(&mut self) {
        if matches!(self.state, RuntimeState::Starting | RuntimeState::Running) {
            return;
        }

        self.state = RuntimeState::Starting;
        self.event_bus.publish(&RuntimeStarted);

        self.audio_engine.initialize();
        self.plugin_manager.initialize_all();
        self.agent_registry.initialize();

        self.state = RuntimeState::Running;
        self.event_bus.publish(&RuntimeReady);

        println!(
            "Starting ARIA Runtime...\n\n\
             Loading Event Bus...\n\
             Loading Memory...\n\n\
             Runtime Ready."
        );
    }

    /// Stops the runtime synchronously.
    pub fn stop(&mut self) {
        if self.state == RuntimeState::Stopped {
            return;
        }

        self.state = RuntimeState::Stopped;
        self.event_bus.publish(&RuntimeStopped);
    }

    /// Returns the current lifecycle state.
    pub fn state(&self) -> RuntimeState {
        self.state
    }

    /// Returns the runtime's agent registry.
    pub fn agent_registry(&self) -> &AgentRegistry {
        &self.agent_registry
    }

    /// Returns mutable access to the runtime's agent registry.
    pub fn agent_registry_mut(&mut self) -> &mut AgentRegistry {
        &mut self.agent_registry
    }

    /// Returns the runtime's application manager.
    pub fn application_manager(&self) -> &ApplicationManager {
        &self.application_manager
    }

    /// Returns mutable access to the runtime's application manager.
    pub fn application_manager_mut(&mut self) -> &mut ApplicationManager {
        &mut self.application_manager
    }

    /// Returns the runtime's audio engine.
    pub fn audio_engine(&self) -> &AudioEngine {
        &self.audio_engine
    }

    /// Returns mutable access to the runtime's audio engine.
    pub fn audio_engine_mut(&mut self) -> &mut AudioEngine {
        &mut self.audio_engine
    }

    /// Returns the runtime's command manager.
    pub fn command_manager(&self) -> &CommandManager {
        &self.command_manager
    }

    /// Returns mutable access to the runtime's command manager.
    pub fn command_manager_mut(&mut self) -> &mut CommandManager {
        &mut self.command_manager
    }

    /// Returns the runtime's event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Returns mutable access to the runtime's event bus.
    pub fn event_bus_mut(&mut self) -> &mut EventBus {
        &mut self.event_bus
    }

    /// Returns the runtime's virtual file system.
    pub fn filesystem(&self) -> &FileSystem {
        &self.filesystem
    }

    /// Returns mutable access to the runtime's virtual file system.
    pub fn filesystem_mut(&mut self) -> &mut FileSystem {
        &mut self.filesystem
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

    /// Returns the runtime's tool manager.
    pub fn tool_manager(&self) -> &ToolManager {
        &self.tool_manager
    }

    /// Returns mutable access to the runtime's tool manager.
    pub fn tool_manager_mut(&mut self) -> &mut ToolManager {
        &mut self.tool_manager
    }
}

#[cfg(test)]
mod tests {
    use super::{Runtime, RuntimeReady, RuntimeStarted, RuntimeState, RuntimeStopped};
    use plugins::Plugin;
    use std::sync::{Arc, Mutex};

    struct TestPlugin {
        initialized: Arc<Mutex<bool>>,
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            "test"
        }

        fn initialize(&mut self) {
            *self.initialized.lock().unwrap() = true;
        }
    }

    #[test]
    fn transitions_through_the_lifecycle() {
        let mut runtime = Runtime::new();

        assert_eq!(runtime.state(), RuntimeState::Created);

        runtime.start();
        assert_eq!(runtime.state(), RuntimeState::Running);

        runtime.stop();
        assert_eq!(runtime.state(), RuntimeState::Stopped);
    }

    #[test]
    fn publishes_lifecycle_events_in_order() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = Runtime::new();

        let received = Arc::clone(&events);
        runtime
            .event_bus_mut()
            .subscribe::<RuntimeStarted>(move |_| {
                received.lock().unwrap().push("started");
            });

        let received = Arc::clone(&events);
        runtime.event_bus_mut().subscribe::<RuntimeReady>(move |_| {
            received.lock().unwrap().push("ready");
        });

        let received = Arc::clone(&events);
        runtime
            .event_bus_mut()
            .subscribe::<RuntimeStopped>(move |_| {
                received.lock().unwrap().push("stopped");
            });

        runtime.start();
        runtime.stop();

        assert_eq!(*events.lock().unwrap(), ["started", "ready", "stopped"]);
    }

    #[test]
    fn initializes_components_and_preserves_pending_jobs() {
        let plugin_initialized = Arc::new(Mutex::new(false));
        let job_executed = Arc::new(Mutex::new(false));
        let mut runtime = Runtime::new();

        runtime.plugin_manager_mut().register(TestPlugin {
            initialized: Arc::clone(&plugin_initialized),
        });

        let executed = Arc::clone(&job_executed);
        runtime.scheduler_mut().schedule(move || {
            *executed.lock().unwrap() = true;
        });

        runtime.start();

        assert!(runtime.audio_engine().is_initialized());
        assert!(runtime.agent_registry().is_initialized());
        assert!(*plugin_initialized.lock().unwrap());
        assert_eq!(runtime.scheduler_mut().run_pending(), 1);
        assert!(*job_executed.lock().unwrap());
    }
}
