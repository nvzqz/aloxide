use std::fmt;

/// A simple Ruby version.
#[derive(Clone, Copy, Debug)]
pub struct Version {
    /// `X.y.z`.
    pub major: u16,
    /// `x.Y.z`.
    pub minor: u16,
    /// `x.y.Z`.
    pub teeny: u16,
}

impl From<(u16, u16, u16)> for Version {
    #[inline]
    fn from((major, minor, teeny): (u16, u16, u16)) -> Self {
        Version { major, minor, teeny }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.teeny)
    }
}

impl Version {
    /// Creates a new instance from `major`, `minor`, and `teeny`.
    #[inline]
    pub fn new(major: u16, minor: u16, teeny: u16) -> Self {
        Version { major, minor, teeny }
    }

    /// Returns an HTTPS URL for `self`.
    #[inline]
    pub fn url(&self) -> String {
        format!(
            "https://cache.ruby-lang.org/pub/ruby/{major}.{minor}/ruby-{major}.{minor}.{teeny}.tar.bz2",
            major = self.major,
            minor = self.minor,
            teeny = self.teeny,
        )
    }
}
