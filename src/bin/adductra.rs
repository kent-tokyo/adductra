//! `AGENTS.md` §18: a thin CLI wrapper over the library API.
//!
//! ponytail: input is plain JSON (`Observation` / `Vec<AdductCandidate>`,
//! Adductra's own serde types), not the `.mgf` spectrum file §18's
//! example shows — parsing vendor/community spectrum formats is
//! deliberately out of scope (`AGENTS.md` §3: "raw vendor format
//! parser"). Add an MGF reader (or reuse `mzdata`, see
//! `docs/landscape.md` §4) when a real workflow needs it; this CLI stays
//! a wrapper either way.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::process::ExitCode;

use adductra::{
    AdductCandidate, CandidateAssessment, EvidenceEvaluator, EvidenceSet,
    FragmentEvidenceEvaluator, IsotopeEvidenceEvaluator, MassEvidenceEvaluator, Observation,
    Ranker, ReferenceSpectrum, SpectralLibraryEvidenceEvaluator, explain,
};

const USAGE: &str = "\
adductra - evidence-first DNA adduct candidate ranking (research tool)

Adductra is a research tool. It does not diagnose cancer or establish
causal exposure.

USAGE:
    adductra rank    --observation <file.json> --candidates <file.json>
                      [--tolerance-ppm <ppm>] [--isotope-tolerance-da <da>]
                      [--reference-spectra <file.json>]
                      [--spectral-mz-tolerance-da <da>]
                      [--spectral-similarity-threshold <t>]
    adductra explain --observation <file.json> --candidates <file.json>
                      --candidate-id <id> [--json]
                      [--tolerance-ppm <ppm>] [--isotope-tolerance-da <da>]
                      [--reference-spectra <file.json>]
                      [--spectral-mz-tolerance-da <da>]
                      [--spectral-similarity-threshold <t>]

--observation is a single JSON Observation object.
--candidates is a JSON array of AdductCandidate objects.
--tolerance-ppm defaults to 10.0, --isotope-tolerance-da to 0.005.
--reference-spectra is an optional JSON array of ReferenceSpectrum; when
given, spectral-library-match evidence is added for candidates with a
matching entry. --spectral-mz-tolerance-da defaults to 0.01,
--spectral-similarity-threshold to 0.7 (must be in (0.5, 1.0)).
";

struct Args {
    observation: String,
    candidates: String,
    tolerance_ppm: f64,
    isotope_tolerance_da: f64,
    candidate_id: Option<String>,
    json: bool,
    reference_spectra: Option<String>,
    spectral_mz_tolerance_da: f64,
    spectral_similarity_threshold: f64,
}

fn next_value(args: &[String], i: &mut usize) -> Result<String, String> {
    let value = args
        .get(*i + 1)
        .ok_or_else(|| format!("{} requires a value", args[*i]))?
        .clone();
    *i += 2;
    Ok(value)
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut observation = None;
    let mut candidates = None;
    let mut tolerance_ppm = 10.0;
    let mut isotope_tolerance_da = 0.005;
    let mut candidate_id = None;
    let mut json = false;
    let mut reference_spectra = None;
    let mut spectral_mz_tolerance_da = 0.01;
    let mut spectral_similarity_threshold = 0.7;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--observation" => observation = Some(next_value(args, &mut i)?),
            "--candidates" => candidates = Some(next_value(args, &mut i)?),
            "--tolerance-ppm" => {
                tolerance_ppm = next_value(args, &mut i)?
                    .parse()
                    .map_err(|_| "invalid --tolerance-ppm".to_string())?;
            }
            "--isotope-tolerance-da" => {
                isotope_tolerance_da = next_value(args, &mut i)?
                    .parse()
                    .map_err(|_| "invalid --isotope-tolerance-da".to_string())?;
            }
            "--candidate-id" => candidate_id = Some(next_value(args, &mut i)?),
            "--json" => {
                json = true;
                i += 1;
            }
            "--reference-spectra" => reference_spectra = Some(next_value(args, &mut i)?),
            "--spectral-mz-tolerance-da" => {
                spectral_mz_tolerance_da = next_value(args, &mut i)?
                    .parse()
                    .map_err(|_| "invalid --spectral-mz-tolerance-da".to_string())?;
            }
            "--spectral-similarity-threshold" => {
                spectral_similarity_threshold = next_value(args, &mut i)?
                    .parse()
                    .map_err(|_| "invalid --spectral-similarity-threshold".to_string())?;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(Args {
        observation: observation.ok_or("--observation is required")?,
        candidates: candidates.ok_or("--candidates is required")?,
        tolerance_ppm,
        isotope_tolerance_da,
        candidate_id,
        json,
        reference_spectra,
        spectral_mz_tolerance_da,
        spectral_similarity_threshold,
    })
}

fn load_observation(path: &str) -> Result<Observation, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("parsing {path}: {e}"))
}

fn load_candidates(path: &str) -> Result<Vec<AdductCandidate>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("parsing {path}: {e}"))
}

fn load_reference_spectra(path: &str) -> Result<Vec<ReferenceSpectrum>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("parsing {path}: {e}"))
}

fn build_spectral_evaluator(
    args: &Args,
) -> Result<Option<SpectralLibraryEvidenceEvaluator>, String> {
    args.reference_spectra
        .as_deref()
        .map(|path| {
            let reference_spectra = load_reference_spectra(path)?;
            SpectralLibraryEvidenceEvaluator::new(
                reference_spectra,
                args.spectral_mz_tolerance_da,
                args.spectral_similarity_threshold,
            )
            .map_err(|e| e.to_string())
        })
        .transpose()
}

fn assess_all(
    observation: &Observation,
    candidates: &[AdductCandidate],
    tolerance_ppm: f64,
    isotope_tolerance_da: f64,
    spectral: Option<&SpectralLibraryEvidenceEvaluator>,
) -> Result<Vec<CandidateAssessment>, String> {
    let mass_evaluator = MassEvidenceEvaluator::new(tolerance_ppm).map_err(|e| e.to_string())?;
    let fragment_evaluator =
        FragmentEvidenceEvaluator::with_built_in_rules().map_err(|e| e.to_string())?;
    let isotope_evaluator =
        IsotopeEvidenceEvaluator::new(isotope_tolerance_da).map_err(|e| e.to_string())?;

    candidates
        .iter()
        .map(|candidate| {
            let mut evidence = mass_evaluator
                .evaluate(observation, candidate)
                .map_err(|e| format!("candidate {}: {e}", candidate.id))?;
            evidence.extend(
                fragment_evaluator
                    .evaluate(observation, candidate)
                    .map_err(|e| format!("candidate {}: {e}", candidate.id))?,
            );
            evidence.extend(
                isotope_evaluator
                    .evaluate(observation, candidate)
                    .map_err(|e| format!("candidate {}: {e}", candidate.id))?,
            );
            if let Some(spectral_evaluator) = spectral {
                evidence.extend(
                    spectral_evaluator
                        .evaluate(observation, candidate)
                        .map_err(|e| format!("candidate {}: {e}", candidate.id))?,
                );
            }
            Ok(CandidateAssessment::new(
                candidate.id.clone(),
                EvidenceSet::new(evidence),
            ))
        })
        .collect()
}

fn cmd_rank(args: &Args) -> Result<(), String> {
    let observation = load_observation(&args.observation)?;
    let candidates = load_candidates(&args.candidates)?;
    let spectral_evaluator = build_spectral_evaluator(args)?;
    let assessments = assess_all(
        &observation,
        &candidates,
        args.tolerance_ppm,
        args.isotope_tolerance_da,
        spectral_evaluator.as_ref(),
    )?;
    let ranked = Ranker::new().rank(assessments);

    println!("{:<5} {:<30} {:>12}", "Rank", "Candidate", "Score");
    for (i, a) in ranked.iter().enumerate() {
        println!(
            "{:<5} {:<30} {:>12.3}",
            i + 1,
            a.candidate_id,
            a.ranking_score.unwrap_or(f64::NAN)
        );
    }
    Ok(())
}

fn cmd_explain(args: &Args) -> Result<(), String> {
    let observation = load_observation(&args.observation)?;
    let candidates = load_candidates(&args.candidates)?;
    let candidate_id = args
        .candidate_id
        .clone()
        .ok_or_else(|| "explain requires --candidate-id".to_string())?;
    let spectral_evaluator = build_spectral_evaluator(args)?;
    let assessments = assess_all(
        &observation,
        &candidates,
        args.tolerance_ppm,
        args.isotope_tolerance_da,
        spectral_evaluator.as_ref(),
    )?;
    let ranked = Ranker::new().rank(assessments);
    let assessment = ranked
        .iter()
        .find(|a| a.candidate_id == candidate_id)
        .ok_or_else(|| format!("no candidate with id {candidate_id:?}"))?;
    let explanation = explain(assessment);

    if args.json {
        let json = serde_json::to_string_pretty(&explanation).map_err(|e| format!("{e}"))?;
        println!("{json}");
    } else {
        print!("{}", explanation.to_text());
    }
    Ok(())
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = raw.first() else {
        eprint!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let rest = &raw[1..];

    let result = match command.as_str() {
        "rank" => parse_args(rest).and_then(|a| cmd_rank(&a)),
        "explain" => parse_args(rest).and_then(|a| cmd_explain(&a)),
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        other => Err(format!("unknown command: {other}\n\n{USAGE}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
