use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io;

#[inline]
pub fn nmake(_target: &str) -> Option<Command> {
    // Requires statements since expressions can't have attributes
    #[cfg(target_os = "windows")]
    return cc::windows_registry::find(_target, "nmake.exe");

    #[cfg(not(target_os = "windows"))]
    return None;
}

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
