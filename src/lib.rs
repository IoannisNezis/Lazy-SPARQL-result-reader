use js_sys::{Function, Uint8Array};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, panic};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{ReadableStream, ReadableStreamDefaultReader};

#[derive(Serialize, Deserialize, Debug)]
struct Header {
    head: Head,
}

#[derive(Serialize, Deserialize, Debug)]
struct Head {
    vars: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Binding(HashMap<String, RDFValue>);

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RDFValue {
    Uri {
        value: String,
    },
    Literal {
        value: String,
        #[serde(rename = "xml:lang", skip_serializing_if = "Option::is_none")]
        lang: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        datatype: Option<String>,
    },
    Bnode {
        value: String,
    },
}

enum ScannerState {
    ReadingHead,
    SearchingBindings,
    SearchingBinding,
    ReadingBinding(u8),
}

#[wasm_bindgen]
pub async fn read(
    stream: ReadableStream,
    batch_size: usize,
    callback: Function,
) -> Result<(), JsValue> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());
    let reader: ReadableStreamDefaultReader = stream.get_reader().unchecked_into();
    let mut buffer = String::new();
    let mut state = ScannerState::ReadingHead;
    let mut binding_buffer: Vec<Binding> = Vec::with_capacity(batch_size);

    loop {
        let chunk = wasm_bindgen_futures::JsFuture::from(reader.read()).await?;
        if js_sys::Reflect::get(&chunk, &JsValue::from_str("done"))?
            .as_bool()
            .unwrap_or(false)
        {
            if !binding_buffer.is_empty() {
                callback.call1(
                    &JsValue::NULL,
                    &serde_wasm_bindgen::to_value(&binding_buffer)?,
                )?;
            }
            break;
        }
        let value = Uint8Array::new(&js_sys::Reflect::get(&chunk, &JsValue::from_str("value"))?);
        let value_string = String::from_utf8(value.to_vec())
            .map_err(|err| JsValue::from_str(&format!("utf8 error: {err}")))?;
        for chr in value_string.chars() {
            buffer.push(chr);
            match (chr, &state) {
                ('}', ScannerState::ReadingHead) => {
                    buffer.push('}');
                    let header: Header = serde_json::from_str(&buffer)
                        .map_err(|err| JsValue::from_str(&format!("JSON parse error: {err}")))?;
                    callback.call1(&JsValue::NULL, &serde_wasm_bindgen::to_value(&header)?)?;
                    state = ScannerState::SearchingBindings;
                }
                ('}', ScannerState::ReadingBinding(1)) => {
                    let binding: Binding = serde_json::from_str(&buffer)
                        .map_err(|err| JsValue::from_str(&format!("JSON parse error: {err}")))?;
                    binding_buffer.push(binding);
                    if binding_buffer.len() == batch_size {
                        callback.call1(
                            &JsValue::NULL,
                            &serde_wasm_bindgen::to_value(&binding_buffer)?,
                        )?;
                        binding_buffer.clear();
                    }
                    state = ScannerState::SearchingBinding;
                }
                ('[', ScannerState::SearchingBindings) => {
                    buffer.clear();
                    state = ScannerState::ReadingBinding(0)
                }
                ('{', ScannerState::SearchingBinding) => {
                    buffer = "{".to_string();
                    state = ScannerState::ReadingBinding(1)
                }
                ('{', ScannerState::ReadingBinding(depth)) => {
                    state = ScannerState::ReadingBinding(depth + 1)
                }
                ('}', ScannerState::ReadingBinding(depth)) => {
                    state = ScannerState::ReadingBinding(depth - 1)
                }
                _ => {}
            }
        }
    }
    Ok(())
}
