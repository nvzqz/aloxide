use std::path::Path;

// Removes `file` when an instance goes out of scope
pub struct RemoveFileHandle<'p> { pub file: &'p Path }

impl Drop for RemoveFileHandle<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.file);
    }
}
