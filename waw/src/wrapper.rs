use crate::{
    buffer::{InputBuffer, OutputBuffer, ParameterBuffer},
    processor::Processor,
};
use js_sys::{Array, Iterator, Object};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletGlobalScope, AudioWorkletNodeOptions, AudioWorkletProcessor};
use web_thread::web::audio_worklet::ExtendAudioWorkletProcessor;

/// Internal data structure that wraps user data with lifecycle management.
pub struct ProcessorWrapperData<D> {
    /// The user's processor data
    pub user_data: D,
    /// Shared flag indicating if the processor should continue processing
    pub is_active: Arc<AtomicBool>,
}

/// A wrapper struct for a type implementing the `Processor` trait, used to interface with the Web Audio API.
pub struct ProcessorWrapper<P: Processor> {
    processor: P,
    input_buffer: InputBuffer,
    output_buffer: OutputBuffer,
    parameter_buffer: ParameterBuffer,
    is_active: Arc<AtomicBool>,
    sample_rate: f32,
}

impl<P: Processor> ExtendAudioWorkletProcessor for ProcessorWrapper<P> {
    type Data = ProcessorWrapperData<P::Data>;

    fn new(
        _this: AudioWorkletProcessor,
        data: Option<Self::Data>,
        options: AudioWorkletNodeOptions,
    ) -> Self {
        let wrapper_data = data.expect("Data required");
        let processor = P::new(wrapper_data.user_data);
        let is_active = wrapper_data.is_active;

        // Cache sample rate
        let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
        let sample_rate = global.sample_rate();

        let initial_buffer_size = 128;

        let channel_count = options.get_channel_count().unwrap_or(1);
        let input_count = options.get_number_of_inputs().unwrap_or(0);
        let output_count = options.get_number_of_outputs().unwrap_or(1);

        let input_buffer = InputBuffer::new(
            (input_count * channel_count).try_into().unwrap(),
            initial_buffer_size,
        );

        let output_buffer = OutputBuffer::new(
            (output_count * channel_count).try_into().unwrap(),
            initial_buffer_size,
        );

        let parameter_buffer = ParameterBuffer::new();

        Self {
            processor,
            input_buffer,
            output_buffer,
            parameter_buffer,
            is_active,
            sample_rate,
        }
    }

    fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
        if !self.is_active.load(Ordering::Acquire) {
            return false;
        }

        // Fill input buffers from JS
        self.input_buffer.fill_from_js(&inputs);

        // Derive output buffer size from the outputs array itself,
        // not from the input buffer (generators have no inputs).
        let output_block_size = if outputs.length() > 0 {
            let ports: Array = outputs.get(0).unchecked_into();
            if ports.length() > 0 {
                let float_array: Float32Array = ports.get(0).unchecked_into();
                float_array.length() as usize
            } else {
                128
            }
        } else {
            128
        };

        self.output_buffer.ensure_size(output_block_size);
        self.output_buffer.ensure_channels_from_js(&outputs);
        self.output_buffer.clear();

        self.parameter_buffer.set_buffer_size(output_block_size);
        self.parameter_buffer.fill_from_js(&parameters);

        let input_refs = self.input_buffer.get_refs();
        let mut output_refs = self.output_buffer.get_mut_refs();
        let params = self.parameter_buffer.get_ref();

        self.processor
            .process(&input_refs, &mut output_refs, self.sample_rate, &params);

        self.output_buffer.copy_to_js(&outputs);

        true
    }

    fn parameter_descriptors() -> Iterator {
        let arr = Array::new();
        for desc in P::parameter_descriptors() {
            arr.push(&desc.into());
        }
        arr.values()
    }
}
