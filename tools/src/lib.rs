//! Synchronous tool registration and execution for ARIA.

use std::collections::BTreeMap;

/// A tool that ARIA can invoke with string arguments.
pub trait Tool {
    fn name(&self) -> &str;
    fn execute(&self, args: &[String]) -> Result<String, String>;
}

/// Owns registered tools and executes them by name.
#[derive(Default)]
pub struct ToolManager {
    tools: BTreeMap<String, Box<dyn Tool>>,
}

impl ToolManager {
    /// Creates an empty tool manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a tool by name.
    ///
    /// A tool with the same name is replaced and returned.
    pub fn register(&mut self, tool: impl Tool + 'static) -> Option<Box<dyn Tool>> {
        let name = tool.name().to_owned();
        self.tools.insert(name, Box::new(tool))
    }

    /// Executes a registered tool.
    pub fn execute(&self, name: &str, args: &[String]) -> Result<String, String> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| format!("unknown tool: {name}"))?;

        tool.execute(args)
    }

    /// Lists all tools in ascending name order.
    pub fn list(&self) -> Vec<&dyn Tool> {
        self.tools.values().map(Box::as_ref).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Tool, ToolManager};

    struct EchoTool;

    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn execute(&self, args: &[String]) -> Result<String, String> {
            Ok(args.join(" "))
        }
    }

    struct FailingTool;

    impl Tool for FailingTool {
        fn name(&self) -> &str {
            "fail"
        }

        fn execute(&self, _args: &[String]) -> Result<String, String> {
            Err("tool failed".to_owned())
        }
    }

    #[test]
    fn registers_and_lists_tools_by_name() {
        let mut manager = ToolManager::new();

        manager.register(FailingTool);
        manager.register(EchoTool);

        let names: Vec<_> = manager.list().into_iter().map(Tool::name).collect();

        assert_eq!(names, ["echo", "fail"]);
    }

    #[test]
    fn executes_a_registered_tool() {
        let mut manager = ToolManager::new();
        manager.register(EchoTool);

        let args = ["hello".to_owned(), "ARIA".to_owned()];

        assert_eq!(manager.execute("echo", &args), Ok("hello ARIA".to_owned()));
    }

    #[test]
    fn propagates_tool_errors() {
        let mut manager = ToolManager::new();
        manager.register(FailingTool);

        assert_eq!(manager.execute("fail", &[]), Err("tool failed".to_owned()));
    }

    #[test]
    fn reports_an_unknown_tool() {
        let manager = ToolManager::new();

        assert_eq!(
            manager.execute("missing", &[]),
            Err("unknown tool: missing".to_owned())
        );
    }
}
