//! `AGENTS.md` §12: track enough to make identical input + identical
//! version/parameters reproducible. `Provenance` is attached to every
//! `Evidence` (what produced it) and to the top-level `AdductReport`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Where a piece of evidence or a report ultimately comes from.
///
/// `AGENTS.md` §26: predicted evidence must never be presented
/// indistinguishably from experimental evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EvidenceSource {
    Experimental,
    Literature,
    Rule,
    Database,
    Derived,
    Predicted,
    UserProvided,
}

/// Version/parameter/citation metadata for one evaluation step.
///
/// `generated_at` is caller-supplied (a timestamp string), never computed
/// internally — evidence evaluation stays a pure function of its inputs.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Provenance {
    pub software_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_citation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm_version: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub parameters: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
}

impl Provenance {
    /// Provenance for a value computed by Adductra itself (as opposed to
    /// supplied by the user or read from an external database/rule file).
    pub fn derived(algorithm_version: impl Into<String>) -> Self {
        Self {
            software_version: env!("CARGO_PKG_VERSION").to_string(),
            algorithm_version: Some(algorithm_version.into()),
            ..Default::default()
        }
    }

    pub fn with_parameter(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.parameters.insert(key.into(), value.to_string());
        self
    }
}
