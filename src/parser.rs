use serde::{Deserialize, Serialize};

use crate::sparql::{Binding, Header};

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
    Done,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PartialResult {
    Header(Header),
    Bindings(Vec<Binding>),
}

impl Parser {
    pub async fn read_char<F>(&mut self, chr: char, callback: F) -> Result<(), serde_json::Error>
    where
        F: AsyncFn(PartialResult) -> (),
    {
        self.input_buffer.push(chr);
        let current_state = self.scanner_state.clone();
        match (chr, current_state) {
            ('}', ScannerState::ReadingHead) => {
                self.input_buffer.push('}');
                let header: Header = serde_json::from_str(&self.input_buffer)?;
                callback(PartialResult::Header(header)).await;
                self.scanner_state = ScannerState::SearchingBindings;
            }
            ('}', ScannerState::ReadingBinding(1)) => {
                let binding: Binding = serde_json::from_str(&self.input_buffer)?;
                self.binding_buffer.push(binding);
                if self.binding_buffer.len() == self.batch_size {
                    let bindings = std::mem::take(&mut self.binding_buffer);
                    callback(PartialResult::Bindings(bindings)).await;
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
