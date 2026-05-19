mod bip32;
mod generator;
mod matcher;
mod output;

use clap::Parser;
use output::WalletResult;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(name = "beauty-wallet")]
#[command(version)]
#[command(about = "Generate Ethereum vanity wallet addresses matching a suffix pattern")]
struct Cli {
    /// Hex suffix to match (e.g., "1988", "8888")
    #[arg(short, long)]
    suffix: String,

    /// Number of matching wallets to find (default: 1)
    #[arg(short, long, default_value_t = 1)]
    count: usize,

    /// Run continuously until Ctrl+C
    #[arg(long, conflicts_with = "count")]
    continuous: bool,

    /// Output JSON file path
    #[arg(short, long, default_value = "beauty-wallet-results.json")]
    output: String,

    /// Show live speed stats
    #[arg(short, long)]
    verbose: bool,

    /// Address indices to check per mnemonic (amortizes PBKDF2 cost)
    #[arg(long, default_value_t = 100)]
    batch_size: u32,
}

struct SharedState {
    attempts: AtomicUsize,
    found_count: AtomicUsize,
    should_stop: AtomicBool,
    results: Mutex<Vec<WalletResult>>,
}

fn main() {
    let cli = Cli::parse();

    // Validate suffix
    let validated_suffix = match matcher::validate_suffix(&cli.suffix) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let continuous = cli.continuous;
    let target_count = if continuous { usize::MAX } else { cli.count };
    let output_path = PathBuf::from(&cli.output);
    let batch_size = cli.batch_size.max(1);

    // Backup existing output file if present
    if output_path.exists() {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let stem = output_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let ext = output_path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let parent = output_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let backup_path = parent.join(format!("{}.{}{}", stem, timestamp, ext));
        if let Err(e) = std::fs::rename(&output_path, &backup_path) {
            eprintln!(
                "Warning: failed to backup {}: {}",
                output_path.display(),
                e
            );
        } else {
            println!("Backed up existing output to: {}", backup_path.display());
        }
    }

    // Startup info
    println!("Beauty Wallet Generator");
    println!("Suffix: {}", validated_suffix);
    println!(
        "Mode: {}",
        if continuous {
            "continuous".to_string()
        } else {
            format!("find {}", target_count)
        }
    );
    println!("Cores: {}", rayon::current_num_threads());
    println!("Batch: {} addresses/mnemonic", batch_size);
    println!("Output: {}", output_path.display());
    println!("---");

    // Shared state
    let state = Arc::new(SharedState {
        attempts: AtomicUsize::new(0),
        found_count: AtomicUsize::new(0),
        should_stop: AtomicBool::new(false),
        results: Mutex::new(Vec::new()),
    });

    // Ctrl+C handler
    let ctrlc_state = state.clone();
    ctrlc::set_handler(move || {
        println!("\nCtrl+C received, shutting down gracefully...");
        ctrlc_state.should_stop.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Pre-compute suffix bytes for fast byte-level matching
    let (suffix_bytes, suffix_odd) = matcher::suffix_to_bytes(&validated_suffix);

    // Progress bar
    let pb = output::create_progress_bar(cli.verbose);
    pb.set_message(format!(
        "Found 0/{}...",
        if continuous {
            "inf".to_string()
        } else {
            target_count.to_string()
        }
    ));

    // Rayon parallel batch generation loop
    use rayon::prelude::*;
    std::iter::from_fn(|| {
        if state.should_stop.load(Ordering::Relaxed) {
            None
        } else {
            Some(())
        }
    })
    .par_bridge()
    .for_each(|_| {
        if state.should_stop.load(Ordering::Relaxed) {
            return;
        }

        // Generate a batch of addresses from one mnemonic (amortizes PBKDF2)
        let batch = match generator::generate_batch(batch_size) {
            Ok(b) => b,
            Err(_) => return,
        };

        state
            .attempts
            .fetch_add(batch.len(), Ordering::Relaxed);
        pb.inc(batch.len() as u64);

        // Check each address in the batch for suffix match
        for raw in &batch {
            if state.should_stop.load(Ordering::Relaxed) {
                return;
            }

            if matcher::matches_suffix_bytes(&raw.address, &suffix_bytes, suffix_odd) {
                if !continuous && state.found_count.load(Ordering::SeqCst) >= target_count {
                    return;
                }

                let wallet_info = raw.to_wallet_info();
                let result = WalletResult::from_wallet_info(&wallet_info);

                {
                    let mut results = state.results.lock().unwrap();
                    results.push(result.clone());
                }

                let found = state.found_count.fetch_add(1, Ordering::SeqCst) + 1;

                output::print_found(&result, found - 1);

                // Incremental save
                {
                    let results = state.results.lock().unwrap();
                    let _ = output::save_results(&output_path, &results);
                }

                // Check if target reached (count mode)
                if !continuous && found >= target_count {
                    state.should_stop.store(true, Ordering::SeqCst);
                }

                pb.set_message(format!(
                    "Found {}/{}",
                    found,
                    if continuous {
                        "inf".to_string()
                    } else {
                        target_count.to_string()
                    }
                ));
            }
        }
    });

    // Final save
    let results = state.results.lock().unwrap();
    if !results.is_empty() {
        let _ = output::save_results(&output_path, &results);
    }

    let total_attempts = state.attempts.load(Ordering::Relaxed) as u64;
    pb.finish_with_message("Done!");
    output::print_summary(&results, total_attempts, &output_path);
}
