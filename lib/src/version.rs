//! Ruby versions.

use std::cmp::Ordering;
use std::convert::TryFrom;
use std::ffi::{CStr, OsStr};
use std::fmt;
use std::num::ParseIntError;
use std::process::Command;
use std::str::{FromStr, Utf8Error};

use crate::RubyExecError;

/// A simple Ruby version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    /// `X.y.z`.
    pub major: u16,
    /// `x.Y.z`.
    pub minor: u16,
    /// `x.y.Z`.
    pub teeny: u16,
    /// The pre-release identifier for `self`.
    pub pre: Option<Box<str>>,
}

impl PartialOrd for Version {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }

        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }

        match self.teeny.cmp(&other.teeny) {
            Ordering::Equal => {}
            ord => return ord,
        }

        match (self.pre(), other.pre()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => {
                match (a.starts_with("rc"), b.starts_with("rc")) {
                    (true, true) | (false, false) => a.cmp(b),
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                }
            },
        }
    }
}

impl<S: Into<Box<str>>> From<(u16, u16, u16, S)> for Version {
    #[inline]
    fn from((major, minor, teeny, pre): (u16, u16, u16, S)) -> Self {
        Version { major, minor, teeny, pre: Some(pre.into()) }
    }
}

impl From<(u16, u16, u16)> for Version {
    #[inline]
    fn from((major, minor, teeny): (u16, u16, u16)) -> Self {
        Version { major, minor, teeny, pre: None }
    }
}

impl From<(u16, u16)> for Version {
    #[inline]
    fn from((major, minor): (u16, u16)) -> Self {
        Version { major, minor, teeny: 0, pre: None }
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
        Version { major, minor: 0, teeny: 0, pre: None }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.teeny)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

impl TryFrom<&[u8]> for Version {
    type Error = VersionParseError;

    #[inline]
    fn try_from(b: &[u8]) -> Result<Self, Self::Error> {
        std::str::from_utf8(b)?.parse()
    }
}

impl TryFrom<&CStr> for Version {
    type Error = VersionParseError;

    #[inline]
    fn try_from(s: &CStr) -> Result<Self, Self::Error> {
        s.to_str()?.parse()
    }
}

impl TryFrom<&OsStr> for Version {
    type Error = VersionParseError;

    #[inline]
    fn try_from(s: &OsStr) -> Result<Self, Self::Error> {
        if let Some(s) = s.to_str() {
            s.parse()
        } else {
            Err(VersionParseError::InvalidUnicode)
        }
    }
}

impl TryFrom<&str> for Version {
    type Error = VersionParseError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl FromStr for Version {
    type Err = VersionParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parser().parse(s)
    }
}

impl Version {
    /// Creates a new instance from `major`, `minor`, and `teeny`.
    #[inline]
    pub fn new(major: u16, minor: u16, teeny: u16) -> Self {
        Version { major, minor, teeny, pre: None }
    }

    /// Creates a new instance from `major`, `minor`, `teeny`, and `pre`.
    #[inline]
    pub fn with_pre(
        major: u16,
        minor: u16,
        teeny: u16,
        pre: impl Into<Box<str>>,
    ) -> Self {
        Version { major, minor, teeny, pre: Some(pre.into()) }
    }

    /// Returns the pre-release identifier string for `self`.
    #[inline]
    pub fn pre(&self) -> Option<&str> {
        self.pre.as_ref().map(|s| &**s)
    }

    /// Attempts to get the version of the current Ruby found in `PATH`.
    #[inline]
    pub fn current() -> Result<Self, RubyVersionError> {
        Self::from_bin("ruby")
    }

    /// Attempts to get the version of a `ruby` executable.
    #[inline]
    pub fn from_bin(ruby: impl AsRef<OsStr>) -> Result<Self, RubyVersionError> {
        Self::from_cmd(&mut Command::new(ruby))
    }

    /// Attempts to get the version of `ruby` by executing it.
    #[inline]
    pub fn from_cmd(ruby: &mut Command) -> Result<Self, RubyVersionError> {
        Ok(RubyExecError::process(
            ruby.args(&["-e", "print RbConfig::CONFIG['RUBY_PROGRAM_VERSION']"])
        )?.parse()?)
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
        format!("ruby-{}.tar.bz2", self)
    }

    /// Returns an HTTPS URL for `self`.
    #[inline]
    pub fn url(&self) -> String {
        format!(
            "https://cache.ruby-lang.org/pub/ruby/{major}.{minor}/ruby-{version}.tar.bz2",
            major = self.major,
            minor = self.minor,
            version = self,
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
        use crate::util::memchr;

        fn split_at(s: &str, byte: u8) -> (&str, Option<&str>) {
            if let Some(index) = memchr(byte, s.as_bytes()) {
                (&s[..index], Some(&s[(index + 1)..]))
            } else {
                (s, None)
            }
        }

        let pre = match split_at(s, b'-') {
            (num, Some(pre)) => {
                s = num;
                Some(pre)
            },
            _ => None,
        };

        let major: u16 = match split_at(s, b'.') {
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
                Ok(major) => return Ok(Version {
                    major,
                    minor: 0,
                    teeny: 0,
                    pre: pre.map(|pre| pre.into()),
                }),
                Err(error) => return Err(MajorInt(error)),
            }
        };
        let mut version = Version::from(major);

        match split_at(s, b'.') {
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

        version.pre = pre.map(|pre| pre.into());

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
    /// Could not convert some string-like type into `&str` to continue parsing.
    Utf8(Utf8Error),
    /// Could not convert some type into a `&str` to continue parsing.
    InvalidUnicode,
}

impl From<Utf8Error> for VersionParseError {
    #[inline]
    fn from(error: Utf8Error) -> Self {
        VersionParseError::Utf8(error)
    }
}

/// Failed to get a Ruby version from a `ruby` executable.
#[derive(Debug)]
pub enum RubyVersionError {
    /// Failed to spawn a process for `ruby`.
    Exec(RubyExecError),
    /// Failed to parse the Ruby version as a `Version`.
    Parse(VersionParseError),
}

impl From<RubyExecError> for RubyVersionError {
    #[inline]
    fn from(error: RubyExecError) -> Self {
        RubyVersionError::Exec(error)
    }
}

impl From<VersionParseError> for RubyVersionError {
    #[inline]
    fn from(error: VersionParseError) -> Self {
        RubyVersionError::Parse(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_any() {
        let parser = VersionParser::new();

        let good = [
            (Version::from(1),                "1"),
            (Version::from((1, 0, 0, "rc1")), "1-rc1"),
            (Version::from((1, 0, 0, "rc2")), "1.0-rc2"),
            (Version::from((1, 0, 0)),        "1.0.0"),
            (Version::from((1, 0, 0, "dev")), "1.0.0-dev"),
        ];

        for (version, string) in &good {
            assert_eq!(version, &parser.parse(string).unwrap());
        }

        let bad = [
            "1.0.",
            "1..-dev",
        ];
        for string in &bad {
            parser.parse(string).unwrap_err();
        }
    }

    #[test]
    fn parse_all() {
        let mut parser = VersionParser::new();
        parser.require_all();

        let good = [
            (Version::from((1, 0, 0)),        "1.0.0"),
            (Version::from((1, 0, 0, "dev")), "1.0.0-dev"),
        ];

        for (version, string) in &good {
            assert_eq!(version, &parser.parse(string).unwrap());
        }

        let bad = [
            "1.0",
            "1.0.",
            "1..-dev",
        ];
        for string in &bad {
            parser.parse(string).unwrap_err();
        }
    }

    #[test]
    fn ordering() {
        let versions = [
            Version::with_pre(0, 0, 1, "dev"),
            Version::with_pre(0, 0, 1, "rc1"),
            Version::with_pre(0, 0, 1, "rc2"),
            Version::new(0, 0, 1),
            Version::new(0, 1, 0),
            Version::new(1, 0, 0),
            Version::with_pre(1, 0, 1, "preview1"),
            Version::with_pre(1, 0, 1, "preview2"),
            Version::new(1, 0, 1),
        ];
        for pair in versions.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            assert!(b > a, "{} > {}", b, a);
        }
    }
}
