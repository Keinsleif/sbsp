use core::f32;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use kira::{Frame, effect::Effect};

#[derive(Clone, Default)]
pub struct SharedLevel {
    left: Arc<AtomicU32>,
    right: Arc<AtomicU32>,
}

impl SharedLevel {
    pub fn set(&self, left: f32, right: f32) {
        self.left.store(left.to_bits(), Ordering::Relaxed);
        self.right.store(right.to_bits(), Ordering::Relaxed);
    }

    pub fn get(&self) -> (f32, f32) {
        let l = f32::from_bits(self.left.load(Ordering::Relaxed));
        let r = f32::from_bits(self.right.load(Ordering::Relaxed));
        (l, r)
    }
}

pub struct LevelMeterEffect {
    shared: SharedLevel,
}

impl LevelMeterEffect {
    pub fn new(shared: SharedLevel) -> Self {
        Self { shared }
    }
}

impl Effect for LevelMeterEffect {
    fn process(&mut self, input: &mut [kira::Frame], _dt: f64, _info: &kira::info::Info) {
        let frame = input.iter().fold(
            Frame {
                left: f32::NAN,
                right: f32::NAN,
            },
            |m, v| Frame {
                left: v.left.abs().max(m.left),
                right: v.right.abs().max(m.right),
            },
        );
        self.shared.set(frame.left, frame.right);
    }
}
