use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Dimension {
    Er,
    Hp,
    Ql,
    Na,
    Ab,
    Sr,
    Sat,
}

impl Dimension {
    pub fn all() -> &'static [Dimension] {
        &[
            Dimension::Er,
            Dimension::Hp,
            Dimension::Ql,
            Dimension::Na,
            Dimension::Ab,
            Dimension::Sr,
            Dimension::Sat,
        ]
    }

    pub fn code(self) -> &'static str {
        match self {
            Dimension::Er => "ER",
            Dimension::Hp => "HP",
            Dimension::Ql => "QL",
            Dimension::Na => "NA",
            Dimension::Ab => "AB",
            Dimension::Sr => "SR",
            Dimension::Sat => "SAT",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Dimension::Er => "Emotional Resonance",
            Dimension::Hp => "Hook Potential",
            Dimension::Ql => "Quotable Lines",
            Dimension::Na => "Narrativity",
            Dimension::Ab => "Audience Breadth",
            Dimension::Sr => "Social Resonance",
            Dimension::Sat => "Satire Depth",
        }
    }

    pub fn parse(code: &str) -> Result<Dimension> {
        match code.trim().to_ascii_uppercase().as_str() {
            "ER" => Ok(Dimension::Er),
            "HP" => Ok(Dimension::Hp),
            "QL" => Ok(Dimension::Ql),
            "NA" => Ok(Dimension::Na),
            "AB" => Ok(Dimension::Ab),
            "SR" => Ok(Dimension::Sr),
            "SAT" => Ok(Dimension::Sat),
            other => Err(anyhow!("unknown dimension code: {other}")),
        }
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_seven_dimensions_and_parses_codes() {
        assert_eq!(Dimension::all().len(), 7);
        assert_eq!(Dimension::parse("ER").unwrap(), Dimension::Er);
        assert_eq!(Dimension::parse("hp").unwrap(), Dimension::Hp);
        assert_eq!(Dimension::Er.label(), "Emotional Resonance");
        assert!(Dimension::parse("BAD").is_err());
    }
}
