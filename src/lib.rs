pub mod sparql;

use crate::sparql::{Binding, Header};
use js_sys::{Function, Uint8Array};
use std::panic;
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{ReadableStream, ReadableStreamDefaultReader};

#[wasm_bindgen]
pub async fn read(
    stream: ReadableStream,
    batch_size: usize,
    callback: Function,
) -> Result<(), JsValue> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());
    let reader: ReadableStreamDefaultReader = stream.get_reader().unchecked_into();
    let mut parser = Parser {
        input_buffer: String::new(),
        binding_buffer: Vec::with_capacity(batch_size),
        scanner_state: ScannerState::ReadingHead,
        batch_size,
    };

    loop {
        let chunk = wasm_bindgen_futures::JsFuture::from(reader.read()).await?;
        if js_sys::Reflect::get(&chunk, &JsValue::from_str("done"))?
            .as_bool()
            .unwrap_or(false)
        {
            break;
        }
        let value = Uint8Array::new(&js_sys::Reflect::get(&chunk, &JsValue::from_str("value"))?);
        let value_string = String::from_utf8(value.to_vec())
            .map_err(|err| JsValue::from_str(&format!("utf8 error: {err}")))?;
        for chr in value_string.chars() {
            parser.read_char(chr, |v| {
                callback.call1(&JsValue::NULL, &serde_wasm_bindgen::to_value(v)?)?;
                Ok(())
            })?;
        }
    }
    if !parser.binding_buffer.is_empty() {
        callback.call1(
            &JsValue::NULL,
            &serde_wasm_bindgen::to_value(&parser.binding_buffer)?,
        )?;
    }
    Ok(())
}

pub struct Parser {
    scanner_state: ScannerState,
    input_buffer: String,
    binding_buffer: Vec<Binding>,
    batch_size: usize,
}

impl Parser {
    pub fn new(batch_size: usize) -> Self {
        Self {
            scanner_state: ScannerState::ReadingHead,
            input_buffer: String::new(),
            binding_buffer: Vec::with_capacity(batch_size),
            batch_size,
        }
    }
}

#[derive(Debug, Clone)]
enum ScannerState {
    ReadingHead,
    SearchingBindings,
    SearchingBinding,
    ReadingBinding(u8),
    ReadingString(Box<ScannerState>),
    ReadingStringEscaped(Box<ScannerState>),
    Done,
}

impl Parser {
    pub fn read_char<F>(&mut self, chr: char, callback: F) -> Result<(), JsValue>
    where
        F: Fn(&Vec<Binding>) -> Result<(), JsValue>,
    {
        self.input_buffer.push(chr);
        let current_state = self.scanner_state.clone();
        match (chr, current_state) {
            ('}', ScannerState::ReadingHead) => {
                self.input_buffer.push('}');
                let header: Header = serde_json::from_str(&self.input_buffer).unwrap();
                // .map_err(|err| JsValue::from_str(&format!("JSON parse error: {err}")))?;
                // callback(&header);
                // callback.call1(&JsValue::NULL, &serde_wasm_bindgen::to_value(&header)?)?;
                self.scanner_state = ScannerState::SearchingBindings;
            }
            ('}', ScannerState::ReadingBinding(1)) => {
                let binding: Binding = serde_json::from_str(&self.input_buffer).unwrap();
                // .map_err(|err| JsValue::from_str(&format!("JSON parse error: {err}")))?;
                self.binding_buffer.push(binding);
                if self.binding_buffer.len() == self.batch_size {
                    callback(&self.binding_buffer)?;
                    // callback.call1(
                    //     &JsValue::NULL,
                    //     &serde_wasm_bindgen::to_value(&self.binding_buffer)?,
                    // )?;
                    self.binding_buffer.clear();
                }
                self.scanner_state = ScannerState::SearchingBinding;
            }
            ('[', ScannerState::SearchingBindings) => {
                self.input_buffer.clear();
                self.scanner_state = ScannerState::SearchingBinding;
            }
            ('{', ScannerState::SearchingBinding) => {
                self.input_buffer = "{".to_string();
                self.scanner_state = ScannerState::ReadingBinding(1);
            }
            ('{', ScannerState::ReadingBinding(depth)) => {
                self.scanner_state = ScannerState::ReadingBinding(depth + 1);
            }
            ('}', ScannerState::ReadingBinding(depth)) => {
                self.scanner_state = ScannerState::ReadingBinding(depth - 1);
            }
            ('"', ScannerState::ReadingBinding(_) | ScannerState::ReadingHead) => {
                self.scanner_state =
                    ScannerState::ReadingString(Box::new(self.scanner_state.clone()));
            }
            ('"', ScannerState::ReadingString(prev_state)) => {
                self.scanner_state = *prev_state;
            }
            ('\\', ScannerState::ReadingString(prev_state)) => {
                self.scanner_state = ScannerState::ReadingStringEscaped(prev_state);
            }
            (_, ScannerState::ReadingStringEscaped(prev_state)) => {
                self.scanner_state = ScannerState::ReadingString(prev_state);
            }
            (']', ScannerState::SearchingBinding) => {
                self.input_buffer.clear();
                self.scanner_state = ScannerState::Done;
            }
            _ => {}
        };
        Ok(())
    }
}
