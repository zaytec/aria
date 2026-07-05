//! Synchronous plugin management for ARIA.

/// A component that extends ARIA with additional behavior.
pub trait Plugin {
    /// Returns the plugin's name.
    fn name(&self) -> &str;

    /// Initializes the plugin.
    fn initialize(&mut self);
}

/// Owns and initializes registered plugins.
#[derive(Default)]
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    /// Creates an empty plugin manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a plugin.
    pub fn register(&mut self, plugin: impl Plugin + 'static) {
        self.plugins.push(Box::new(plugin));
    }

    /// Initializes every plugin in registration order.
    pub fn initialize_all(&mut self) {
        for plugin in &mut self.plugins {
            plugin.initialize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Plugin, PluginManager};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct TestPlugin {
        name: &'static str,
        initialization_order: Rc<RefCell<Vec<&'static str>>>,
    }

    impl TestPlugin {
        fn new(name: &'static str, initialization_order: Rc<RefCell<Vec<&'static str>>>) -> Self {
            Self {
                name,
                initialization_order,
            }
        }
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            self.name
        }

        fn initialize(&mut self) {
            self.initialization_order.borrow_mut().push(self.name);
        }
    }

    #[test]
    fn registers_plugins() {
        let mut manager = PluginManager::new();
        let initialization_order = Rc::new(RefCell::new(Vec::new()));

        manager.register(TestPlugin::new("test", initialization_order));

        assert_eq!(manager.plugins.len(), 1);
        assert_eq!(manager.plugins[0].name(), "test");
    }

    #[test]
    fn initializes_plugins_in_registration_order() {
        let initialization_order = Rc::new(RefCell::new(Vec::new()));
        let mut manager = PluginManager::new();

        for name in ["first", "second", "third"] {
            manager.register(TestPlugin::new(name, Rc::clone(&initialization_order)));
        }

        manager.initialize_all();

        assert_eq!(
            *initialization_order.borrow(),
            vec!["first", "second", "third"]
        );
    }
}
