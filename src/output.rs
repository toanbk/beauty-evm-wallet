// WalletResult struct, JSON serialization, atomic file writes, progress bar

use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use crate::generator::WalletInfo;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WalletResult {
    pub mnemonic: String,
    pub address: String,     // with 0x prefix
    pub private_key: String, // with 0x prefix
    pub found_at: DateTime<Utc>,
}

impl WalletResult {
    pub fn from_wallet_info(info: &WalletInfo) -> Self {
        Self {
            mnemonic: info.mnemonic.clone(),
            address: format!("0x{}", info.address),
            private_key: format!("0x{}", info.private_key),
            found_at: Utc::now(),
        }
    }
}

/// Atomically writes results to JSON file.
/// Strategy: write to .tmp file, then rename.
/// Caller passes the complete results slice; this overwrites the file each time.
pub fn save_results(path: &Path, results: &[WalletResult]) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(results)
        .map_err(std::io::Error::other)?;

    // Write to temp file, then atomic rename
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;

    // Set permissions 0600 on temp file BEFORE rename to avoid exposure window
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))?;
    }

    // Atomic rename (preserves permissions set above)
    fs::rename(&tmp_path, path)?;

    Ok(())
}

pub fn create_progress_bar(verbose: bool) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    if verbose {
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg} | {pos} attempts | {per_sec}")
                .unwrap(),
        );
    } else {
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
    }
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Prints a found wallet to stdout (address only, never private key).
pub fn print_found(result: &WalletResult, index: usize) {
    println!("\n  [{}] Found: {}", index + 1, result.address);
}

/// Prints final summary.
pub fn print_summary(results: &[WalletResult], total_attempts: u64, output_path: &Path) {
    println!("\n--- Results ---");
    println!("Wallets found: {}", results.len());
    println!("Total attempts: {}", total_attempts);
    for (i, r) in results.iter().enumerate() {
        println!("  [{}] {}", i + 1, r.address);
    }
    println!("Saved to: {}", output_path.display());
    println!("WARNING: Output file contains private keys. Keep it secure!");
}
