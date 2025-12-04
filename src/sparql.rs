use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct SparqlResult {
    head: Head,
    results: Bindings,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
struct Bindings {
    bindings: Vec<Binding>,
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Header {
    head: Head,
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
struct Head {
    vars: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Binding(HashMap<String, RDFValue>);

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
