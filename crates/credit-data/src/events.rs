use credit_core::{events::*, model::*};

pub fn catalog() -> Vec<EventDefinition> {
    use Effect::*;
    let mut list = Vec::new();
    let mut add = |id: &str, scope, headline: &str, effects, severity, condition| {
        list.push(EventDefinition {
            id: id.into(),
            scope,
            condition,
            weight: 10,
            cooldown: 6,
            effects,
            headline: headline.into(),
            severity,
        })
    };
    let m = EventScope::Macro;
    let i = EventScope::Issuer;
    let a = Condition::Any;
    add(
        "hike",
        m,
        "中央銀行、政策引き締め",
        vec![PolicyBp(25.), RatesBp(25.), Appetite(-0.04)],
        2,
        a,
    );
    add(
        "cut",
        m,
        "中央銀行、利下げ",
        vec![PolicyBp(-25.), RatesBp(-25.), Appetite(0.02)],
        2,
        a,
    );
    add(
        "inflation",
        m,
        "Inflation、予想を上回る",
        vec![
            Regime(credit_core::model::Regime::Inflation),
            Inflation(0.055),
            RatesBp(35.),
            MarketBp(15.),
        ],
        2,
        a,
    );
    add(
        "recession",
        m,
        "景気後退入り、利益見通し悪化",
        vec![
            Regime(credit_core::model::Regime::Recession),
            Growth(-0.02),
            RatesBp(-30.),
            MarketBp(45.),
            Appetite(-0.1),
        ],
        3,
        a,
    );
    add(
        "crisis",
        m,
        "信用市場で資金調達ストレス",
        vec![
            Regime(credit_core::model::Regime::CreditCrisis),
            Growth(-0.045),
            MacroLiquidity(-0.25),
            MarketBp(90.),
            Appetite(-0.2),
        ],
        3,
        a,
    );
    add(
        "injection",
        m,
        "中央銀行が流動性供給",
        vec![MacroLiquidity(0.2), MarketBp(-25.), Appetite(0.1)],
        2,
        a,
    );
    add(
        "quality",
        m,
        "安全資産への資金移動",
        vec![RatesBp(-20.), MarketBp(35.), Appetite(-0.08)],
        2,
        a,
    );
    add(
        "soft_landing",
        m,
        "景気は底堅く、Inflationは鈍化",
        vec![
            Regime(credit_core::model::Regime::Goldilocks),
            MarketBp(-20.),
            Appetite(0.08),
        ],
        1,
        a,
    );
    add(
        "growth",
        m,
        "成長率が市場予想を上回る",
        vec![Growth(0.035), RatesBp(15.), MarketBp(-15.), Appetite(0.06)],
        1,
        a,
    );
    add(
        "funding",
        m,
        "短期資金市場で調達費用が上昇",
        vec![MacroLiquidity(-0.15), MarketBp(30.)],
        2,
        a,
    );
    for (id, sector, title, effects) in [
        (
            "oil",
            Sector::Transportation,
            "原油高、運送費用が増加",
            vec![Ebitda(0.85), FcfEbitda(-0.1), MarketBp(20.)],
        ),
        (
            "advertising",
            Sector::Media,
            "広告需要が減速",
            vec![Revenue(0.94), Ebitda(0.85), MarketBp(25.)],
        ),
        (
            "consumer",
            Sector::Consumer,
            "個人消費が減速",
            vec![Revenue(0.95), Ebitda(0.88)],
        ),
        (
            "logistics",
            Sector::Transportation,
            "物流需要が拡大",
            vec![Revenue(1.08), Ebitda(1.12)],
        ),
        (
            "property",
            Sector::RealEstate,
            "不動産価格下落、担保評価を見直し",
            vec![AssetQuality(-0.15), Ebitda(0.95), MarketBp(40.)],
        ),
        (
            "healthcare",
            Sector::Healthcare,
            "医療報酬改定、利益率が低下",
            vec![Ebitda(0.90), FcfEbitda(-0.05)],
        ),
        (
            "tech",
            Sector::Technology,
            "企業のTech投資が拡大",
            vec![Revenue(1.10), Ebitda(1.15)],
        ),
        (
            "utility",
            Sector::Utilities,
            "料金改定の承認が遅延",
            vec![FcfEbitda(-0.12), Ebitda(0.96)],
        ),
        (
            "financial_funding",
            Sector::Financials,
            "金融セクターの調達費用上昇",
            vec![FcfEbitda(-0.1), Stress(0.2), MarketBp(25.)],
        ),
        (
            "capex",
            Sector::DataInfrastructure,
            "データ設備投資が加速",
            vec![Revenue(1.08), FcfEbitda(-0.30), Debt(1.08)],
        ),
    ] {
        add(id, EventScope::Sector(sector), title, effects, 2, a);
    }
    for (id, title, effects, severity, condition) in [
        (
            "beat",
            "決算、利益が予想を上回る",
            vec![Ebitda(1.10), FcfEbitda(0.08)],
            1,
            a,
        ),
        (
            "miss",
            "決算、利益が予想を下回る",
            vec![Ebitda(0.88), FcfEbitda(-0.08)],
            2,
            a,
        ),
        (
            "guidance_cut",
            "業績見通しを引き下げ",
            vec![Ebitda(0.92), MarketBp(20.)],
            2,
            a,
        ),
        (
            "guidance_raise",
            "業績見通しを引き上げ",
            vec![Ebitda(1.08), MarketBp(-10.)],
            1,
            a,
        ),
        (
            "merger",
            "事業統合、統合費用を計上",
            vec![Revenue(1.10), Ebitda(1.05), FcfEbitda(-0.15)],
            2,
            a,
        ),
        (
            "debt_acquisition",
            "大型買収を発表、買収対価は全額Debt調達",
            vec![Acquisition, MarketBp(65.)],
            3,
            Condition::Leveraged,
        ),
        (
            "asset_sale",
            "非中核資産を売却",
            vec![CashEbitda(0.4), Ebitda(0.96), AssetQuality(-0.02)],
            1,
            a,
        ),
        (
            "dividend",
            "増配を発表、Cash流出",
            vec![CashEbitda(-0.35), MarketBp(12.)],
            2,
            a,
        ),
        (
            "buyback",
            "自己株式取得を発表",
            vec![CashEbitda(-0.5), MarketBp(20.)],
            2,
            a,
        ),
        (
            "debt_reduction",
            "余剰CashでDebtを削減",
            vec![DebtReduction],
            1,
            a,
        ),
        (
            "ceo",
            "CEO交代、戦略見直しへ",
            vec![Stress(0.1), MarketBp(8.)],
            1,
            a,
        ),
        (
            "fraud",
            "会計不正の調査を開始",
            vec![
                Stress(1.2),
                AssetQuality(-0.1),
                MarketBp(100.),
                BondLiquidity(-15.),
            ],
            3,
            a,
        ),
        (
            "rating_review",
            "Rating Agencyが見直しを開始",
            vec![Stress(0.15), MarketBp(15.)],
            2,
            Condition::Leveraged,
        ),
        (
            "refinance",
            "返済期限を延長、新条件で借換",
            vec![Refinance],
            1,
            a,
        ),
        (
            "covenant",
            "Covenant余裕が縮小",
            vec![Stress(0.5), MarketBp(40.)],
            3,
            Condition::Leveraged,
        ),
        (
            "liquidity_warning",
            "資金繰りの余裕が縮小",
            vec![Stress(0.6), BondLiquidity(-10.), MarketBp(45.)],
            3,
            Condition::CashStrained,
        ),
        (
            "tender",
            "Bond Tender、負債整理を進める",
            vec![DebtReduction, MarketBp(-10.)],
            1,
            a,
        ),
        (
            "equity_raise",
            "増資で資金を確保",
            vec![EquityRaise, MarketBp(-20.)],
            1,
            a,
        ),
        (
            "default",
            "支払い不履行",
            vec![Stress(2.)],
            3,
            Condition::SystemOnly,
        ),
        (
            "restructuring",
            "債務再編、回収Cashを支払い",
            vec![DebtReduction],
            3,
            Condition::SystemOnly,
        ),
        (
            "contract_win",
            "長期契約を獲得",
            vec![Revenue(1.06), Ebitda(1.08)],
            1,
            a,
        ),
        (
            "contract_loss",
            "主要契約を失う",
            vec![Revenue(0.90), Ebitda(0.82)],
            2,
            a,
        ),
        (
            "cost_cut",
            "コスト削減が進展",
            vec![Ebitda(1.08), FcfEbitda(0.06)],
            1,
            a,
        ),
        (
            "outage",
            "設備障害、操業が一時停止",
            vec![Revenue(0.97), Ebitda(0.90), FcfEbitda(-0.1)],
            2,
            a,
        ),
        (
            "litigation",
            "訴訟関連費用を計上",
            vec![CashEbitda(-0.2), FcfEbitda(-0.1), Stress(0.2)],
            2,
            a,
        ),
        (
            "impairment",
            "資産減損を発表",
            vec![AssetQuality(-0.12), Stress(0.2), MarketBp(20.)],
            2,
            a,
        ),
        (
            "capex_overrun",
            "建設費用が計画を超過",
            vec![FcfEbitda(-0.25), Debt(1.06)],
            2,
            a,
        ),
        (
            "working_capital",
            "在庫と売掛金がCashを消費",
            vec![CashEbitda(-0.35), FcfEbitda(-0.15)],
            2,
            a,
        ),
        (
            "collateral",
            "担保資産の評価が低下",
            vec![AssetQuality(-0.15), MarketBp(25.)],
            2,
            a,
        ),
        (
            "credit_line",
            "銀行が融資枠の条件を厳格化",
            vec![Stress(0.4), MarketBp(25.), CashEbitda(-0.1)],
            2,
            a,
        ),
    ] {
        add(id, i, title, effects, severity, condition);
    }
    list
}
