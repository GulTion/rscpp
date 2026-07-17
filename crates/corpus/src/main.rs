use rscpp_corpus::{format_report, report_to_json, run_corpus};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "Usage: rscpp-corpus [--dir testing] [--limit 50] [--offset 0] [--out report.json]"
        );
        return ExitCode::from(2);
    }

    let mut dir = PathBuf::from("testing");
    let mut limit: usize = 50;
    let mut offset: usize = 0;
    let mut out: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                dir = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default());
                i += 2;
            }
            "--limit" => {
                limit = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(50);
                i += 2;
            }
            "--offset" => {
                offset = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "--out" => {
                out = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {other}");
                return ExitCode::from(2);
            }
        }
    }

    let report = match run_corpus(&dir, offset, limit) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    if report.processed == 0 {
        println!(
            "no files in batch (dir={}, offset={}, limit={})",
            dir.display(),
            offset,
            limit
        );
        return ExitCode::SUCCESS;
    }

    print!("{}", format_report(&report));

    if let Some(path) = out {
        if let Err(e) = fs::write(&path, report_to_json(&report)) {
            eprintln!("failed to write {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("\nwrote {}", path.display());
    }

    ExitCode::SUCCESS
}
