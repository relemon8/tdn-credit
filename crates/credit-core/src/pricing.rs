use crate::{model::*, units::*};

/// Contractual cashflows discounted by the interpolated curve plus a z-spread.
/// Semiannual compounding; no extra PD discount (would double count credit).
pub fn dirty_price(
    bond: &BondState,
    month: u32,
    curve: TreasuryCurve,
    spread: BasisPoints,
) -> Price {
    if month >= bond.maturity_month {
        return Price::new(0.0).unwrap();
    }
    let mut price = 0.0;
    let mut payment = bond.maturity_month;
    while payment > month {
        let years = (payment - month) as f64 / 12.0;
        let rate = (curve.at(years).value() + spread.as_rate().value()).max(-0.90);
        let cf = 100.0 * bond.coupon.value() / 2.0
            + if payment == bond.maturity_month {
                100.0
            } else {
                0.0
            };
        price += cf / (1.0 + rate / 2.0).powf(2.0 * years);
        if payment < 6 {
            break;
        }
        payment -= 6;
    }
    Price::new(price).unwrap()
}
pub fn accrued(bond: &BondState, month: u32) -> Price {
    if month >= bond.maturity_month {
        return Price::new(0.0).unwrap();
    }
    let until = (bond.maturity_month - month) % 6;
    let since = if until == 0 { 0 } else { 6 - until };
    Price::new(100.0 * bond.coupon.value() * since as f64 / 12.0).unwrap()
}
pub fn payment(bond: &BondState, month: u32) -> Price {
    let amount = if month <= bond.maturity_month && (bond.maturity_month - month).is_multiple_of(6)
    {
        100.0 * bond.coupon.value() / 2.0
            + if month == bond.maturity_month {
                100.0
            } else {
                0.0
            }
    } else {
        0.0
    };
    Price::new(amount).unwrap()
}
pub fn durations(
    bond: &BondState,
    month: u32,
    curve: TreasuryCurve,
) -> (DurationYears, DurationYears) {
    let p = dirty_price(bond, month, curve, bond.market_spread).value();
    if p <= 0.0 {
        return (
            DurationYears::new(0.0).unwrap(),
            DurationYears::new(0.0).unwrap(),
        );
    }
    let up = BasisPoints::new(1.0).unwrap();
    let down = BasisPoints::new(-1.0).unwrap();
    let rates = -(dirty_price(bond, month, curve.shifted(up), bond.market_spread).value()
        - dirty_price(bond, month, curve.shifted(down), bond.market_spread).value())
        / (2.0 * p * 0.0001);
    let spreads = -(dirty_price(
        bond,
        month,
        curve,
        BasisPoints::new(bond.market_spread.value() + 1.0).unwrap(),
    )
    .value()
        - dirty_price(
            bond,
            month,
            curve,
            BasisPoints::new(bond.market_spread.value() - 1.0).unwrap(),
        )
        .value())
        / (2.0 * p * 0.0001);
    (
        DurationYears::new(rates).unwrap(),
        DurationYears::new(spreads).unwrap(),
    )
}
pub fn yield_to_maturity(bond: &BondState, month: u32, dirty: Price) -> Option<Rate> {
    if month >= bond.maturity_month || dirty.value() <= 0.0 {
        return None;
    }
    let mut low = -0.90;
    let mut high = 10.0;
    for _ in 0..80 {
        let mid = (low + high) / 2.0;
        let flat = TreasuryCurve {
            two: Rate::new(mid).unwrap(),
            five: Rate::new(mid).unwrap(),
            ten: Rate::new(mid).unwrap(),
        };
        if dirty_price(bond, month, flat, BasisPoints::new(0.0).unwrap()).value() > dirty.value() {
            low = mid;
        } else {
            high = mid;
        }
    }
    Some(Rate::new((low + high) / 2.0).unwrap())
}
