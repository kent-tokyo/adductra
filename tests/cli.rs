//! Smoke tests for the `adductra` CLI binary (`AGENTS.md` §18). Not
//! exhaustive coverage of `src/bin/adductra.rs` — just enough to catch a
//! broken build/wiring between the binary and the library.

use std::io::Write;
use std::process::Command;

const OBSERVATION_JSON: &str = r#"{
    "id": "obs-8oxodg-1",
    "precursor_mz": 284.0989,
    "charge": 1,
    "ion_adduct": "ProtonAdd",
    "product_ions": [
        {"mz": 168.0516, "intensity": 100.0},
        {"mz": 140.0567, "intensity": 40.0},
        {"mz": 112.0618, "intensity": 15.0}
    ],
    "isotope_labels": []
}"#;

const CANDIDATES_JSON: &str = r#"[
    {
        "id": "8-oxo-dG",
        "name": "8-oxo-2'-deoxyguanosine",
        "formula": "C10H13N5O5",
        "nucleobase_origin": {"Other": "8-oxo-guanine"},
        "provenance": {"software_version": "0.1.0"}
    },
    {
        "id": "adenine-isomer",
        "name": "isomeric adenine-derived decoy",
        "formula": "C10H13N5O5",
        "nucleobase_origin": "Adenine",
        "provenance": {"software_version": "0.1.0"}
    }
]"#;

fn write_temp(name: &str, content: &str) -> std::path::PathBuf {
    let path =
        std::env::temp_dir().join(format!("adductra-cli-test-{}-{name}", std::process::id()));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn rank_orders_correct_candidate_first() {
    let obs_path = write_temp("obs.json", OBSERVATION_JSON);
    let candidates_path = write_temp("candidates.json", CANDIDATES_JSON);

    let output = Command::new(env!("CARGO_BIN_EXE_adductra"))
        .args([
            "rank",
            "--observation",
            obs_path.to_str().unwrap(),
            "--candidates",
            candidates_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines[1].contains("8-oxo-dG"), "{stdout}");
    assert!(lines[2].contains("adenine-isomer"), "{stdout}");

    std::fs::remove_file(obs_path).ok();
    std::fs::remove_file(candidates_path).ok();
}

#[test]
fn explain_json_round_trips_as_valid_explanation() {
    let obs_path = write_temp("obs2.json", OBSERVATION_JSON);
    let candidates_path = write_temp("candidates2.json", CANDIDATES_JSON);

    let output = Command::new(env!("CARGO_BIN_EXE_adductra"))
        .args([
            "explain",
            "--observation",
            obs_path.to_str().unwrap(),
            "--candidates",
            candidates_path.to_str().unwrap(),
            "--candidate-id",
            "8-oxo-dG",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let explanation: adductra::Explanation = serde_json::from_str(&stdout).unwrap();
    assert_eq!(explanation.candidate_id, "8-oxo-dG");
    assert!(!explanation.lines.is_empty());

    std::fs::remove_file(obs_path).ok();
    std::fs::remove_file(candidates_path).ok();
}

#[test]
fn missing_file_errors_cleanly_not_a_panic() {
    let output = Command::new(env!("CARGO_BIN_EXE_adductra"))
        .args([
            "rank",
            "--observation",
            "/nonexistent/does-not-exist.json",
            "--candidates",
            "/nonexistent/also-missing.json",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("error:"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}
