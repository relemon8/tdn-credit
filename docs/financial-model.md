# Financial model / data v2

This is a fictional game model, not a calibrated forecast or investment advice.
All coefficients are versioned with data v2. Statement figures are JPY 100m;
settlement/portfolio amounts are integer JPY. Rates are decimal fractions, bp
convert by dividing by 10,000, and quoted prices are per 100 face.

## Timing and accounting

Turn 0 is initial allocation. `NextMonth` advances existing positions through
one month, records end-of-month NAV/attribution/index, and presents the market
for the next allocation. Orders between updates consume no market randomness.
Their mid-to-execution costs are reconciled in the next monthly record; current
NAV reflects them immediately. The initial evaluation is at 24 observations; browser play can continue past this milestone.
This command boundary implements the proposed monthly steps with valuation
shown before the next set of orders, preventing retroactive trading profits.

Macro/curve evolution and macro events precede issuer fundamentals and issuer/sector events; after metrics,
rating and default, all tradable prices are recomputed. Intermediate prices are
not offered for execution. No stale pre-news price is tradable.

## Issuer model

Revenue and EBITDA follow GDP, sector cyclicality and business volatility.
FCF is an after-interest annualized TTM flow; one twelfth enters cash each month.
Maturity buckets age monthly under a uniform-within-bucket approximation.
Eligible debt is refinanced into the long bucket; otherwise cash/committed lines
pay maturities. Unfunded obligations produce a payment shortfall and default.
There is no unlimited automatic borrowing to conceal negative FCF.
Net debt is debt minus cash and may be negative. Risk-factor transforms floor
net-cash leverage at zero where appropriate. Funding liquidity coverage is a
conservative stress indicator: (cash + undrawn lines + positive FCF) divided by
(next-year maturities + an additional interest reserve). The extra interest
reserve is a stress cushion, not another deduction from after-interest FCF.

KMR is a non-bank financial-services/leasing company, not a deposit-taking bank.
Using EBITDA credit metrics for a bank would require a different model.

## Credit score

Normalized components (clamped 0–100):

| Component | Initial transform | Weight |
|---|---|---|
| Net leverage | 100 − 12 × leverage | 24% |
| Coverage | 15 + 24 × ln(max(coverage, 0.01)) | 18% |
| Liquidity | 45 × funding liquidity coverage | 17% |
| FCF margin | 50 + 250 × margin | 14% |
| Stability | issuer business-stability input | 12% |
| Macro | 75 / 55 / 35 / 10 by regime | 8% |
| Event | 100 − event risk − 30 × stress | 7% |

Sector peer normalization is added to the weighted score. Offsets in
`credit::sector_score_adjustment` are explicit fictional calibrations; they are
fixed across turns and never force a published rating. These offsets and the
weights are tuning parameters, not empirically estimated credit-agency models.

AAA begins at 90, AA at 80, A at 70, BBB at 60, BB at 50, B at 40, CCC at 30;
three notches divide each non-AAA band. Rating agency requires persistent
evidence beyond a 0.8-point hysteresis margin. Lag is 2–5 months by difficulty.
Public ratings do not mechanically overwrite current internal credit quality.

## Fair spread and market spread

Fair starts from an interpolated score table: 30→850bp, 40→450, 50→250,
60→160, 70→100, 80→60, 90→30. Add leverage, coverage, FCF, funding liquidity,
macro, sector, refinancing, seniority, bond liquidity, term and event premiums.
Healthy metric adjustments do not stack unlimited discounts onto a low base.
Floor/ceiling: 25–2,500bp. This hybrid model intentionally permits credit-risk
premia larger than expected losses; it is not an arbitrage-free hazard model.

Market spreads partially converge to a risk-appetite-adjusted fair target,
plus news/flow/noise. Convergence is slower for illiquid bonds. Efficient
difficulties converge faster. A fair/market difference is not guaranteed alpha.

The player sees an estimate from last-disclosed financials, with an uncertainty
range. Internal model truth remains hidden during continued play, including after the evaluation milestone.
News about a transaction may disclose its own numeric impacts earlier than a
regular statement. Harder settings have slower statement disclosures.

## PD and recovery

Annual base PD is interpolated from internal score (0.005% at 90, 0.03% at 80,
0.1% at 70, 0.4% at 60, 1.5% at 50, 6% at 40, 20% at 30), multiplied by
leverage, coverage, liquidity, regime, refinancing and stress factors. Rookie
receives a support factor. Annual PD is capped at 95% while non-defaulted.
Monthly PD = 1 − (1 − annual PD)^(1/12). It is sampled once per issuer/month.
An actual unfunded payment shortfall is a separate deterministic default path.

Recovery baselines: secured 60%, senior 40%, subordinated 20%, adjusted for
asset quality, forced sale and resolution stress. Each claim is additionally
capped by a common distributable-asset ratio, so aggregate creditors cannot
recover more than the modeled asset pool. This conservative cap is simpler than
a full multi-tranche insolvency waterfall. Recovery is fixed on default and
cash-settled after three months. No coupon is paid in the default month or later.

## Pricing

Semiannual fixed coupons. Dirty price is sum of contractual cashflows discounted
at interpolated Treasury plus market z-spread with semiannual compounding.
Clean = dirty − accrued. Cash settlements use dirty bid/ask. Coupon receipt is
not an additional gain on top of accrued interest. Principal is repaid at maturity.

Curve is linear between 2/5/10Y and flat outside endpoints. This is deliberate;
there is no bootstrapped zero curve, option value, tax, FX or callability.
Do not also PD-discount cashflows: that would double-count credit in this model.
Duration and spread duration are central differences using ±1bp. They are
nearly identical for these option-free parallel-shift cashflows.

## Liquidity and execution

Full bid/ask in price points = (0.15 + 2.4 × (1 − L)^2) × regime × stress.
Regime multipliers are 1 / 1.3 / 1.8 / 4. L is bond liquidity scaled 0–1.
Buy at mid + half spread; sell at max(0, mid − half spread). No size impact,
partial fills or short selling in MVP. Minimum face increment JPY 10m.
NAV is mid; indicative liquidation value is bid. Cash interest is 0%.

## Attribution and index

Sequential counterfactual prices: prior → aged/cashflows → new curve → new
market spread → default/recovery → execution costs. Changes telescope exactly;
cross effects go to the spread leg. Rounded JPY differences remain as explicit
residual, covered by tests. Realized/unrealized is a separate accounting lens.

Carry includes coupon accrual and aging/pull-to-par/roll-down, not merely coupon.
Default loss also removes an otherwise scheduled coupon in the default month.

Index has equal 5% initial bond weights, integer face units held until redemption, retained coupon cash,
same price/default/recovery/cashflow functions, and no turnover/execution cost.
At solvent maturity, the redeemed principal alone is reinvested at mid into a new bond of the same issuer. The new contract has a 3-year (short leg) or 7-year (long leg) tenor and a coupon based on the current curve plus market spread. Default recovery stays in cash. This does not periodically rebalance issuer weights. It is a fictional rolling index, not a real-world commercial index. Excess is return difference, not wealth ratio.

## Limits

No endogenous player price impact, taxes, inflation-adjusted score, CDS,
derivatives, corporate equity simulation or reinvested index coupons. An equity
holder may welcome a debt-funded acquisition; the game does not fabricate an
equity price to claim that this always occurs. Scenario P/L is an instantaneous
DCF shock with additional liquidation costs, excluding future defaults/carry.
Ranking is a transparent game heuristic; 24 monthly Sharpe observations are not
statistically strong evidence of skill. Balance needs further playtesting.

Event cash amounts use a positive earnings scale (absolute EBITDA, with a 2%
revenue floor). Earnings shocks are direction-aware for loss-making issuers;
negative EBITDA must not turn a buyback into cash income or an earnings miss
into improved profit.

## Debt funding costs

Interest expense is an annual debt-service run rate, not a universal mean-
reverting rate. Twenty percent of outstanding debt is modeled as floating and
responds to policy-rate changes. Fixed-rate debt retains its legacy financing
cost until rolled. Actual maturities refinance at the current 5Y Treasury plus
the issuer's lowest observable bond spread, with a 25bp nominal funding floor.
New credit-line drawings pay this market funding rate; repayments remove the
legacy cost of repaid debt. This is distinct from bondholders' contractual
coupons, which do not change merely because market yields moved.

This couples funding markets back into issuer coverage/FCF/default risk. Macro
events are processed before that month's operating/funding update; issuer and
sector disclosures follow. Credit spreads still price forward-looking risk and
published ratings still lag the underlying metrics.

The score-based PD already incorporates leverage, coverage, liquidity and FCF.
Their additional dynamic multipliers are combined with exponent 0.35 to avoid
repeatedly counting correlated deterioration. Thus annualized current-state PD
is base(score) × product(factors)^0.35 × difficulty support, clamped to 0–95%
before default. It is not a frozen twelve-month forecast through changing
regimes. An initial-universe test also checks that near-par fair-spread premia
cover reference PD × loss-given-default before other risk premia.
