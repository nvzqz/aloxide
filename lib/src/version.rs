//! Ruby versions.

use std::convert::TryFrom;
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

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

impl From<(u16, u16)> for Version {
    #[inline]
    fn from((major, minor): (u16, u16)) -> Self {
        Version { major, minor, teeny: 0 }
    }
}

impl From<(u16,)> for Version {
    #[inline]
    fn from((major,): (u16,)) -> Self {
        major.into()
    }
}

impl From<u16> for Version {
    #[inline]
    fn from(major: u16) -> Version {
        Version { major, minor: 0, teeny: 0 }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.teeny)
    }
}

impl TryFrom<&str> for Version {
    type Error = VersionParseError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parser().parse(s)
    }
}

impl FromStr for Version {
    type Err = VersionParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl Version {
    /// Creates a new instance from `major`, `minor`, and `teeny`.
    #[inline]
    pub fn new(major: u16, minor: u16, teeny: u16) -> Self {
        Version { major, minor, teeny }
    }

    /// Returns a parser that can be used to construct a `Version` out of a
    /// string through various configurations.
    #[inline]
    pub fn parser() -> VersionParser {
        VersionParser::default()
    }

    /// Returns the name of the archive file corresponding to `self`.
    #[inline]
    pub fn archive_name(&self) -> String {
        format!(
            "ruby-{major}.{minor}.{teeny}.tar.bz2",
            major = self.major,
            minor = self.minor,
            teeny = self.teeny,
        )
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

/// A `Version` parser that be configured to varying levels of strictness.
#[derive(Clone, Copy, Debug, Default)]
pub struct VersionParser {
    require_minor: bool,
    require_teeny: bool,
}

impl VersionParser {
    /// Creates a new instance with optional minor and teeny versions.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `self` to require `x.y`.
    #[inline]
    pub fn require_minor(&mut self) -> &mut Self {
        self.require_minor = true;
        self
    }

    /// Sets `self` to require `x.y.z`.
    #[inline]
    pub fn require_all(&mut self) -> &mut Self {
        self.require_minor = true;
        self.require_teeny = true;
        self
    }

    /// Convert `s` into a `Version` based on the rules defined on `self`.
    pub fn parse(&self, mut s: &str) -> Result<Version, VersionParseError> {
        use VersionParseError::*;
        use memchr::memchr;

        fn split_at_dot(s: &str) -> (&str, Option<&str>) {
            if let Some(index) = memchr(b'.', s.as_bytes()) {
                (&s[..index], Some(&s[(index + 1)..]))
            } else {
                (s, None)
            }
        }

        let major: u16 = match split_at_dot(s) {
            (_, None) if self.require_minor => {
                return Err(MinorMissing);
            },
            (start, Some(end)) => match start.parse() {
                Ok(major) => {
                    s = end;
                    major
                },
                Err(error) => return Err(MajorInt(error)),
            },
            (remaining, None) => match remaining.parse::<u16>() {
                Ok(major) => {
                    return Ok(major.into());
                },
                Err(error) => return Err(MajorInt(error)),
            }
        };
        let mut version = Version::from(major);

        match split_at_dot(s) {
            (_, None) if self.require_teeny => {
                return Err(TeenyMissing);
            },
            (start, Some(end)) => match start.parse() {
                Ok(minor) => {
                    s = end;
                    version.minor = minor;
                },
                Err(error) => return Err(MinorInt(error)),
            },
            (remaining, None) => match remaining.parse() {
                Ok(minor) => {
                    version.minor = minor;
                },
                Err(error) => return Err(MinorInt(error)),
            }
        }

        match s.parse() {
            Ok(teeny) => version.teeny = teeny,
            Err(error) => return Err(TeenyInt(error)),
        }

        Ok(version)
    }
}

/// The error returned when parsing a string into a `Version` fails.
#[derive(Clone, Debug)]
pub enum VersionParseError {
    /// 'x.Y' missing.
    MinorMissing,
    /// 'x.y.Z' missing.
    TeenyMissing,
    /// Invalid 'X.y.z'.
    MajorInt(ParseIntError),
    /// Invalid 'x.Y.z'.
    MinorInt(ParseIntError),
    /// Invalid 'x.y.Z'.
    TeenyInt(ParseIntError),
}
