use std::fs;
use std::path::{Path, PathBuf};

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
