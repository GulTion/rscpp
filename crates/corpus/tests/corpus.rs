use rscpp_corpus::{
    check_file, format_report, list_cpp_files, report_to_json, run_corpus, select_batch, FileKind,
};
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn list_cpp_files_sorted() {
    let mut files = list_cpp_files(&fixtures_dir()).unwrap();
    let names: Vec<_> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    assert!(names.iter().any(|n| n == "ok.cpp"));
}

#[test]
fn select_batch_offset_limit() {
    let files = list_cpp_files(&fixtures_dir()).unwrap();
    let batch = select_batch(&files, 0, 2);
    assert_eq!(batch.len(), 2);
    let batch2 = select_batch(&files, 100, 10);
    assert!(batch2.is_empty());
}

#[test]
fn check_file_ok_parse_sema() {
    let dir = fixtures_dir();
    let ok = check_file(&dir.join("ok.cpp")).unwrap();
    assert!(matches!(ok.kind, FileKind::Ok));

    let bad = check_file(&dir.join("parse_bad.cpp")).unwrap();
    assert!(matches!(bad.kind, FileKind::Parse));
    assert!(bad.message.is_some());

    let sema = check_file(&dir.join("sema_bad.cpp")).unwrap();
    assert!(matches!(sema.kind, FileKind::Sema));
    assert!(sema.message.is_some());
}

#[test]
fn run_corpus_ranks_errors() {
    let report = run_corpus(&fixtures_dir(), 0, 50).unwrap();
    assert_eq!(report.processed, 3);
    assert_eq!(report.ok, 1);
    assert_eq!(report.fail, 2);
    assert!(!report.groups.is_empty());
    assert!(report.groups.iter().all(|g| g.count >= 1));
    for w in report.groups.windows(2) {
        assert!(w[0].count >= w[1].count);
    }
}

#[test]
fn format_report_contains_summary() {
    let report = run_corpus(&fixtures_dir(), 0, 50).unwrap();
    let text = format_report(&report);
    assert!(text.contains("ok:"));
    assert!(text.contains("fail:"));
    assert!(text.contains("Top errors"));
}

#[test]
fn report_to_json_round_shape() {
    let report = run_corpus(&fixtures_dir(), 0, 50).unwrap();
    let json = report_to_json(&report);
    assert!(json.contains("\"processed\""));
    assert!(json.contains("\"groups\""));
}
