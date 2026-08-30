use std::path::Path;

fn main() {
    // Rebuild when embedded frontend assets change. Enumerate files because
    // directory-level `rerun-if-changed` does not catch nested file edits.
    let assets = Path::new("assets");
    if assets.is_dir() {
        for entry in walk(assets) {
            println!("cargo:rerun-if-changed={}", entry.display());
        }
    }
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}
