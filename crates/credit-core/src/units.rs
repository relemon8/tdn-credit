use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
#[error("value is not finite or outside the permitted range")]
pub struct UnitError;

macro_rules! finite_unit {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
        #[serde(try_from = "f64", into = "f64")]
        pub struct $name(f64);
        impl $name {
            pub fn new(value: f64) -> Result<Self, UnitError> {
                if value.is_finite() {
                    Ok(Self(value))
                } else {
                    Err(UnitError)
                }
            }
            pub fn value(self) -> f64 {
                self.0
            }
        }
        impl TryFrom<f64> for $name {
            type Error = UnitError;
            fn try_from(value: f64) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
        impl From<$name> for f64 {
            fn from(value: $name) -> f64 {
                value.0
            }
        }
    };
}
finite_unit!(BasisPoints);
finite_unit!(Rate);
finite_unit!(Price);
finite_unit!(DurationYears);

impl BasisPoints {
    pub fn as_rate(self) -> Rate {
        Rate(self.0 / 10_000.0)
    }
}
impl Rate {
    pub fn from_percent(percent: f64) -> Result<Self, UnitError> {
        Self::new(percent / 100.0)
    }
    pub fn as_bp(self) -> BasisPoints {
        BasisPoints(self.0 * 10_000.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Probability(f64);
impl Probability {
    pub fn new(value: f64) -> Result<Self, UnitError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(UnitError)
        }
    }
    pub fn value(self) -> f64 {
        self.0
    }
    pub fn monthly(self) -> Self {
        Self(1.0 - (1.0 - self.0).powf(1.0 / 12.0))
    }
}
impl TryFrom<f64> for Probability {
    type Error = UnitError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<Probability> for f64 {
    fn from(value: Probability) -> f64 {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money(pub i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaceAmount(pub i64);
impl FaceAmount {
    pub fn value_at(self, price: Price) -> Money {
        Money((self.0 as f64 * price.value() / 100.0).round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_hundred_bp_is_one_percent() {
        assert_eq!(
            BasisPoints::new(100.0).unwrap().as_rate(),
            Rate::from_percent(1.0).unwrap()
        );
    }
    #[test]
    fn monthly_hazard_compounds_back_to_annual() {
        for annual in [0.0, 0.01, 0.2, 0.95, 1.0] {
            let monthly = Probability::new(annual).unwrap().monthly().value();
            assert!((1.0 - (1.0 - monthly).powi(12) - annual).abs() < 1e-12);
        }
    }
}
