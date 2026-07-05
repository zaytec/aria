//! Synchronous command registration and execution for ARIA.

use std::collections::BTreeMap;

/// An action that can be executed by ARIA.
pub trait Command {
    /// Returns the command's unique name.
    fn name(&self) -> &str;

    /// Executes the command synchronously.
    fn execute(&mut self);
}

/// Owns registered commands and executes them by name.
#[derive(Default)]
pub struct CommandManager {
    commands: BTreeMap<String, Box<dyn Command>>,
}

impl CommandManager {
    /// Creates an empty command manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a command by name.
    ///
    /// If a command with the same name already exists, it is replaced and returned.
    pub fn register(&mut self, command: impl Command + 'static) -> Option<Box<dyn Command>> {
        let name = command.name().to_owned();
        self.commands.insert(name, Box::new(command))
    }

    /// Executes a command by name, returning whether it was found.
    pub fn execute(&mut self, name: &str) -> bool {
        let Some(command) = self.commands.get_mut(name) else {
            return false;
        };

        command.execute();
        true
    }

    /// Lists all commands in ascending name order.
    pub fn list(&self) -> Vec<&dyn Command> {
        self.commands.values().map(Box::as_ref).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, CommandManager};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct TestCommand {
        name: &'static str,
        executions: Rc<RefCell<Vec<&'static str>>>,
    }

    impl TestCommand {
        fn new(name: &'static str, executions: Rc<RefCell<Vec<&'static str>>>) -> Self {
            Self { name, executions }
        }
    }

    impl Command for TestCommand {
        fn name(&self) -> &str {
            self.name
        }

        fn execute(&mut self) {
            self.executions.borrow_mut().push(self.name);
        }
    }

    #[test]
    fn registers_and_lists_commands_by_name() {
        let executions = Rc::new(RefCell::new(Vec::new()));
        let mut manager = CommandManager::new();

        manager.register(TestCommand::new("stop", Rc::clone(&executions)));
        manager.register(TestCommand::new("play", executions));

        let names: Vec<_> = manager.list().into_iter().map(Command::name).collect();

        assert_eq!(names, ["play", "stop"]);
    }

    #[test]
    fn executes_a_registered_command() {
        let executions = Rc::new(RefCell::new(Vec::new()));
        let mut manager = CommandManager::new();
        manager.register(TestCommand::new("play", Rc::clone(&executions)));

        assert!(manager.execute("play"));
        assert_eq!(*executions.borrow(), ["play"]);
    }

    #[test]
    fn reports_an_unknown_command() {
        let mut manager = CommandManager::new();

        assert!(!manager.execute("missing"));
    }
}
