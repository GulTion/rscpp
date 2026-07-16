use rscpp_corpus::{list_cpp_files, select_batch};
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
