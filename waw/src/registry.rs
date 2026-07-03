use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use crate::node::AudioWorkletNodeWrapper;
use crate::processor::Processor;
use crate::wrapper::{ProcessorWrapper, ProcessorWrapperData};
use wasm_bindgen::prelude::*;
use web_thread::web::audio_worklet::BaseAudioContextExt;
use web_sys::AudioContext;



/// Registration entry for inventory
#[derive(Clone, Copy)]
pub struct ProcessorRegistration {
    /// The name of the processor to register
    pub name: &'static str,
    /// The function used to register the processor
    pub register_fn: fn() -> Result<(), JsValue>,
}

impl ProcessorRegistration {
    /// Creates a new `ProcessorRegistration` with the given name and registration function.
    pub const fn new(name: &'static str, register_fn: fn() -> Result<(), JsValue>) -> Self {
        Self { name, register_fn }
    }
}

// Collect all registrations using inventory
inventory::collect!(ProcessorRegistration);

/// Register all processors in the given audio context
pub async fn register_all(ctx: &AudioContext) -> Result<(), JsValue> {
    let registrations: Vec<_> = inventory::iter::<ProcessorRegistration>()
        .map(|reg| *reg)
        .collect();

    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let errors_clone = errors.clone();
    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = completed.clone();

    let _handle = ctx
        .clone()
        .register_thread(None, move || {
            for reg in &registrations {
                if let Err(e) = (reg.register_fn)() {
                    errors_clone.lock().unwrap().push(format!(
                        "Failed to register {}: {:?}",
                        reg.name, e
                    ));
                }
            }
            completed_clone.store(true, Ordering::Release);
        })
        .await
        .map_err(|e| JsValue::from_str(&format!("register_thread: {:?}", e)))?;

    // `register_thread` returns before the closure finishes (ThreadMemory is sent
    // before the user task runs). Wait for the closure to complete.
    while !completed.load(Ordering::Acquire) {
        web_thread::web::yield_now_async(web_thread::web::YieldTime::UserBlocking).await;
    }

    // Yield to allow AudioWorklet sync messages (from registerProcessor) to
    // propagate to the main thread before creating AudioWorkletNodes.
    web_thread::web::yield_now_async(web_thread::web::YieldTime::UserBlocking).await;

    let errors = errors.lock().unwrap();
    if !errors.is_empty() {
        return Err(JsValue::from_str(&errors.join("; ")));
    }

    Ok(())
}

/// Create an audio worklet node
pub fn create_node<P: Processor>(
    ctx: &AudioContext,
    name: &str,
    data: P::Data,
    options: Option<&web_sys::AudioWorkletNodeOptions>,
) -> Result<AudioWorkletNodeWrapper, JsValue> {
    use web_thread::web::audio_worklet::BaseAudioContextExt;

    // Create the shared active state flag
    let is_active = Arc::new(AtomicBool::new(true));

    // Wrap the user data with the active state
    let wrapper_data = ProcessorWrapperData {
        user_data: data,
        is_active: is_active.clone(),
    };

    // Create the node
    let node = ctx
        .audio_worklet_node::<ProcessorWrapper<P>>(name, wrapper_data, options)
        .map_err(|e| JsValue::from_str(&format!("Failed to create node: {:?}", e)))?;

    // Return the wrapped node with the shared active state
    Ok(AudioWorkletNodeWrapper::new(node, is_active))
}
