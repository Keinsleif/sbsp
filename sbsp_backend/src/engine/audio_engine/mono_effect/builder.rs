use kira::effect::EffectBuilder;

use crate::engine::audio_engine::mono_effect::{
    MonoEffect, command_writers_and_readers, handle::MonoEffectHandle,
};

pub struct MonoEffectBuilder {
    pub enabled: bool,
}

impl MonoEffectBuilder {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl Default for MonoEffectBuilder {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl EffectBuilder for MonoEffectBuilder {
    type Handle = MonoEffectHandle;

    fn build(self) -> (Box<dyn kira::effect::Effect>, Self::Handle) {
        let (command_writers, command_readers) = command_writers_and_readers();
        (
            Box::new(MonoEffect::new(self, command_readers)),
            MonoEffectHandle::new(command_writers),
        )
    }
}
