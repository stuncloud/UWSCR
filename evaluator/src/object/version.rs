use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Version{major, minor, patch}
    }
    pub fn parse(&self) -> f64 {
        format!("{}.{}{}", self.major, self.minor, self.patch).parse().unwrap_or(0.0)
    }
}

impl FromStr for Version {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = s.split('.').collect::<Vec<&str>>();
        let major = v[0].parse::<u32>()?;
        let minor = v[1].parse::<u32>()?;
        let patch = v[2].parse::<u32>()?;
        Ok(Version{major, minor, patch})
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major &&
        self.minor == other.minor &&
        self.patch == other.patch
    }
}

impl PartialEq<String> for Version {
    fn eq(&self, other: &String) -> bool {
        self.to_string() == *other
    }
}

impl PartialEq<f64> for Version {
    fn eq(&self, other: &f64) -> bool {
        self.parse() == *other
    }
}
