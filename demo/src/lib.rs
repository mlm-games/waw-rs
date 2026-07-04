use std::cell::RefCell;

use repose_core::*;
use repose_material::material3::{ButtonConfig, Slider, SliderConfig};
use repose_ui::{TextStyle, *};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::AudioContext;

pub mod filter;
pub mod oscillator;

thread_local! {
    static FREQ_PARAM: RefCell<Option<web_sys::AudioParam>> = RefCell::new(None);
    static AUDIO_CTX: RefCell<Option<AudioContext>> = RefCell::new(None);
    static RESUMED: RefCell<bool> = RefCell::new(false);
}

fn app(_s: &mut Scheduler, _rc: &repose_platform::RenderContext) -> View {
    let th = theme();
    let freq = remember_state_with_key("freq", || 440.0f32);
    let current_freq = *freq.borrow();

    let on_freq_change = {
        let f = freq.clone();
        move |new_val: f32| {
            *f.borrow_mut() = new_val;
            FREQ_PARAM.with(|p| {
                if let Some(ref param) = *p.borrow() {
                    let _ = param.set_value(new_val);
                }
            });
            request_frame();
        }
    };

    let on_toggle = move || {
        RESUMED.with(|r| {
            let mut resumed = r.borrow_mut();
            if !*resumed {
                AUDIO_CTX.with(|ctx| {
                    if let Some(ref c) = *ctx.borrow() {
                        let _ = c.resume();
                    }
                });
                *resumed = true;
            }
            request_frame();
        });
    };

    let resumed = RESUMED.with(|r| *r.borrow());

    Column(Modifier::new()
        .fill_max_size()
        .background(th.background)
        .padding(16.0)
        .align_items(AlignItems::CENTER))
    .child((
        Text("Web Audio Worklet")
            .size(24.0)
            .color(th.on_background)
            .modifier(Modifier::new().padding(8.0)),
        Box(Modifier::new().height(24.0)),
        Text(format!("Frequency: {:.0} Hz", current_freq))
            .size(18.0)
            .color(th.on_background),
        Box(Modifier::new().height(8.0)),
        Slider(current_freq, (20.0, 1200.0), Some(1.0), on_freq_change, SliderConfig::default()),
        Box(Modifier::new().height(24.0)),
        repose_material::material3::Button(
            Modifier::new(),
            on_toggle,
            ButtonConfig::default(),
            move || {
                Text(if resumed { "Playing" } else { "Click to Play" })
            },
        ),
    ))
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    let ctx = AudioContext::new().map_err(|e| {
        JsValue::from_str(&format!("AudioContext::new: {:?}", e))
    })?;
    waw::register_all(&ctx).await.map_err(|e| {
        JsValue::from_str(&format!("register_all: {:?}", e))
    })?;

    let osc = oscillator::OscillatorNode::new(&ctx, 440.0)?;
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

    FREQ_PARAM.with(|p| *p.borrow_mut() = Some(frequency));
    AUDIO_CTX.with(|c| *c.borrow_mut() = Some(ctx));

    let _ = repose_platform::web::run_web_app(
        app,
        repose_platform::web::WebOptions::new(None),
    );

    Ok(())
}
