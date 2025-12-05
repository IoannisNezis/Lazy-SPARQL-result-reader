pub mod parser;
pub mod sparql;

use crate::parser::{ParsedChunk, Parser};
use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{ReadableStream, ReadableStreamDefaultReader};

#[derive(Debug)]
pub enum SparqlResultReaderError {
    CorruptStream,
    Utf8EncodingError,
    JsonParseError(String),
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn read<F: Fn(&ParsedChunk)>(
    stream: ReadableStream,
    batch_size: usize,
    callback: F,
) -> Result<(), SparqlResultReaderError> {
    let reader: ReadableStreamDefaultReader = stream.get_reader().unchecked_into();
    let mut parser = Parser::new(batch_size);

    loop {
        let chunk = wasm_bindgen_futures::JsFuture::from(reader.read())
            .await
            .map_err(|_| SparqlResultReaderError::CorruptStream)?;
        if js_sys::Reflect::get(&chunk, &JsValue::from_str("done"))
            .map_err(|_| SparqlResultReaderError::CorruptStream)?
            .as_bool()
            .unwrap_or(false)
        {
            break;
        }
        let value = Uint8Array::new(
            &js_sys::Reflect::get(&chunk, &JsValue::from_str("value"))
                .map_err(|_| SparqlResultReaderError::CorruptStream)?,
        );
        let value_string = String::from_utf8(value.to_vec())
            .map_err(|_| SparqlResultReaderError::Utf8EncodingError)?;
        for chr in value_string.chars() {
            parser
                .read_char(chr, &callback)
                .map_err(|err| SparqlResultReaderError::JsonParseError(err.to_string()))?;
        }
    }
    if let Some(chunk) = parser.flush() {
        callback(&chunk);
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
use js_sys::Function;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn read(
    stream: ReadableStream,
    batch_size: usize,
    callback: &Function,
) -> Result<(), JsValue> {
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

    if let Some(ParsedChunk::Bindings(bindings)) = parser.flush() {
        callback.call1(&JsValue::NULL, &serde_wasm_bindgen::to_value(&bindings)?)?;
    }
    Ok(())
}
