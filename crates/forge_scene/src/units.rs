use serde::{Deserialize, Serialize};

/// Unità di lunghezza dei file di origine. La scena interna è sempre in metri.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LengthUnit {
    #[default]
    Millimeter,
    Centimeter,
    Meter,
    Inch,
}

impl LengthUnit {
    pub const ALL: [LengthUnit; 4] = [LengthUnit::Millimeter, LengthUnit::Centimeter, LengthUnit::Meter, LengthUnit::Inch];

    pub fn to_meters(self) -> f32 {
        match self {
            LengthUnit::Millimeter => 0.001,
            LengthUnit::Centimeter => 0.01,
            LengthUnit::Meter => 1.0,
            LengthUnit::Inch => 0.0254,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            LengthUnit::Millimeter => "mm",
            LengthUnit::Centimeter => "cm",
            LengthUnit::Meter => "m",
            LengthUnit::Inch => "pollici",
        }
    }
}
