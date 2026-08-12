//! `AGENTS.md` §5/§7: a candidate DNA adduct under evaluation. Always
//! carries a resolvable formula (derived from SMILES when structure is
//! given); structure itself stays optional (§2: "optional structure
//! hints").

use serde::{Deserialize, Serialize};

use super::provenance::Provenance;
use crate::error::AdductraError;
use crate::mass_table::Formula;

/// `AGENTS.md` §7 P1: extensible nucleobase/nucleoside classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NucleobaseOrigin {
    Adenine,
    Guanine,
    Cytosine,
    Thymine,
    Uracil,
    NucleosideDerived,
    NucleotideDerived,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdductCandidate {
    pub id: String,
    pub name: String,
    /// Hill-notation element-count formula. Always present and
    /// resolvable — derived from `smiles` automatically when the
    /// candidate is constructed from structure.
    pub formula: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub smiles: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nucleobase_origin: Option<NucleobaseOrigin>,
    pub provenance: Provenance,
}

impl AdductCandidate {
    pub fn from_formula(
        id: impl Into<String>,
        name: impl Into<String>,
        formula: impl Into<String>,
        provenance: Provenance,
    ) -> Result<Self, AdductraError> {
        let formula = formula.into();
        Formula::parse(&formula)?; // validate eagerly
        Ok(Self {
            id: id.into(),
            name: name.into(),
            formula,
            smiles: None,
            nucleobase_origin: None,
            provenance,
        })
    }

    pub fn from_smiles(
        id: impl Into<String>,
        name: impl Into<String>,
        smiles: impl Into<String>,
        provenance: Provenance,
    ) -> Result<Self, AdductraError> {
        let smiles = smiles.into();
        let formula = crate::chem_adapter::formula_from_smiles(&smiles)?;
        Ok(Self {
            id: id.into(),
            name: name.into(),
            formula,
            smiles: Some(smiles),
            nucleobase_origin: None,
            provenance,
        })
    }

    pub fn with_nucleobase_origin(mut self, origin: NucleobaseOrigin) -> Self {
        self.nucleobase_origin = Some(origin);
        self
    }

    /// Neutral monoisotopic mass of this candidate, natural isotopic
    /// abundance. `Err` if `formula` was mutated post-construction into
    /// something unparseable.
    pub fn monoisotopic_mass(&self) -> Result<f64, AdductraError> {
        Ok(Formula::parse(&self.formula)?.monoisotopic_mass())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_formula_validates_eagerly() {
        assert!(
            AdductCandidate::from_formula(
                "c1",
                "test",
                "not a formula!!",
                Provenance::derived("test")
            )
            .is_err()
        );
    }

    #[test]
    fn from_smiles_derives_formula() {
        let c = AdductCandidate::from_smiles(
            "c1",
            "guanine",
            "Nc1nc2[nH]cnc2c(=O)[nH]1",
            Provenance::derived("test"),
        )
        .unwrap();
        assert!(c.formula.starts_with("C5H5N5O"), "got {}", c.formula);
        assert!(c.monoisotopic_mass().unwrap() > 150.0);
    }
}
