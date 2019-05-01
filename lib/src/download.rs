//! Utilities for downloading Ruby.

use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use crate::{Version, util::RemoveFileHandle};

/// Unpacks the contents of `archive` (a `.tar.bz2`) into `dst_dir`.
#[inline]
pub fn unpack(
    archive: impl io::Read,
    dst_dir: impl AsRef<Path>,
) -> io::Result<()> {
    tar::Archive::new(bzip2::read::BzDecoder::new(archive)).unpack(dst_dir)
}

/// Downloads and unpacks Ruby's source code.
pub struct RubySrcDownloader<'a> {
    version: Version,
    dst_dir: &'a Path,
    ignore_existing_dir: bool,
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
            ignore_existing_dir: false,
            ignore_cache: false,
            cache: false,
            cache_dir: None,
        }
    }

    /// Overwrite the sources directory in `dst_dir` if it already exists.
    ///
    /// **Warning:** This will overwrite the contents of the existing sources
    /// directory. Use carefully!
    #[inline]
    pub fn ignore_existing_dir(mut self) -> Self {
        self.ignore_existing_dir = true;
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

        // Use substring of `archive_name`
        let src_name_len = archive_name.len() - archive_ext_len;
        let src_name = &archive_name[..src_name_len];
        let src_dir = self.dst_dir.join(src_name);

        if !self.ignore_existing_dir && src_dir.exists() {
            // Reuse the existing sources
            return Ok(src_dir);
        }

        let new_archive_dir: PathBuf;
        let (archive_dir, ignore_existing): (&Path, bool) = if self.cache {
            // Use provided directory or default to "aloxide" in system cache
            let dir = match self.cache_dir {
                Some(cache_dir) => cache_dir,
                None => match dirs::cache_dir() {
                    Some(mut dir) => {
                        dir.push("aloxide");
                        new_archive_dir = dir;
                        &new_archive_dir
                    },
                    None => return Err(MissingCache),
                },
            };
            (dir, self.ignore_cache)
        } else {
            let mut dir = env::temp_dir();
            dir.push("aloxide");
            new_archive_dir = dir;
            (&new_archive_dir, true)
        };
        fs::create_dir_all(archive_dir).map_err(CreateArchiveDir)?;

        let archive_path = archive_dir.join(&archive_name);

        let remove_archive: Option<RemoveFileHandle> = if !self.cache {
            // Clean up archive in temp dir
            Some(RemoveFileHandle { file: &archive_path })
        } else {
            None
        };

        let archive_exists = archive_path.exists();

        let file = if ignore_existing || !archive_exists {
            Self::_download(self.version, &archive_path)?
        } else {
            File::open(&archive_path).map_err(OpenArchive)?
        };

        Self::_unpack(file, &self.dst_dir)?;

        drop(remove_archive);
        Ok(src_dir)
    }

    fn _download(version: Version, archive_path: &Path) -> Result<File, RubySrcDownloadError> {
        use RubySrcDownloadError::*;

        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(archive_path)
            .map_err(CreateArchive)?;

        let mut response = ureq::get(&version.url()).call().into_reader();
        match io::copy(&mut response, &mut file) {
            Ok(_) => Ok(file),
            Err(error) => Err(RequestArchive(error)),
        }
    }

    fn _unpack(
        file: File,
        dst_dir: &Path,
    ) -> Result<(), RubySrcDownloadError> {
        use RubySrcDownloadError::*;

        fs::create_dir_all(dst_dir).map_err(CreateDstDir)?;
        unpack(file, dst_dir).map_err(UnpackArchive)
    }
}

/// The error returned when
/// [`RubySrcDownloader::download`](struct.RubySrcDownloader.html#method.download)
/// fails.
#[derive(Debug)]
pub enum RubySrcDownloadError {
    /// No cache directory could be found for the current user.
    MissingCache,
    /// Failed to open an existing archive.
    OpenArchive(io::Error),
    /// Failed to create a directory for the archive.
    CreateArchiveDir(io::Error),
    /// Failed to create a file for the archive.
    CreateArchive(io::Error),
    /// Failed to GET the archive.
    RequestArchive(io::Error),
    /// Failed to unpack the `.tar.gz` archive.
    UnpackArchive(io::Error),
    /// Failed to create the destination directory.
    CreateDstDir(io::Error),
}
