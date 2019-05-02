use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use memchr::memchr;
use tar::{Archive, EntryType, Header};

pub fn unpack(
    mut archive: Archive<&mut dyn io::Read>,
    dst_dir: &Path,
) -> io::Result<()> {
    let entries = archive.entries()?.raw(true);

    // Reuse the same allocation instead of calling `.join()`, which allocates
    // a new path each time
    let mut path_buf_os = OsString::from(dst_dir);

    for entry in entries {
        let mut entry = entry?;
        let header = entry.header();

        let entry_path = entry.path()?;

        let mut path_buf = PathBuf::from(path_buf_os);
        path_buf.push(&entry_path);

        if is_dir(&header) {
            fs::create_dir_all(&path_buf)?;
        } else {
            if let Some(parent) = path_buf.parent() {
                fs::create_dir_all(parent)?;
            }
            entry.unpack(&path_buf)?;
        }

        path_buf_os = path_buf.into_os_string();
        path_buf_os.clear();
        path_buf_os.push(dst_dir);
    }

    Ok(())
}

fn is_dir(header: &Header) -> bool {
    match header.entry_type() {
        // This fixes an issue in some Ruby archives (namely 2.6.0) where some
        // directories are encoded as regular files
        EntryType::Regular => ends_with_slash(&header.as_old().name),
        EntryType::Directory => true,
        _ => false,
    }
}

fn ends_with_slash(name: &[u8; 100]) -> bool {
    if let Some(i) = memchr(0, name) {
        name.get(i - 1) == Some(&b'/')
    } else {
        false
    }
}
