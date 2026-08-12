//! What was actually measured. `AGENTS.md` §2/§7: precursor m/z, product
//! ions, optional isotope labels, optional formula hint.

use serde::{Deserialize, Serialize};

use super::numeric::NonNegativeF64;
use crate::error::AdductraError;
use crate::mass_table::Element;

/// One observed MS/MS product ion (m/z, optionally with intensity).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductIon {
    pub mz: NonNegativeF64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub intensity: Option<NonNegativeF64>,
}

impl ProductIon {
    pub fn new(mz: f64, intensity: Option<f64>) -> Result<Self, AdductraError> {
        let mz = NonNegativeF64::new(mz, "product_ion.mz")?;
        let intensity = intensity
            .map(|i| NonNegativeF64::new(i, "product_ion.intensity"))
            .transpose()?;
        Ok(Self { mz, intensity })
    }
}

/// A class of isotope-labeled atoms in the experiment, e.g. "5 atoms of
/// ¹⁵N". Not atom-mapped to a structural position — v0.1 isotope
/// evidence reasons about label counts and total mass shift only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsotopeLabel {
    pub element: Element,
    pub mass_number: u16,
    pub count: u8,
}

impl IsotopeLabel {
    pub fn new(element: Element, mass_number: u16, count: u8) -> Self {
        Self {
            element,
            mass_number,
            count,
        }
    }

    /// Total mass shift vs. natural isotopic abundance for `count`
    /// labeled atoms. `Err` if `(element, mass_number)` isn't a
    /// recognized isotope (see `mass_table::isotope_mass`).
    pub fn total_shift_da(&self) -> Result<f64, AdductraError> {
        let labeled = crate::mass_table::isotope_mass(self.element, self.mass_number)?;
        let natural = self.element.monoisotopic_mass();
        Ok((labeled - natural) * self.count as f64)
    }
}

/// MS ionization adduct — how the precursor ion formed from the neutral
/// molecule (`[M+H]+`, `[M-H]-`, ...). Deliberately distinct from a "DNA
/// adduct" (the biological candidate) — the landscape survey
/// (`docs/landscape.md` §1) flags this as a real naming collision in the
/// adductomics field, so the type names avoid the word "adduct" alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IonAdductType {
    ProtonAdd,
    ProtonLoss,
    SodiumAdd,
    PotassiumAdd,
    AmmoniumAdd,
    Custom { label: String, mass_shift_da: f64 },
}

impl IonAdductType {
    /// Mass shift applied to the neutral monoisotopic mass to obtain the
    /// ion mass (before dividing by charge), in Daltons. Positive-mode
    /// adducts add mass; `ProtonLoss` (negative mode) subtracts it.
    pub fn mass_shift_da(&self) -> f64 {
        use crate::mass_table::{ELECTRON_MASS, PROTON_MASS};
        match self {
            IonAdductType::ProtonAdd => PROTON_MASS,
            IonAdductType::ProtonLoss => -PROTON_MASS,
            IonAdductType::SodiumAdd => Element::Na.monoisotopic_mass() - ELECTRON_MASS,
            IonAdductType::PotassiumAdd => Element::K.monoisotopic_mass() - ELECTRON_MASS,
            IonAdductType::AmmoniumAdd => {
                Element::N.monoisotopic_mass() + 4.0 * Element::H.monoisotopic_mass()
                    - ELECTRON_MASS
            }
            IonAdductType::Custom { mass_shift_da, .. } => *mass_shift_da,
        }
    }

    pub fn label(&self) -> String {
        match self {
            IonAdductType::ProtonAdd => "[M+H]+".to_string(),
            IonAdductType::ProtonLoss => "[M-H]-".to_string(),
            IonAdductType::SodiumAdd => "[M+Na]+".to_string(),
            IonAdductType::PotassiumAdd => "[M+K]+".to_string(),
            IonAdductType::AmmoniumAdd => "[M+NH4]+".to_string(),
            IonAdductType::Custom { label, .. } => label.clone(),
        }
    }
}

/// What was actually measured: precursor m/z, charge, ionization,
/// optional product ions and isotope labels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub precursor_mz: NonNegativeF64,
    pub charge: i8,
    pub ion_adduct: IonAdductType,
    #[serde(default)]
    pub product_ions: Vec<ProductIon>,
    #[serde(default)]
    pub isotope_labels: Vec<IsotopeLabel>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub formula_hint: Option<String>,
}

impl Observation {
    /// Convenience constructor validating `precursor_mz` and rejecting
    /// zero charge up front. Not the only way to build an `Observation`
    /// (fields are public for deserialization) — evaluators must still
    /// treat `charge == 0` defensively (`AGENTS.md` §17).
    pub fn new(
        id: impl Into<String>,
        precursor_mz: f64,
        charge: i8,
        ion_adduct: IonAdductType,
    ) -> Result<Self, AdductraError> {
        if charge == 0 {
            return Err(AdductraError::InvalidCharge(charge));
        }
        let precursor_mz = NonNegativeF64::new(precursor_mz, "observation.precursor_mz")?;
        Ok(Self {
            id: id.into(),
            precursor_mz,
            charge,
            ion_adduct,
            product_ions: Vec::new(),
            isotope_labels: Vec::new(),
            formula_hint: None,
        })
    }

    pub fn with_product_ions(mut self, ions: Vec<ProductIon>) -> Self {
        self.product_ions = ions;
        self
    }

    pub fn with_isotope_labels(mut self, labels: Vec<IsotopeLabel>) -> Self {
        self.isotope_labels = labels;
        self
    }

    pub fn with_formula_hint(mut self, formula: impl Into<String>) -> Self {
        self.formula_hint = Some(formula.into());
        self
    }

    /// Neutral monoisotopic mass implied by this observation's precursor
    /// m/z, charge, and ion adduct — the ionization "undone". Assumes
    /// homogeneous adduct stacking for `|charge| > 1` (e.g. `[M+2H]2+`
    /// carries two protons), the standard mass-spec convention. `Err` if
    /// `charge == 0`. Shared by `MassEvidenceEvaluator` and
    /// `IsotopeEvidenceEvaluator` so the two never drift apart.
    pub fn observed_neutral_mass(&self) -> Result<f64, AdductraError> {
        if self.charge == 0 {
            return Err(AdductraError::InvalidCharge(0));
        }
        let z = self.charge.unsigned_abs() as f64;
        let total_ion_shift = self.ion_adduct.mass_shift_da() * z;
        Ok(self.precursor_mz.get() * z - total_ion_shift)
    }

    /// Validates every isotope label's `count` against how many atoms of
    /// that element `formula` actually has, then sums the expected mass
    /// shift across all labels (0.0 if there are none). `Err` if any
    /// label requests more labeled atoms than the formula has (§17:
    /// "impossible isotope count" must be rejected explicitly, not
    /// silently accepted).
    pub fn total_isotope_shift_da(
        &self,
        formula: &crate::mass_table::Formula,
    ) -> Result<f64, AdductraError> {
        let mut total = 0.0;
        for label in &self.isotope_labels {
            let available = formula.count(label.element);
            if label.count as u32 > available {
                return Err(AdductraError::ImpossibleIsotopeCount {
                    element: label.element.symbol().to_string(),
                    requested: label.count,
                    available,
                });
            }
            total += label.total_shift_da()?;
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_charge_rejected() {
        assert!(Observation::new("obs1", 168.05, 0, IonAdductType::ProtonAdd).is_err());
    }

    #[test]
    fn isotope_shift_matches_known_delta() {
        // 5 atoms of 15N (e.g. a 15N5-labeled internal standard) each add
        // ~0.997 Da (one neutron), for a total shift of ~4.985 Da.
        let label = IsotopeLabel::new(Element::N, 15, 5);
        let shift = label.total_shift_da().unwrap();
        assert!((shift - 4.985_175).abs() < 1e-4, "got {shift}");
    }

    #[test]
    fn metal_and_ammonium_adducts_subtract_electron_mass() {
        // Independently-computed expected values (NIST monoisotopic
        // masses minus one electron), not derived via the same formula
        // `mass_shift_da()` uses internally -- a test that mirrors the
        // implementation wouldn't catch an implementation bug.
        assert!(
            (IonAdductType::SodiumAdd.mass_shift_da() - 22.989_220).abs() < 1e-5,
            "got {}",
            IonAdductType::SodiumAdd.mass_shift_da()
        );
        assert_eq!(IonAdductType::SodiumAdd.label(), "[M+Na]+");

        assert!(
            (IonAdductType::PotassiumAdd.mass_shift_da() - 38.963_158).abs() < 1e-5,
            "got {}",
            IonAdductType::PotassiumAdd.mass_shift_da()
        );
        assert_eq!(IonAdductType::PotassiumAdd.label(), "[M+K]+");

        assert!(
            (IonAdductType::AmmoniumAdd.mass_shift_da() - 18.033_826).abs() < 1e-5,
            "got {}",
            IonAdductType::AmmoniumAdd.mass_shift_da()
        );
        assert_eq!(IonAdductType::AmmoniumAdd.label(), "[M+NH4]+");
    }

    #[test]
    fn custom_ion_adduct_reports_its_own_shift_and_label() {
        let adduct = IonAdductType::Custom {
            label: "[M+CH3OH+H]+".to_string(),
            mass_shift_da: 33.034,
        };
        assert_eq!(adduct.mass_shift_da(), 33.034);
        assert_eq!(adduct.label(), "[M+CH3OH+H]+");
    }

    #[test]
    fn custom_ion_adduct_neutral_mass_uses_its_own_shift() {
        // A custom positive-mode adduct: neutral mass should back out to
        // (precursor_mz * z) - mass_shift_da, same as the built-in variants.
        let obs = Observation::new(
            "obs1",
            150.0,
            1,
            IonAdductType::Custom {
                label: "[M+X]+".to_string(),
                mass_shift_da: 10.0,
            },
        )
        .unwrap();
        assert_eq!(obs.observed_neutral_mass().unwrap(), 140.0);
    }
}
