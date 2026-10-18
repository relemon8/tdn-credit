//! Optional badges observe financial state and never consume randomness or grant rewards.
use crate::PresentationState;
use credit_core::{model::*, GameState};

pub enum Condition {
    Assets(i64),
    Event(&'static str),
    Buy,
    Sell,
    Complete,
    Diversify,
    BeatIndex,
}
pub struct Achievement {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub condition: Condition,
}
macro_rules! badge {
    ($id:literal, $title:literal, $description:literal, $condition:expr) => {
        Achievement {
            id: $id,
            title: $title,
            description: $description,
            condition: $condition,
        }
    };
}
use Condition::*;
pub const CATALOG: &[Achievement] = &[
    badge!(
        "assets65",
        "まずうちさぁ、65億あるんだけど",
        "純資産65億円に到達",
        Assets(6_500_000_000)
    ),
    badge!(
        "assets81",
        "野獣の入口",
        "純資産81億円に到達",
        Assets(8_100_000_000)
    ),
    badge!(
        "assets100",
        "やりますねぇ！",
        "純資産100億円に到達",
        Assets(10_000_000_000)
    ),
    badge!(
        "assets114",
        "いいよ！来いよ！",
        "純資産114億円に到達",
        Assets(11_400_000_000)
    ),
    badge!(
        "assets514",
        "来いよ、次の桁へ",
        "純資産514億円に到達",
        Assets(51_400_000_000)
    ),
    badge!(
        "assets810",
        "野獣級の資産",
        "純資産810億円に到達",
        Assets(81_000_000_000)
    ),
    badge!(
        "assets1919",
        "いいゾ～これ",
        "純資産1,919億円に到達",
        Assets(191_900_000_000)
    ),
    badge!(
        "assets114514",
        "114514億円、到達です",
        "純資産114,514億円に到達（超長期のロマン枠）",
        Assets(11_451_400_000_000)
    ),
    badge!("buy", "買って、どうぞ", "初めて債券を購入", Buy),
    badge!("sell", "じゃあ俺、売って帰るから", "初めて債券を売却", Sell),
    badge!(
        "complete",
        "お疲れナス！",
        "24か月以上の運用を完走",
        Complete
    ),
    badge!(
        "diversify",
        "こ↑こ↓にも配分",
        "同時に5社以上の債券を保有",
        Diversify
    ),
    badge!(
        "beat_index",
        "はっきりわかんだね",
        "評価期間後に比較指数を上回る",
        BeatIndex
    ),
    badge!(
        "hike",
        "おっ、上がってんじゃーん",
        "利上げイベントを経験",
        Event("hike")
    ),
    badge!(
        "cut",
        "冷えてるか～？",
        "利下げイベントを経験",
        Event("cut")
    ),
    badge!(
        "inflation",
        "物価こわれる",
        "インフレショックを経験",
        Event("inflation")
    ),
    badge!(
        "recession",
        "ああ＾～景気が落ちるんじゃ＾～",
        "景気後退イベントを経験",
        Event("recession")
    ),
    badge!(
        "crisis",
        "まずいですよ！",
        "信用危機イベントを経験",
        Event("crisis")
    ),
    badge!(
        "injection",
        "ありがとナス！",
        "流動性供給イベントを経験",
        Event("injection")
    ),
    badge!(
        "quality",
        "逃げるんだよォ！",
        "安全資産への逃避を経験",
        Event("quality")
    ),
    badge!(
        "soft_landing",
        "いい着地してんねぇ",
        "軟着陸イベントを経験",
        Event("soft_landing")
    ),
    badge!(
        "growth",
        "すっげぇ成長してる",
        "成長の上振れを経験",
        Event("growth")
    ),
    badge!(
        "funding",
        "資金、出そうと思えば",
        "資金市場の緊張を経験",
        Event("funding")
    ),
    badge!(
        "beat",
        "いい仕事してますねぇ",
        "好決算イベントを経験",
        Event("beat")
    ),
    badge!(
        "miss",
        "悲しいなぁ",
        "決算未達イベントを経験",
        Event("miss")
    ),
    badge!(
        "guidance_cut",
        "見通しが冷えてますよ",
        "業績見通しの引き下げを経験",
        Event("guidance_cut")
    ),
    badge!(
        "guidance_raise",
        "見とけよ見とけよ～",
        "業績見通しの引き上げを経験",
        Event("guidance_raise")
    ),
    badge!(
        "merger",
        "合体、いいっすねぇ",
        "合併イベントを経験",
        Event("merger")
    ),
    badge!(
        "debt_acquisition",
        "借りすぎィ！",
        "借入による買収を経験",
        Event("debt_acquisition")
    ),
    badge!(
        "asset_sale",
        "売って資金にしていく",
        "資産売却を経験",
        Event("asset_sale")
    ),
    badge!(
        "dividend",
        "株主にはいいゾ～これ",
        "増配イベントを経験",
        Event("dividend")
    ),
    badge!(
        "buyback",
        "自分を買う",
        "自社株買いを経験",
        Event("buyback")
    ),
    badge!(
        "debt_reduction",
        "返済して、どうぞ",
        "負債削減を経験",
        Event("debt_reduction")
    ),
    badge!(
        "ceo",
        "お前が経営するのか…",
        "経営者交代を経験",
        Event("ceo")
    ),
    badge!(
        "fraud",
        "ファッ！？",
        "不正調査イベントを経験",
        Event("fraud")
    ),
    badge!(
        "rating_review",
        "おっ、そうだな",
        "格付け見直しイベントを経験",
        Event("rating_review")
    ),
    badge!(
        "refinance",
        "まだ借りられる",
        "借り換えイベントを経験",
        Event("refinance")
    ),
    badge!(
        "covenant",
        "約束守って、どうぞ",
        "財務制限条項への懸念を経験",
        Event("covenant")
    ),
    badge!(
        "liquidity_warning",
        "現金がないんですがそれは",
        "資金繰り警告を経験",
        Event("liquidity_warning")
    ),
    badge!(
        "tender",
        "買い戻してくれるんですか！？",
        "債券買い戻しを経験",
        Event("tender")
    ),
    badge!(
        "equity_raise",
        "出資ありがとナス！",
        "増資イベントを経験",
        Event("equity_raise")
    ),
    badge!(
        "default",
        "なんだこれは……たまげたなぁ",
        "債務不履行イベントを経験（保有は不要）",
        Event("default")
    ),
    badge!(
        "restructuring",
        "再建して、どうぞ",
        "再建イベントを経験",
        Event("restructuring")
    ),
    badge!(
        "outage",
        "止まってんじゃねーよ",
        "設備停止イベントを経験",
        Event("outage")
    ),
    badge!(
        "capex_overrun",
        "予算こわれる",
        "設備投資の予算超過を経験",
        Event("capex_overrun")
    ),
];

pub fn earned(a: &Achievement, game: &GameState) -> bool {
    match a.condition {
        Assets(target) => game.nav().0 >= target || game.history.iter().any(|r| r.nav.0 >= target),
        Event(id) => {
            game.event_history.iter().any(|e| e.definition_id == id)
                || game.news.iter().any(|n| n.event_id == id)
        }
        Buy => game.trades.iter().any(|t| t.side == Side::Buy),
        Sell => game.trades.iter().any(|t| t.side == Side::Sell),
        Complete => game.evaluation_complete() && game.turn >= 24,
        Diversify => {
            game.portfolio
                .positions
                .keys()
                .map(|id| &game.bonds[id].issuer)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                >= 5
        }
        BeatIndex => game.evaluation_complete() && game.nav().0 > game.benchmark_nav().0,
    }
}

pub fn update(p: &mut PresentationState, game: &GameState) {
    for a in CATALOG {
        if earned(a, game) {
            p.achievements.entry(a.id.to_string()).or_insert(game.turn);
        }
    }
}
