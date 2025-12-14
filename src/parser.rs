use serde::{Deserialize, Serialize};

use crate::sparql::{Binding, Header, Meta};

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

    /// Returins the remaining bindings, consuming the parser.
    pub fn flush(self) -> Option<PartialResult> {
        (!self.binding_buffer.is_empty()).then_some(PartialResult::Bindings(self.binding_buffer))
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
    SearchchingMeta,
    ReadingMeta,
    Done,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PartialResult {
    Header(Header),
    Bindings(Vec<Binding>),
    Meta(Meta),
}

impl Parser {
    pub fn read_char(
        &mut self,
        chr: char,
        limit: Option<usize>,
        offset: usize,
    ) -> Result<Option<PartialResult>, serde_json::Error> {
        self.input_buffer.push(chr);
        let current_state = self.scanner_state.clone();
        let mut bindings_counter = 0;
        match (chr, current_state) {
            ('}', ScannerState::ReadingHead) => {
                self.input_buffer.push('}');
                let header: Header = serde_json::from_str(&self.input_buffer)?;
                self.scanner_state = ScannerState::SearchingBindings;
                return Ok(Some(PartialResult::Header(header)));
            }
            ('}', ScannerState::ReadingBinding(1)) => {
                bindings_counter += 1;
                if bindings_counter > offset
                    && limit.is_none_or(|limit| bindings_counter - offset <= (limit))
                {
                    let binding: Binding = serde_json::from_str(&self.input_buffer)?;
                    self.binding_buffer.push(binding);
                    self.scanner_state = ScannerState::SearchingBinding;
                    if self.binding_buffer.len() == self.batch_size {
                        let bindings = std::mem::take(&mut self.binding_buffer);
                        return Ok(Some(PartialResult::Bindings(bindings)));
                    }
                }
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
                self.scanner_state = ScannerState::SearchchingMeta;
            }
            ('{', ScannerState::SearchchingMeta) => {
                self.input_buffer.clear();
                self.input_buffer.push('{');
                self.scanner_state = ScannerState::ReadingMeta;
            }
            ('}', ScannerState::ReadingMeta) => {
                self.scanner_state = ScannerState::Done;
                let meta: Meta = serde_json::from_str(&self.input_buffer)?;
                return Ok(Some(PartialResult::Meta(meta)));
            }
            _ => {}
        };
        Ok(None)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        parser::{Parser, PartialResult},
        sparql::{Binding, Bindings, Head, Header, Meta, RDFValue, SparqlResult},
    };

    #[test]
    fn parser_schema() {
        let input = r#"{"head":{"vars":[]},"results":{"bindings":[{"":{"type":"uri","value":""},"U*":{"type":"uri","value":"*\"","curie":""}}]},"meta":{"query-time-ms":0,"result-size-total":0}}"#;
        let serde_parsed_result: SparqlResult = serde_json::from_str(&input).unwrap();

        let mut parsed_result = SparqlResult {
            head: Head { vars: Vec::new() },
            results: Bindings {
                bindings: Vec::new(),
            },
            meta: Meta {
                query_time_ms: 0,
                result_size_total: 0,
            },
        };

        let mut parser = Parser::new(1);
        for chr in input.chars() {
            match parser
                .read_char(chr, None, 0)
                .expect("Input should be valid")
            {
                Some(PartialResult::Header(Header { head })) => parsed_result.head = head,
                Some(PartialResult::Bindings(bindings)) => {
                    parsed_result.results.bindings.extend(bindings);
                }
                Some(PartialResult::Meta(meta)) => parsed_result.meta = meta,
                None => {}
            }
        }

        assert_eq!(
            serde_parsed_result, parsed_result,
            "parser failed for this input:\n{input}"
        );
    }
}
