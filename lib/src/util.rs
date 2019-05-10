use std::fs;
use std::path::{Path, PathBuf};
use std::io;

pub fn walk_files<F>(dir: &Path, mut f: F) -> io::Result<()>
    where for<'a> F: FnMut(PathBuf) -> io::Result<()>
{
    _walk_files(dir, &mut f)
}

// Takes `&mut F` to avoid infinite type-level recursion
fn _walk_files<F>(dir: &Path, f: &mut F) -> io::Result<()>
    where for<'a> F: FnMut(PathBuf) -> io::Result<()>
{
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.metadata()?.is_dir() {
            _walk_files(&path, f)?;
        } else {
            f(path)?;
        }
    }
    Ok(())
}
