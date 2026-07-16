use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rscpp_parser::parse;
use rscpp_sema::analyze;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKind {
    Ok,
    Parse,
    Sema,
}

#[derive(Debug, Clone)]
pub struct FileResult {
    pub path: PathBuf,
    pub kind: FileKind,
    pub message: Option<String>,
    pub span_start: Option<usize>,
    pub span_end: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ErrorGroup {
    pub kind: String,
    pub message: String,
    pub count: usize,
    pub samples: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CorpusReport {
    pub dir: String,
    pub offset: usize,
    pub limit: usize,
    pub processed: usize,
    pub ok: usize,
    pub fail: usize,
    pub groups: Vec<ErrorGroup>,
}

pub fn check_file(path: &Path) -> Result<FileResult, String> {
    let src = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    match parse(&src) {
        Err(e) => Ok(FileResult {
            path: path.to_path_buf(),
            kind: FileKind::Parse,
            message: Some(e.message),
            span_start: Some(e.span.start),
            span_end: Some(e.span.end),
        }),
        Ok(tu) => {
            let sema = analyze(&tu);
            if sema.ok() {
                Ok(FileResult {
                    path: path.to_path_buf(),
                    kind: FileKind::Ok,
                    message: None,
                    span_start: None,
                    span_end: None,
                })
            } else {
                let e = &sema.errors[0];
                Ok(FileResult {
                    path: path.to_path_buf(),
                    kind: FileKind::Sema,
                    message: Some(e.message.clone()),
                    span_start: Some(e.span.start),
                    span_end: Some(e.span.end),
                })
            }
        }
    }
}

pub fn run_corpus(dir: &Path, offset: usize, limit: usize) -> Result<CorpusReport, String> {
    let files = list_cpp_files(dir)?;
    let batch = select_batch(&files, offset, limit);
    let mut ok = 0usize;
    let mut fail = 0usize;
    let mut map: HashMap<(String, String), Vec<String>> = HashMap::new();

    for path in &batch {
        let res = check_file(path)?;
        match res.kind {
            FileKind::Ok => ok += 1,
            FileKind::Parse | FileKind::Sema => {
                fail += 1;
                let kind = match res.kind {
                    FileKind::Parse => "parse".to_string(),
                    FileKind::Sema => "sema".to_string(),
                    FileKind::Ok => unreachable!(),
                };
                let msg = res.message.unwrap_or_default();
                let sample = path.display().to_string();
                map.entry((kind, msg)).or_default().push(sample);
            }
        }
    }

    let mut groups: Vec<ErrorGroup> = map
        .into_iter()
        .map(|((kind, message), samples)| {
            let count = samples.len();
            let samples: Vec<_> = samples.into_iter().take(3).collect();
            ErrorGroup {
                kind,
                message,
                count,
                samples,
            }
        })
        .collect();
    groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.message.cmp(&b.message)));

    Ok(CorpusReport {
        dir: dir.display().to_string(),
        offset,
        limit,
        processed: batch.len(),
        ok,
        fail,
        groups,
    })
}

pub fn list_cpp_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let rd = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for ent in rd {
        let ent = ent.map_err(|e| e.to_string())?;
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) == Some("cpp") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

pub fn select_batch(files: &[PathBuf], offset: usize, limit: usize) -> Vec<PathBuf> {
    files.iter().skip(offset).take(limit).cloned().collect()
}

pub fn format_report(report: &CorpusReport) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "=== corpus: {} files (dir={}, offset={}, limit={}) ===\n",
        report.processed, report.dir, report.offset, report.limit
    ));
    s.push_str(&format!("ok: {}  fail: {}\n\n", report.ok, report.fail));
    s.push_str("Top errors:\n");
    for g in report.groups.iter().take(20) {
        s.push_str(&format!("  {}×  [{}] {}\n", g.count, g.kind, g.message));
        if let Some(sample) = g.samples.first() {
            s.push_str(&format!("         e.g. {sample}\n"));
        }
    }
    s
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

pub fn report_to_json(report: &CorpusReport) -> String {
    let mut groups = String::new();
    for (i, g) in report.groups.iter().enumerate() {
        if i > 0 {
            groups.push(',');
        }
        let samples: Vec<_> = g
            .samples
            .iter()
            .map(|p| format!("\"{}\"", json_escape(p)))
            .collect();
        groups.push_str(&format!(
            "{{\"kind\":\"{}\",\"message\":\"{}\",\"count\":{},\"samples\":[{}]}}",
            json_escape(&g.kind),
            json_escape(&g.message),
            g.count,
            samples.join(",")
        ));
    }
    format!(
        "{{\"run\":{{\"dir\":\"{}\",\"offset\":{},\"limit\":{},\"processed\":{}}},\"summary\":{{\"ok\":{},\"fail\":{}}},\"groups\":[{}]}}",
        json_escape(&report.dir),
        report.offset,
        report.limit,
        report.processed,
        report.ok,
        report.fail,
        groups
    )
}
