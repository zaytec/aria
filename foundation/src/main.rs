use events::{EventBus, ProjectCreated};

fn main() {
    let mut event_bus = EventBus::new();

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

    println!("\nSystem Ready.");
}
