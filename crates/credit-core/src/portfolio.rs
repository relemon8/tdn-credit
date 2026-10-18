use crate::{model::*, units::*};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum TradeError {
    #[error("bond does not exist")]
    UnknownBond,
    #[error("amount must be positive and a multiple of JPY 10 million")]
    InvalidAmount,
    #[error("insufficient cash")]
    InsufficientCash,
    #[error("insufficient holdings")]
    InsufficientHoldings,
    #[error("bond is no longer tradable")]
    NotTradable,
    #[error("amount is too large")]
    Overflow,
}
pub fn nav(portfolio: &PortfolioState, bonds: &BTreeMap<String, BondState>) -> Money {
    Money(
        portfolio.cash.0
            + portfolio
                .positions
                .iter()
                .map(|(id, p)| p.face.value_at(bonds[id].dirty()).0)
                .sum::<i64>(),
    )
}
pub fn liquidating_nav(portfolio: &PortfolioState, bonds: &BTreeMap<String, BondState>) -> Money {
    Money(
        portfolio.cash.0
            + portfolio
                .positions
                .iter()
                .map(|(id, p)| p.face.value_at(bonds[id].bid()).0)
                .sum::<i64>(),
    )
}
pub fn trade(
    portfolio: &mut PortfolioState,
    bonds: &BTreeMap<String, BondState>,
    turn: u32,
    id: &str,
    side: Side,
    face: FaceAmount,
) -> Result<Trade, TradeError> {
    let bond = bonds.get(id).ok_or(TradeError::UnknownBond)?;
    if face.0 <= 0 || face.0 % 10_000_000 != 0 {
        return Err(TradeError::InvalidAmount);
    }
    if bond.settled {
        return Err(TradeError::NotTradable);
    }
    let price = match side {
        Side::Buy => bond.ask(),
        Side::Sell => bond.bid(),
    };
    let raw = face.0 as f64 * price.value() / 100.0;
    if raw >= i64::MAX as f64 || !raw.is_finite() {
        return Err(TradeError::Overflow);
    }
    let cash_amount = face.value_at(price);
    let mid_value = face.value_at(bond.dirty());
    let cost = Money((cash_amount.0 - mid_value.0).abs());
    match side {
        Side::Buy => {
            if cash_amount.0 > portfolio.cash.0 {
                return Err(TradeError::InsufficientCash);
            }
            let existing = portfolio.positions.get(id);
            let next_face = existing
                .map_or(0, |p| p.face.0)
                .checked_add(face.0)
                .ok_or(TradeError::Overflow)?;
            let next_cost = existing
                .map_or(0, |p| p.cost.0)
                .checked_add(cash_amount.0)
                .ok_or(TradeError::Overflow)?;
            portfolio.positions.insert(
                id.into(),
                Position {
                    face: FaceAmount(next_face),
                    cost: Money(next_cost),
                },
            );
            portfolio.cash.0 -= cash_amount.0;
        }
        Side::Sell => {
            let position = portfolio
                .positions
                .get(id)
                .ok_or(TradeError::InsufficientHoldings)?;
            if position.face.0 < face.0 {
                return Err(TradeError::InsufficientHoldings);
            }
            let basis =
                (position.cost.0 as f64 * face.0 as f64 / position.face.0 as f64).round() as i64;
            let next_cash = portfolio
                .cash
                .0
                .checked_add(cash_amount.0)
                .ok_or(TradeError::Overflow)?;
            let position = portfolio.positions.get_mut(id).unwrap();
            position.face.0 -= face.0;
            position.cost.0 -= basis;
            portfolio.realized.0 += cash_amount.0 - basis;
            portfolio.cash.0 = next_cash;
            if position.face.0 == 0 {
                portfolio.positions.remove(id);
            }
        }
    }
    portfolio.trading_cost.0 += cost.0;
    Ok(Trade {
        turn,
        bond: id.into(),
        side,
        face,
        price,
        cash_amount,
        cost,
    })
}
