mod builder;
mod handle;

pub use builder::MonoEffectBuilder;
pub use handle::MonoEffectHandle;

use kira::{command_writers_and_readers, effect::Effect};

struct MonoEffect {
    command_readers: CommandReaders,
    enabled: bool,
}

impl MonoEffect {
    fn new(builder: MonoEffectBuilder, command_readers: CommandReaders) -> Self {
        Self { command_readers, enabled: builder.enabled }
    }
}

impl Effect for MonoEffect {
	fn on_start_processing(&mut self) {
        if let Some(enabled) = self.command_readers.set_enable.read() {
            self.enabled = enabled;
        }
	}
    
    fn process(&mut self, input: &mut [kira::Frame], _dt: f64, _info: &kira::info::Info) {
        if self.enabled {
            for frame in input {
                let mono_frame = frame.as_mono();
                *frame = mono_frame;
            }
        }
    }
}

command_writers_and_readers! {
    set_enable: bool,
}