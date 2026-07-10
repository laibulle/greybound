use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

pub(super) fn atomic_f32(value: f32) -> Arc<AtomicU32> {
    Arc::new(AtomicU32::new(value.to_bits()))
}

pub(super) fn store_f32(slot: &AtomicU32, value: f32) {
    slot.store(value.to_bits(), Ordering::Relaxed);
}

pub(super) fn load_f32(slot: &AtomicU32) -> f32 {
    f32::from_bits(slot.load(Ordering::Relaxed))
}

pub(super) fn protect_dac(sample: f32) -> f32 {
    sample.clamp(-0.98, 0.98)
}
