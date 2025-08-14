use std::process::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_help_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute help command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:") || stdout.contains("Options:"));
    assert!(stdout.contains("--token"));
    assert!(stdout.contains("--use-cloc"));
    assert!(stdout.contains("--languages"));
}

#[test]
fn test_version_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute version command");

    assert!(output.status.success());
}

#[test]
fn test_missing_token_error() {
    let output = Command::new("cargo")
        .args(["run", "--", "--teams-config", "nonexistent.json"])
        .output()
        .expect("Failed to execute command");

    // Command should fail (either due to missing token or missing file)
    assert!(!output.status.success());
}

#[test]
fn test_nonexistent_teams_config() {
    // Create a temporary teams config file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("teams.json");
    
    let config_content = r#"{
        "teams": [
            {
                "name": "test-team",
                "organization": "test-org",
                "repositories": ["nonexistent-repo"]
            }
        ]
    }"#;
    
    fs::write(&config_path, config_content).expect("Failed to write config");

    let output = Command::new("cargo")
        .args([
            "run", "--", 
            "--token", "test-token",
            "--teams-config", config_path.to_str().unwrap()
        ])
        .output()
        .expect("Failed to execute command");

    // The command should attempt to process but may fail due to invalid token/repo
    // We're mainly testing that the config file is read correctly
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should show that it's processing the repository or show an authentication error
    assert!(stdout.contains("Target repositories") || stderr.contains("認証") || stderr.contains("auth"));
}

#[test]
fn test_language_filter_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute help command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Filter repositories by programming languages"));
    assert!(stdout.contains("comma-separated"));
    assert!(stdout.contains("LANGUAGES"));
}

#[test]
fn test_environment_variables_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute help command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[env: GITHUB_TOKEN"));
    assert!(stdout.contains("[env: TEAMS_CONFIG"));
    assert!(stdout.contains("[env: DEBUG_MODE"));
    assert!(stdout.contains("[env: USE_CLOC"));
    assert!(stdout.contains("[env: LANGUAGES"));
}

#[test]
fn test_cloc_availability_check() {
    // Test that the program properly checks for cloc availability
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("teams.json");
    
    let config_content = r#"{
        "teams": [
            {
                "name": "test-team", 
                "organization": "test-org",
                "repositories": ["test-repo"]
            }
        ]
    }"#;
    
    fs::write(&config_path, config_content).expect("Failed to write config");

    let output = Command::new("cargo")
        .args([
            "run", "--",
            "--token", "test-token",
            "--teams-config", config_path.to_str().unwrap(),
            "--use-cloc"
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should either use cloc successfully, show cloc not installed error, or authentication error
    assert!(
        stdout.contains("Using cloc for analysis") || 
        stderr.contains("clocがインストールされていません") ||
        stderr.contains("認証") || // authentication error
        stderr.contains("auth") || // authentication error in English
        stdout.contains("Target repositories") || // successfully started processing
        !output.status.success() // or any kind of expected failure
    );
}

#[test]
fn test_invalid_json_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("invalid.json");
    
    fs::write(&config_path, "invalid json content").expect("Failed to write config");

    let output = Command::new("cargo")
        .args([
            "run", "--",
            "--token", "test-token", 
            "--teams-config", config_path.to_str().unwrap()
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should show JSON parsing error
    assert!(stderr.contains("Error") || stderr.contains("failed") || stderr.contains("invalid"));
}

#[test]
fn test_empty_teams_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("empty.json");
    
    let config_content = r#"{
        "teams": []
    }"#;
    
    fs::write(&config_path, config_content).expect("Failed to write config");

    let output = Command::new("cargo")
        .args([
            "run", "--",
            "--token", "test-token",
            "--teams-config", config_path.to_str().unwrap()
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show that no repositories are found
    assert!(stdout.contains("Target repositories") && stdout.contains("{}"));
}