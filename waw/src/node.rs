use std::ops::Deref;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use web_sys::AudioWorkletNode;

/// A wrapper around `AudioWorkletNode` that signals the processor to stop when dropped.
///
/// This ensures that when the last clone of the wrapper is dropped, the processor running
/// in the AudioWorklet thread will stop processing on the next process call.
pub struct AudioWorkletNodeWrapper {
    node: AudioWorkletNode,
    is_active: Arc<AtomicBool>,
}

impl AudioWorkletNodeWrapper {
    /// Creates a new wrapper around an AudioWorkletNode with a shared active state.
    pub(crate) fn new(node: AudioWorkletNode, is_active: Arc<AtomicBool>) -> Self {
        Self { node, is_active }
    }

    /// Returns a reference to the underlying AudioWorkletNode.
    pub fn node(&self) -> &AudioWorkletNode {
        &self.node
    }

    /// Consumes the wrapper and returns the underlying AudioWorkletNode.
    ///
    /// `Drop` is skipped so the processor continues running. The original
    /// `is_active` flag is left as-is
    pub fn into_inner(self) -> AudioWorkletNode {
        let node = self.node.clone();
        std::mem::forget(self);
        node
    }
}

impl Deref for AudioWorkletNodeWrapper {
    type Target = AudioWorkletNode;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl Drop for AudioWorkletNodeWrapper {
    fn drop(&mut self) {
        // Only deactivate when the last clone drops.
        if Arc::strong_count(&self.is_active) == 1 {
            self.is_active.store(false, Ordering::Release);
        }
    }
}

impl Clone for AudioWorkletNodeWrapper {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            is_active: self.is_active.clone(),
        }
    }
}
