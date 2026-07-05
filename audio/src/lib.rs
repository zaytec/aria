//! Synchronous audio playback primitives for ARIA.

/// ARIA's synchronous audio engine.
#[derive(Default)]
pub struct AudioEngine {
    initialized: bool,
}

impl AudioEngine {
    /// Creates an uninitialized audio engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes the audio engine.
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Returns whether the audio engine has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Plays a named sound synchronously.
    pub fn play(&self, sound: &str) {
        println!("Playing sound: {sound}");
    }
}

#[cfg(test)]
mod tests {
    use super::AudioEngine;

    #[test]
    fn initializes_successfully() {
        let mut engine = AudioEngine::new();

        engine.initialize();

        assert!(engine.is_initialized());
    }

    #[test]
    fn accepts_sound_names_without_panicking() {
        let engine = AudioEngine::new();

        engine.play("startup-chime");
        engine.play("");
    }
}
