//! Synchronous agent registration and discovery for ARIA.

use std::collections::BTreeMap;

/// An agent that can be registered with the ARIA runtime.
pub trait Agent {
    /// Returns the agent's unique name.
    fn name(&self) -> &str;
}

/// Owns agents and provides deterministic lookup and listing.
#[derive(Default)]
pub struct AgentRegistry {
    agents: BTreeMap<String, Box<dyn Agent>>,
    initialized: bool,
}

impl AgentRegistry {
    /// Creates an empty agent registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes the agent registry.
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Returns whether the registry has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Registers an agent by name.
    ///
    /// If an agent with the same name already exists, it is replaced and returned.
    pub fn register(&mut self, agent: impl Agent + 'static) -> Option<Box<dyn Agent>> {
        let name = agent.name().to_owned();
        self.agents.insert(name, Box::new(agent))
    }

    /// Retrieves an agent by name.
    pub fn get(&self, name: &str) -> Option<&dyn Agent> {
        self.agents.get(name).map(Box::as_ref)
    }

    /// Lists all agents in ascending name order.
    pub fn list(&self) -> Vec<&dyn Agent> {
        self.agents.values().map(Box::as_ref).collect()
    }

    /// Removes and returns an agent by name.
    pub fn remove(&mut self, name: &str) -> Option<Box<dyn Agent>> {
        self.agents.remove(name)
    }
}

#[cfg(test)]
mod tests {
    use super::{Agent, AgentRegistry};

    struct TestAgent {
        name: &'static str,
    }

    impl TestAgent {
        fn new(name: &'static str) -> Self {
            Self { name }
        }
    }

    impl Agent for TestAgent {
        fn name(&self) -> &str {
            self.name
        }
    }

    #[test]
    fn registers_an_agent() {
        let mut registry = AgentRegistry::new();

        let replaced = registry.register(TestAgent::new("composer"));

        assert!(replaced.is_none());
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn retrieves_an_agent() {
        let mut registry = AgentRegistry::new();
        registry.register(TestAgent::new("composer"));

        let agent = registry.get("composer");

        assert_eq!(agent.map(Agent::name), Some("composer"));
    }

    #[test]
    fn removes_an_agent() {
        let mut registry = AgentRegistry::new();
        registry.register(TestAgent::new("composer"));

        let removed = registry.remove("composer");

        assert_eq!(removed.as_deref().map(Agent::name), Some("composer"));
        assert!(registry.get("composer").is_none());
    }

    #[test]
    fn lists_agents_by_name() {
        let mut registry = AgentRegistry::new();
        registry.register(TestAgent::new("reasoner"));
        registry.register(TestAgent::new("composer"));
        registry.register(TestAgent::new("planner"));

        let names: Vec<_> = registry.list().into_iter().map(Agent::name).collect();

        assert_eq!(names, vec!["composer", "planner", "reasoner"]);
    }
}
