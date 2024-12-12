use std::str::FromStr;

use serde::Serialize;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Zone {
    #[default]
    Tokyo1,
    Tokyo2,
    Ishikari1,
    Ishikari2,
}

impl std::fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                Zone::Tokyo1 => "tk1a",
                Zone::Tokyo2 => "tk1b",
                Zone::Ishikari1 => "is1a",
                Zone::Ishikari2 => "is1b",
            }
        )
    }
}

impl FromStr for Zone {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tk1a" => Ok(Zone::Tokyo1),
            "tk1b" => Ok(Zone::Tokyo2),
            "is1a" => Ok(Zone::Ishikari1),
            "is1b" => Ok(Zone::Ishikari2),
            _ => Err(format!("cannot create Zone from {s}, use tk1a, tk1b, is1a or is1b instead."))
        }
    }
}

#[derive(Serialize, Default)]
pub enum EDiskConnection {
    #[default]
    Virtio,
    Idle,
}

impl EDiskConnection {
    pub fn as_str(&self) -> &str {
        match &self {
            Self::Virtio => "virtio",
            Self::Idle => "ide",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone() {
        let zone = Zone::Tokyo1;
        assert_eq!(format!("{}", zone), "tk1a");
    }
}
