use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::AudioContext;

pub mod filter;
pub mod oscillator;

async fn register_context() -> Result<AudioContext, JsValue> {
    let ctx = AudioContext::new().map_err(|e| {
        JsValue::from_str(&format!("AudioContext::new: {:?}", e))
    })?;
    waw::register_all(&ctx).await.map_err(|e| {
        JsValue::from_str(&format!("register_all: {:?}", e))
    })?;
    Ok(ctx)
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    let result = run().await;
    match &result {
        Err(e) => {
            let msg = format!("Error: {:?}", e);
            web_sys::console::error_1(&msg.clone().into());
            if let Some(window) = web_sys::window() {
                if let Some(doc) = window.document() {
                    if let Some(body) = doc.body() {
                        let el = doc.create_element("pre").ok();
                        if let Some(el) = el {
                            el.set_text_content(Some(&msg));
                            let _ = body.append_child(&el);
                        }
                    }
                }
            }
        }
        Ok(()) => {}
    }
    result
}

async fn run() -> Result<(), JsValue> {
    let ctx = register_context().await?;

    let osc = oscillator::OscillatorNode::new(&ctx, 110.0)?;
    let filter = filter::FilterNode::new(&ctx, 440.0)?;

    let osc_node = osc.node();
    let filter_node = filter.node();
    web_sys::AudioNode::connect_with_audio_node(
        osc_node.unchecked_ref(),
        filter_node.unchecked_ref(),
    )?;
    web_sys::AudioNode::connect_with_audio_node(
        filter_node.unchecked_ref(),
        ctx.destination().unchecked_ref(),
    )?;

    let params = web_sys::AudioWorkletNode::parameters(&osc_node)?;
    let frequency = web_sys::AudioParamMap::get(&params, "frequency")
        .ok_or("no frequency param")?;

    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let slider = document
        .query_selector("#frequency")
        .map_err(|_| "query failed")?
        .ok_or("no slider")?;
    let slider: web_sys::HtmlInputElement = slider.unchecked_into();
    let slider_clone = slider.clone();

    let freq = frequency;
    let on_input = Closure::<dyn Fn()>::new(move || {
        let val = slider.value().parse::<f32>().unwrap_or(440.0);
        freq.set_value(val);
    });
    slider_clone.add_event_listener_with_callback(
        "input",
        on_input.as_ref().unchecked_ref::<js_sys::Function>(),
    )?;
    on_input.forget();

    let resume = Closure::<dyn Fn()>::new(move || {
        let _ = ctx.resume();
    });
    document.add_event_listener_with_callback(
        "click",
        resume.as_ref().unchecked_ref::<js_sys::Function>(),
    )?;
    resume.forget();

    Ok(())
}
