pub mod parser;
pub mod sparql;

use crate::parser::{ParsedChunk, Parser};
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
    let mut parser = Parser::new(batch_size);

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
            parser
                .read_char(chr, |v| {
                    callback
                        .call1(
                            &JsValue::NULL,
                            &serde_wasm_bindgen::to_value(v)
                                .expect("Every ParsedChunk should be serialiable"),
                        )
                        .expect("The JS function should not throw an error");
                })
                .map_err(|err| JsValue::from_str(&format!("JSON parse error: {err}")))?;
        }
    }
    if let ParsedChunk::Bindings(bindings) = parser.flush()
        && !bindings.is_empty()
    {
        callback.call1(&JsValue::NULL, &serde_wasm_bindgen::to_value(&bindings)?)?;
    }
    Ok(())
}
