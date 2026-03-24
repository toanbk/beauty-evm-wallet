// Integration tests for beauty-wallet CLI tool
// Tests CLI argument handling, JSON output, and file permissions

use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to run CLI with given args and temp output file
fn run_cli_with_args(args: &[&str], temp_dir: &TempDir) -> std::process::Output {
    let output_file = temp_dir.path().join("test-results.json");
    let mut cmd_args = vec!["--output", output_file.to_str().unwrap()];
    cmd_args.extend(args);

    Command::new(env!("CARGO_BIN_EXE_beauty-wallet"))
        .args(&cmd_args)
        .output()
        .expect("Failed to execute beauty-wallet binary")
}

/// Helper to parse JSON output file
fn read_json_output(path: &PathBuf) -> Vec<serde_json::Value> {
    let mut file = fs::File::open(path).expect("Failed to open output file");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read output file");

    serde_json::from_str(&contents).expect("Failed to parse JSON output")
}

#[test]
fn test_cli_finds_short_suffix() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Run CLI with --suffix a to find wallets ending in 'a'
    // This might take a moment but should find at least one with short suffix
    let output = run_cli_with_args(&["--suffix", "a", "--count", "1"], &temp_dir);

    // Check CLI exits successfully
    assert!(output.status.success(),
        "CLI failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr));

    // Verify output file exists
    let output_file = temp_dir.path().join("test-results.json");
    assert!(output_file.exists(), "Output file not created");

    // Verify JSON has at least 1 result
    let results = read_json_output(&output_file);
    assert!(!results.is_empty(), "No results found in JSON output");
    assert_eq!(results.len(), 1, "Expected 1 result, got {}", results.len());

    // Verify first result ends with 'a'
    let address = results[0]["address"].as_str().expect("address field missing");
    assert!(address.ends_with("a"),
        "Address {} does not end with 'a'", address);
}

#[test]
fn test_cli_count_mode() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Run CLI with --suffix a --count 3
    let output = run_cli_with_args(&["--suffix", "a", "--count", "3"], &temp_dir);

    assert!(output.status.success(),
        "CLI failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr));

    let output_file = temp_dir.path().join("test-results.json");
    assert!(output_file.exists(), "Output file not created");

    // Verify JSON has at least 3 results (due to parallel scheduling, may be more)
    let results = read_json_output(&output_file);
    assert!(results.len() >= 3, "Expected at least 3 results, got {}", results.len());

    // Verify all end with 'a'
    for result in &results {
        let address = result["address"].as_str().expect("address field missing");
        assert!(address.ends_with("a"),
            "Address {} does not end with 'a'", address);
    }
}

#[test]
fn test_cli_invalid_suffix_rejected() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Run CLI with invalid hex suffix
    let output = run_cli_with_args(&["--suffix", "xyz"], &temp_dir);

    // Should fail with non-zero exit code
    assert!(!output.status.success(),
        "CLI should reject invalid suffix 'xyz'");

    // Error message should be on stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.to_lowercase().contains("error") || stderr.to_lowercase().contains("invalid"),
        "Expected error message, got: {}", stderr);
}

#[test]
#[cfg(unix)]
fn test_output_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Run CLI with --suffix a to generate output
    let output = run_cli_with_args(&["--suffix", "a", "--count", "1"], &temp_dir);

    assert!(output.status.success(),
        "CLI failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr));

    let output_file = temp_dir.path().join("test-results.json");
    assert!(output_file.exists(), "Output file not created");

    // Verify file permissions are 0o600 (rw-------)
    let metadata = fs::metadata(&output_file).expect("Failed to get file metadata");
    let mode = metadata.permissions().mode();
    let mode_stripped = mode & 0o777;

    assert_eq!(mode_stripped, 0o600,
        "Expected file permissions 0o600, got 0o{:o}", mode_stripped);
}

#[test]
fn test_output_json_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Run CLI to generate output
    let output = run_cli_with_args(&["--suffix", "a", "--count", "1"], &temp_dir);

    assert!(output.status.success(),
        "CLI failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr));

    let output_file = temp_dir.path().join("test-results.json");
    let results = read_json_output(&output_file);

    // Verify each result has required fields
    for result in &results {
        assert!(result["mnemonic"].is_string(), "mnemonic field missing or not string");
        assert!(result["address"].is_string(), "address field missing or not string");
        assert!(result["private_key"].is_string(), "private_key field missing or not string");
        assert!(result["found_at"].is_string(), "found_at field missing or not string");

        // Verify address has 0x prefix
        let address = result["address"].as_str().unwrap();
        assert!(address.starts_with("0x"), "Address should start with 0x prefix");

        // Verify private_key has 0x prefix
        let privkey = result["private_key"].as_str().unwrap();
        assert!(privkey.starts_with("0x"), "Private key should start with 0x prefix");
    }
}

#[test]
fn test_cli_default_count_is_one() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Run CLI without --count flag (default should be 1)
    let output = run_cli_with_args(&["--suffix", "a"], &temp_dir);

    assert!(output.status.success(),
        "CLI failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr));

    let output_file = temp_dir.path().join("test-results.json");
    let results = read_json_output(&output_file);

    assert_eq!(results.len(), 1, "Default count should be 1, got {}", results.len());
}
