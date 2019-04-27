use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{Version, util::RemoveFileHandle};

/// Downloads and unpacks Ruby's source code.
pub struct RubySrcDownloader<'a> {
    version: Version,
    dst_dir: &'a Path,
    src_dir: Option<PathBuf>,
    ignore_src_dir: bool,
    ignore_cache: bool,
    cache: bool,
    cache_dir: Option<&'a Path>,
}

impl<'a> RubySrcDownloader<'a> {
    #[inline]
    pub(crate) fn new(version: Version, dst_dir: &'a Path) -> RubySrcDownloader {
        RubySrcDownloader {
            version,
            dst_dir,
            src_dir: None,
            ignore_src_dir: false,
            ignore_cache: false,
            cache: false,
            cache_dir: None,
        }
    }

    /// Overwrite the sources directory in `dst_dir` if it already exists.
    ///
    /// **Warning:** This will remove the contents of the existing sources
    /// directory. Use carefully!
    #[inline]
    pub fn ignore_src_dir(mut self) -> Self {
        self.ignore_src_dir = true;
        self
    }

    /// Sets the name of the output source directory at `dst_dir`.
    ///
    /// The default is the same as the archive's name sans extension.
    #[inline]
    pub fn src_dir_name(mut self, name: impl AsRef<Path>) -> Self {
        self.src_dir = Some(self.dst_dir.join(name));
        self
    }

    /// Forces the download even if a cached download exists.
    #[inline]
    pub fn ignore_cache(mut self) -> Self {
        self.ignore_cache = true;
        self
    }

    /// Sets whether to cache the downloaded archive in a default directory.
    ///
    /// This also allows for using a previously cached download.
    #[inline]
    pub fn cache(mut self) -> Self {
        self.cache = true;
        self
    }

    /// Sets the directory to use for caching the downloaded archive.
    ///
    /// The default is "aloxide" in the user's cache directory.
    #[inline]
    pub fn cache_dir<P: AsRef<Path> + ?Sized>(mut self, path: &'a P) -> Self {
        self.cache_dir = Some(path.as_ref());
        self.cache()
    }

    /// Downloads and returns the directory containing the Ruby sources.
    ///
    /// If `skip_unpack` is set, the returned path is that of the archive.
    pub fn download(self) -> Result<PathBuf, RubySrcDownloadError> {
        use RubySrcDownloadError::*;

        let archive_name = self.version.archive_name();
        let archive_ext = ".tar.bz2";
        let archive_ext_len = archive_ext.len();
        debug_assert!(archive_name.ends_with(archive_ext));

        let src_dir = if let Some(dir) = self.src_dir {
            dir
        } else {
            // Use substring of `archive_name`
            let src_name_len = archive_name.len() - archive_ext_len;
            let src_name = &archive_name[..src_name_len];
            self.dst_dir.join(src_name)
        };

        if !self.ignore_src_dir && src_dir.exists() {
            // Reuse the existing sources
            return Ok(src_dir);
        }

        let default_cache_dir: PathBuf;
        let cache_dir: Option<&Path> = if self.cache {
            // Use provided directory or default to "aloxide" in system cache
            match self.cache_dir {
                Some(cache_dir) => {
                    Some(cache_dir)
                },
                None => match dirs::cache_dir() {
                    Some(mut dir) => {
                        dir.push("aloxide");
                        default_cache_dir = dir;
                        Some(&default_cache_dir)
                    },
                    None => return Err(MissingCache),
                },
            }
        } else {
            None
        };

        // Use cache directory if `self.cache` is set, or a temporary "aloxide"
        // directory otherwise
        let (archive_path, ignore_existing) = match cache_dir {
            Some(dir) => (dir.join(&archive_name), self.ignore_cache),
            None => {
                let mut dir = env::temp_dir();
                dir.push("aloxide");

                if let Err(error) = fs::create_dir_all(&dir) {
                    return Err(CreateTempDir(error));
                }

                dir.push(&archive_name);
                (dir, true)
            },
        };

        let remove_archive: Option<RemoveFileHandle> = if !self.cache {
            // Clean up archive in temp dir
            Some(RemoveFileHandle { file: &archive_path })
        } else {
            None
        };

        let archive_exists = archive_path.exists();
        if ignore_existing || !archive_exists {
            Self::_download(self.version, &archive_path)?;
        }

        Self::_unpack(&archive_path, &src_dir)?;

        drop(remove_archive);
        Ok(src_dir)
    }

    fn _download(version: Version, archive_path: &Path) -> Result<(), RubySrcDownloadError> {
        unimplemented!("TODO: Download {} to {:?}", version, archive_path);
    }

    fn _unpack(archive_path: &Path, src_dir: &Path) -> Result<(), RubySrcDownloadError> {
        unimplemented!("TODO: Unpack {:?} into {:?}", archive_path, src_dir);
    }
}

/// The error returned when
/// [`RubySrcDownloader::download`](struct.RubySrcDownloader.html#method.download)
/// fails.
#[derive(Debug)]
pub enum RubySrcDownloadError {
    /// No cache directory could be found for the current user.
    MissingCache,
    /// Failed to create a temporary "aloxide" directory.
    CreateTempDir(io::Error),
}
