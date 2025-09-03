use super::CommandWriters;

pub struct MonoEffectHandle {
    pub(super) command_writers: CommandWriters,
}

impl MonoEffectHandle {
    pub fn new(command_writers: CommandWriters) -> Self {
        Self { command_writers }
    }

    pub fn set_enable(&mut self, enabled: bool) {
        self.command_writers.set_enable.write(enabled);
    }
}