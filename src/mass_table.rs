//! Adductra-owned monoisotopic mass constants and formula→mass conversion.
//!
//! Deliberately independent of `chematic::chem::exact_mass` — see
//! `chem_adapter.rs` and `ARCHITECTURE.md` for why. Values are NIST
//! atomic mass evaluation monoisotopic (most-abundant-isotope) masses in
//! Daltons (u), restricted to elements that appear in DNA/nucleoside
//! chemistry and common LC-MS ionization adducts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::AdductraError;

/// Mass of a free proton (u). `[M+H]⁺` gains a proton, not a hydrogen
/// atom — using the H-atom mass instead omits the electron and is wrong
/// by ~0.00055 Da (a few ppm at typical DNA-adduct masses).
pub const PROTON_MASS: f64 = 1.007_276_466_879;
/// Mass of an electron (u).
pub const ELECTRON_MASS: f64 = 0.000_548_579_909;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Element {
    H,
    C,
    N,
    O,
    P,
    S,
    Na,
    K,
    Cl,
}

impl Element {
    pub fn symbol(self) -> &'static str {
        match self {
            Element::H => "H",
            Element::C => "C",
            Element::N => "N",
            Element::O => "O",
            Element::P => "P",
            Element::S => "S",
            Element::Na => "Na",
            Element::K => "K",
            Element::Cl => "Cl",
        }
    }

    pub fn from_symbol(symbol: &str) -> Result<Self, AdductraError> {
        match symbol {
            "H" => Ok(Element::H),
            "C" => Ok(Element::C),
            "N" => Ok(Element::N),
            "O" => Ok(Element::O),
            "P" => Ok(Element::P),
            "S" => Ok(Element::S),
            "Na" => Ok(Element::Na),
            "K" => Ok(Element::K),
            "Cl" => Ok(Element::Cl),
            other => Err(AdductraError::UnknownElement(other.to_string())),
        }
    }

    /// Mass of this element's most abundant natural isotope (u).
    pub fn monoisotopic_mass(self) -> f64 {
        match self {
            Element::H => 1.007_825_032_07,
            Element::C => 12.0,
            Element::N => 14.003_074_004_8,
            Element::O => 15.994_914_619_56,
            Element::P => 30.973_761_63,
            Element::S => 31.972_071_00,
            Element::Na => 22.989_769_28,
            Element::K => 38.963_706_49,
            Element::Cl => 34.968_852_68,
        }
    }

    /// The mass number (protons + neutrons) of this element's most
    /// abundant natural isotope, e.g. 12 for carbon.
    pub fn natural_mass_number(self) -> u16 {
        match self {
            Element::H => 1,
            Element::C => 12,
            Element::N => 14,
            Element::O => 16,
            Element::P => 31,
            Element::S => 32,
            Element::Na => 23,
            Element::K => 39,
            Element::Cl => 35,
        }
    }
}

/// Mass of a specific labeled isotope, e.g. `isotope_mass(Element::C, 13)`
/// for ¹³C. Only the isotopes named in `AGENTS.md` §7 P1 (¹³C, ¹⁵N, D,
/// ¹⁸O) plus each element's natural isotope are supported in v0.1; add
/// more as isotope-labeling benchmark cases need them.
pub fn isotope_mass(element: Element, mass_number: u16) -> Result<f64, AdductraError> {
    if mass_number == element.natural_mass_number() {
        return Ok(element.monoisotopic_mass());
    }
    match (element, mass_number) {
        (Element::C, 13) => Ok(13.003_354_835_07),
        (Element::N, 15) => Ok(15.000_108_898_2),
        (Element::H, 2) => Ok(2.014_101_777_85), // deuterium
        (Element::O, 18) => Ok(17.999_161_0),
        _ => Err(AdductraError::UnknownElement(format!(
            "{}-{mass_number}",
            element.symbol()
        ))),
    }
}

/// Mass accuracy error in parts-per-million, the standard mass-spec
/// convention: `(observed - theoretical) / theoretical * 1e6`. Positive
/// means the observed mass is heavier than expected.
///
/// `theoretical` must be `> 0`; callers only ever pass a molecular mass,
/// which is always positive, so this returns a plain `f64` rather than a
/// `Result` — `NaN`/`inf` cannot occur unless `theoretical == 0.0`.
pub fn ppm_error(theoretical: f64, observed: f64) -> f64 {
    (observed - theoretical) / theoretical * 1e6
}

/// A parsed chemical formula (element → atom count), always representing
/// natural isotopic abundance. Isotope labeling is modeled separately as
/// an additive mass shift (see `IsotopeLabel` evidence), not by mutating
/// the formula itself.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Formula(BTreeMap<Element, u32>);

impl Formula {
    pub fn from_counts(counts: BTreeMap<Element, u32>) -> Self {
        Self(counts)
    }

    pub fn parse(formula: &str) -> Result<Self, AdductraError> {
        let raw = crate::chem_adapter::parse_formula_counts(formula)?;
        let mut counts = BTreeMap::new();
        for (symbol, count) in raw {
            counts.insert(Element::from_symbol(&symbol)?, count);
        }
        Ok(Self(counts))
    }

    pub fn count(&self, element: Element) -> u32 {
        self.0.get(&element).copied().unwrap_or(0)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Element, u32)> + '_ {
        self.0.iter().map(|(&e, &c)| (e, c))
    }

    /// Monoisotopic (exact) mass of the neutral molecule, natural
    /// isotopic abundance, in Daltons.
    pub fn monoisotopic_mass(&self) -> f64 {
        self.0
            .iter()
            .map(|(element, &count)| element.monoisotopic_mass() * count as f64)
            .sum()
    }

    pub fn to_hill_string(&self) -> String {
        let mut s = String::new();
        // Hill order: C, then H, then remaining elements alphabetically.
        if let Some(&c) = self.0.get(&Element::C) {
            s.push_str(&format!("C{c}"));
        }
        if let Some(&h) = self.0.get(&Element::H) {
            s.push_str(&format!("H{h}"));
        }
        let mut rest: Vec<_> = self
            .0
            .iter()
            .filter(|&(&e, _)| e != Element::C && e != Element::H)
            .collect();
        rest.sort_by_key(|(e, _)| e.symbol());
        for (element, &count) in rest {
            s.push_str(&format!("{}{count}", element.symbol()));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_isotope_deltas_match_literature() {
        let c12 = Element::C.monoisotopic_mass();
        let c13 = isotope_mass(Element::C, 13).unwrap();
        assert!((c13 - c12 - 1.003_355).abs() < 1e-5);

        // Substituting one 14N for one 15N adds ~1 neutron (~0.997 Da), NOT
        // the ~0.000109 Da mass *defect* of 15N alone relative to integer 15.
        let n14 = Element::N.monoisotopic_mass();
        let n15 = isotope_mass(Element::N, 15).unwrap();
        assert!((n15 - n14 - 0.997_035).abs() < 1e-5);
    }

    #[test]
    fn eight_oxo_dg_base_mass_matches_expected_mh_plus() {
        // 8-oxo-guanine free base, C5H5N5O2, [M+H]+ expected ~168.0511
        let formula = Formula::parse("C5H5N5O2").unwrap();
        let mh_plus = formula.monoisotopic_mass() + PROTON_MASS;
        assert!((mh_plus - 168.0511).abs() < 0.001, "got {mh_plus}");
    }

    #[test]
    fn unsupported_isotope_is_error() {
        assert!(isotope_mass(Element::C, 14).is_err());
    }
}
