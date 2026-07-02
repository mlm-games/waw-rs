# waw-rs

`waw-rs` helps you create Web Audio Worklets using Rust, without crying.

See [WebAssembly/Rust Tutorial: Pitch-perfect Audio Processing](https://www.toptal.com/webassembly/webassembly-rust-tutorial-web-audio)

This is all very experimental.

## Requirements

This crate **requires a nightly Rust toolchain**, WebAssembly target features `+atomics` and `+bulk-memory`, and `build-std` to recompile the standard library with threading support. A `.cargo/config.toml` like this is needed:

```toml
[unstable]
build-std = ['std', 'panic_abort']

[build]
target = ["wasm32-unknown-unknown"]
rustflags = [
    "-Ctarget-feature=+atomics,+bulk-memory",
    "-Clink-arg=--shared-memory",
    "-Clink-arg=--import-memory",
    "-Clink-arg=--export=__wasm_init_tls",
    "-Clink-arg=--export=__tls_size",
    "-Clink-arg=--export=__tls_align",
    "-Clink-arg=--export=__tls_base",
    "--cfg=web_sys_unstable_apis",
]
```

> **Note:** Audio Worklet contexts lack some browser APIs (`TextDecoder`, etc.). You may need a polyfill loaded via `audioWorklet.addModule(...)` before calling `register_all()`. See the [demo](demo) for an example.

## Usage

Add waw-rs to your Cargo.toml:

```toml
waw = { git = "https://github.com/Marcel-G/waw-rs" }
```

Implement the `Processor` trait and register your audio node:

```rust
use wasm_bindgen::prelude::*;
use waw::{register, AudioWorkletNodeWrapper, ParameterValuesRef, Processor};

#[derive(Clone)]
pub struct MyData {
    pub frequency: f32,
}

pub struct MyProcessor {
    phase: f32,
    frequency: f32,
}

impl Processor for MyProcessor {
    type Data = MyData;

    fn new(data: Self::Data) -> Self {
        Self { phase: 0.0, frequency: data.frequency }
    }

    fn process(
        &mut self,
        _inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
        sample_rate: f32,
        params: &ParameterValuesRef,
    ) {
        // ... your audio processing logic
    }
}

#[wasm_bindgen]
pub struct MyNode {
    wrapper: AudioWorkletNodeWrapper,
}

#[wasm_bindgen]
impl MyNode {
    #[wasm_bindgen(constructor)]
    pub fn new(ctx: &web_sys::AudioContext, frequency: f32) -> Result<MyNode, JsValue> {
        let data = MyData { frequency };
        
        // Configure input/output ports via AudioWorkletNodeOptions
        let options = web_sys::AudioWorkletNodeOptions::new();
        options.set_number_of_inputs(0);  // Generator: no inputs
        options.set_number_of_outputs(1); // Mono output
        
        let wrapper = MyProcessor::create_node(ctx, data, Some(&options))?;
        Ok(MyNode { wrapper })
    }

    #[wasm_bindgen(getter)]
    pub fn node(&self) -> web_sys::AudioWorkletNode {
        self.wrapper.node().clone()
    }
}

register!(MyProcessor, "my-processor");
```

Build with wasm-pack:

```bash
wasm-pack build --target web
```

Use in JavaScript:

```typescript
import init, { MyNode, register_all } from './pkg/your_project';

const main = async () => {
  await init();
  const context = new AudioContext();
  
  // Register all audio worklet processors
  await register_all(context);

  const node = new MyNode(context, 440.0);
  node.node.connect(context.destination);

  document.addEventListener('click', () => context.resume(), { once: true });
};

main();
```

See the [demo](demo) for a complete example.

## Links

- [wasm-bindgen WASM audio worklet](https://rustwasm.github.io/wasm-bindgen/examples/wasm-audio-worklet.html#wasm-audio-worklet)
