//! The only module allowed to import `chematic::*` directly
//! (`ARCHITECTURE.md`). Used strictly for structure/formula *parsing* —
//! never for mass computation, since `chematic::chem::exact_mass` is
//! unreliable for isotope-labeled atoms (verified 2026-08-12: it uses an
//! atom's isotope mass *number* as its mass in Daltons, giving e.g. a
//! ¹³C delta of 1.000000 instead of 1.003355). Adductra owns mass
//! computation itself in [`crate::mass_table`].

use crate::error::AdductraError;

/// Parse a SMILES string and return its Hill-notation element-count
/// formula (e.g. `"C10H13N5O5"`), via `chematic`'s structure perception.
/// Does not touch mass in any way.
pub fn formula_from_smiles(smiles: &str) -> Result<String, AdductraError> {
    let mol = chematic::smiles::parse(smiles).map_err(|e| AdductraError::InvalidSmiles {
        smiles: smiles.to_string(),
        reason: e.to_string(),
    })?;
    Ok(chematic::chem::calc_mol_formula(&mol))
}

/// Parse a Hill-notation (or bracket/parenthesized) formula string into an
/// element-symbol → atom-count map. Pure string parsing, no mass involved.
pub fn parse_formula_counts(
    formula: &str,
) -> Result<std::collections::HashMap<String, u32>, AdductraError> {
    chematic::chem::parse_formula(formula).map_err(|e| AdductraError::InvalidFormula {
        formula: formula.to_string(),
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_from_smiles_matches_known_compound() {
        // 8-oxo-2'-deoxyguanosine skeleton check: guanine base alone (C5H5N5O)
        let formula = formula_from_smiles("Nc1nc2[nH]cnc2c(=O)[nH]1").unwrap();
        // Hill order: C first, H second, then alphabetical.
        assert!(formula.starts_with("C5H5N5O"), "got {formula}");
    }

    #[test]
    fn parse_formula_counts_basic() {
        let counts = parse_formula_counts("C10H13N5O5").unwrap();
        assert_eq!(counts["C"], 10);
        assert_eq!(counts["H"], 13);
        assert_eq!(counts["N"], 5);
        assert_eq!(counts["O"], 5);
    }

    #[test]
    fn invalid_smiles_is_error_not_panic() {
        assert!(formula_from_smiles("not a smiles (((").is_err());
    }
}
