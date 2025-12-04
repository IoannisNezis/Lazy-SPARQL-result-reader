#![no_main]
use lazy_sparql_result_reader::{Parser, sparql::SparqlResult};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|result: SparqlResult| {
    let input =
        serde_json::to_string(&result).expect("Arbitrary should create searializable instances");

    let mut parser = Parser::new(2);
    for chr in input.chars() {
        parser
            .read_char(chr, |_| Ok(()))
            .expect("Input should be valid");
    }
});
