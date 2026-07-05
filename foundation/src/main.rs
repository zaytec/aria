use events::{EventBus, ProjectCreated};
use memory::Memory;

fn main() {
    let mut event_bus = EventBus::new();
    let mut memory = Memory::new();

    event_bus.subscribe::<ProjectCreated>(|event| {
        println!("Project created: {}", event.name());
    });

    println!(
        "=================================\n\
         ARIA Foundation v0.0.1\n\
         =================================\n\n\
         Initializing...\n\n\
         ✓ Runtime\n\
         ✓ Event Bus\n\
         ✓ Memory\n\
         ✓ Agent Registry"
    );

    println!("\nPublishing demonstration event...");
    event_bus.publish(&ProjectCreated::new("ARIA"));

    println!("\nStoring demonstration memory...");
    memory.set("project.name", "ARIA");

    if let Some(project_name) = memory.get("project.name") {
        println!("Retrieved project: {project_name}");
    }

    println!("\nSystem Ready.");
}
