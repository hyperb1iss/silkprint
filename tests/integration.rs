//! CLI integration tests for the `silkprint` binary.
//!
//! Uses `assert_cmd` + `predicates` for ergonomic process assertions.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

/// Build a `Command` for the silkprint binary with color disabled.
fn silkprint() -> Command {
    let bin_path = assert_cmd::cargo::cargo_bin!("silkprint");
    let mut cmd = Command::new(bin_path);
    cmd.arg("--color").arg("never");
    cmd
}

// ── Help & Info ──────────────────────────────────────────────────

#[test]
fn test_help() {
    silkprint()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("silkprint"));
}

#[test]
fn test_list_themes() {
    silkprint()
        .arg("--list-themes")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("silk-light").and(predicate::str::contains("40")),
        );
}

// ── Error cases ──────────────────────────────────────────────────

#[test]
fn test_missing_input() {
    // No input file — should fail because all modes except --list-themes require one.
    silkprint()
        .assert()
        .failure();
}

#[test]
fn test_nonexistent_file() {
    silkprint()
        .arg("nonexistent.md")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("not exist")));
}

#[test]
fn test_conflicting_flags() {
    silkprint()
        .arg("--quiet")
        .arg("--verbose")
        .arg("tests/fixtures/basic.md")
        .assert()
        .failure();
}

#[test]
fn test_nonexistent_theme() {
    silkprint()
        .arg("--theme")
        .arg("nonexistent")
        .arg("tests/fixtures/basic.md")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// ── Validation & Inspection modes ────────────────────────────────

#[test]
fn test_check_mode() {
    silkprint()
        .arg("--check")
        .arg("tests/fixtures/basic.md")
        .assert()
        .success();
}

#[test]
fn test_dump_typst() {
    silkprint()
        .arg("--dump-typst")
        .arg("tests/fixtures/basic.md")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("#set").or(predicate::str::contains("page")),
        );
}

// ── Render modes ─────────────────────────────────────────────────

#[test]
fn test_render_to_file() {
    let out = NamedTempFile::new().expect("should create tempfile");
    let out_path = out.path().to_str().expect("tempfile path should be valid UTF-8");

    silkprint()
        .arg("tests/fixtures/basic.md")
        .arg("-o")
        .arg(out_path)
        .assert()
        .success();

    let metadata = std::fs::metadata(out.path()).expect("output file should exist");
    assert!(metadata.len() > 0, "PDF output should be non-empty");
}

#[test]
fn test_render_stdout() {
    let output = silkprint()
        .arg("tests/fixtures/basic.md")
        .arg("-o")
        .arg("-")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert!(!output.is_empty(), "stdout PDF bytes should be non-empty");
    // PDF files start with %PDF
    assert!(
        output.starts_with(b"%PDF"),
        "stdout output should be a valid PDF (starts with %PDF)"
    );
}

#[test]
fn test_theme_flag() {
    silkprint()
        .arg("--theme")
        .arg("silk-light")
        .arg("--check")
        .arg("tests/fixtures/basic.md")
        .assert()
        .success();
}
