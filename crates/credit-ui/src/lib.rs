use credit_core::{credit, model::*, portfolio, GameState};
use credit_presentation::{oku, percent, PresentationState};
use leptos::prelude::*;

#[derive(Default, Clone)]
pub struct Page {
    pub tab: String,
    pub issuer: String,
    pub query: String,
    pub sort: String,
    pub all: bool,
}

fn seniority_label(value: Seniority) -> &'static str {
    match value {
        Seniority::Secured => "担保付き",
        Seniority::Senior => "通常の優先債",
        Seniority::Subordinated => "返済が後順位の債券",
    }
}

fn hidden(token: &str, revision: u64) -> impl IntoView {
    view! {
        <input type="hidden" name="token" value=token.to_string() />
        <input type="hidden" name="revision" value=revision.to_string() />
    }
}
fn action(token: &str, revision: u64, name: &str, label: &str) -> impl IntoView {
    view! {
        <form method="post" action="/command">
            {hidden(token, revision)}
            <button
                name="action"
                value=name.to_string()
                accesskey=if name == "next" { Some("x") } else { None }
            >
                {label.to_string()}
            </button>
        </form>
    }
}
fn sign(value: f64) -> &'static str {
    if value >= 0. {
        "positive"
    } else {
        "negative"
    }
}
fn stat(label: &str, value: String, sub: String) -> impl IntoView {
    view! {
        <div class="stat">
            <span class="eyebrow">{label.to_string()}</span>
            <strong>{value}</strong>
            <small>{sub}</small>
        </div>
    }
}
fn desk_fair(g: &GameState, b: &BondState) -> f64 {
    let mut i = g.issuers[&b.issuer].clone();
    i.financials = i.disclosed.clone();
    i.event_stress = 0.;
    i.score = credit::score(&i, &g.macro_state);
    credit::fair_spread(&i, b, &g.macro_state, g.turn).value()
}
fn bond_card(g: &GameState, b: &BondState, token: &str, revision: u64) -> AnyView {
    let i = &g.issuers[&b.issuer];
    let held = g.portfolio.positions.get(&b.id).map_or(0, |p| p.face.0);
    let estimate = desk_fair(g, b);
    let range = match g.config.difficulty {
        Difficulty::Rookie => 25.,
        Difficulty::Analyst => 40.,
        _ => 70.,
    };
    let remaining = b.maturity_month.saturating_sub(g.turn) as f64 / 12.;
    view! {
        <article class="bond-card">
            <div class="card-top">
                <a class="ticker" href=format!("/?tab=issuers&issuer={}", b.issuer)>
                    {b.issuer.clone()}
                </a>
                <span
                    class="badge"
                    title="企業の公表格付け。債券ごとの返済優先順位とは別の指標です"
                >
                    {i.agency.published.label()}
                </span>
                <span class="muted">{seniority_label(b.seniority)}</span>
            </div>
            <h3>
                {format!(
                    "{} {:.2} / {}",
                    b.id,
                    b.coupon.value() * 100.,
                    2030 + b.maturity_month / 12,
                )}
            </h3>
            <div class="quote">
                <strong>{format!("{:.2}", b.clean_price.value())}</strong>
                <span>
                    {b
                        .yield_rate
                        .map(|y| format!("満期までの利回り {:.2}%", y.value() * 100.))
                        .unwrap_or("回収待ちの債権".into())}
                </span>
            </div>
            <dl class="mini-grid">
                <div>
                    <dt>{if i.defaulted { "破綻前の上乗せ利回り" } else { "市場の上乗せ利回り" }}</dt>
                    <dd>{format!("{:.0} bp", b.market_spread.value())}</dd>
                </div>
                <div>
                    <dt>"妥当な上乗せ幅（推計）"</dt>
                    <dd>
                        {if i.defaulted {
                            "— / 破綻後の債権".into()
                        } else {
                            format!(
                                "{:.0}–{:.0} bp",
                                (estimate - range).max(25.),
                                estimate + range,
                            )
                        }}
                    </dd>
                </div>
                <div>
                    <dt title="デュレーション。金利が1%上がると、価格はおよそこの数値の%分下がります">"金利への敏感さ"</dt>
                    <dd>{format!("{:.2} y", b.duration.value())}</dd>
                </div>
                <div>
                    <dt>"残存 / 保有額面"</dt>
                    <dd>{format!("{remaining:.1} y / {} 億円", oku(held))}</dd>
                </div>
            </dl>
            <details class="bond-terms">
                <summary>"債券の条件・破綻時の回収"</summary>
                <p>
                    {format!(
                        "利息 年{:.2}%・半年払い / {} / 回収率 {} {:.0}%（破綻時に確定）",
                        b.coupon.value() * 100.,
                        seniority_label(b.seniority),
                        if i.defaulted { "確定値" } else { "基準仮定" },
                        b.recovery.value() * 100.,
                    )}
                </p>
            </details>
            <p class="quote-line">
                {format!(
                    "売却 {:.2} / 購入 {:.2} · 利息込みの価格",
                    b.bid().value(),
                    b.ask().value(),
                )}
            </p>
            <form method="post" action="/command" class="trade-form">
                {hidden(token, revision)}
                <input type="hidden" name="bond" value=b.id.clone() />
                <label>
                    "額面（円）"
                    <input
                        name="amount"
                        type="number"
                        min="10000000"
                        step="10000000"
                        value="1000000000"
                        required
                        aria-label=format!("{} 注文額面（円）", b.id)
                    />
                </label>
                <div class="button-pair">
                    <button name="action" value="buy" disabled=b.settled>
                        "買う（売り手の提示価格）"
                    </button>
                    <button
                        name="action"
                        value="sell"
                        disabled=b.settled || held == 0
                    >
                        "売る（買い手の提示価格）"
                    </button>
                </div>
            </form>
        </article>
    }
    .into_any()
}
fn market_page(g: &GameState, token: &str, revision: u64) -> AnyView {
    let bnav = g.benchmark_nav().0 as f64 / g.initial_capital.0 as f64 - 1.;
    let ret = g.nav().0 as f64 / g.initial_capital.0 as f64 - 1.;
    let monthly = g.history.last().map_or(0., |r| r.monthly_return);
    let mut focus: Vec<&BondState> = g.bonds.values().filter(|b| !b.settled).collect();
    focus.sort_by(|a, b| {
        let score = |x: &BondState| {
            let held = g.portfolio.positions.contains_key(&x.id);
            let movement = g
                .history
                .iter()
                .rev()
                .nth(1)
                .and_then(|r| r.prices.get(&x.id))
                .map_or(0., |p| (x.clean_price.value() - p.value()).abs());
            movement + if held { 3. } else { 0. }
        };
        score(b).total_cmp(&score(a)).then(a.id.cmp(&b.id))
    });
    let mut issuers = std::collections::BTreeSet::new();
    let focus: Vec<_> = focus
        .into_iter()
        .filter(|b| issuers.insert(b.issuer.clone()))
        .take(3)
        .collect();
    view! {
        <section class="stats">
            {stat(
                "運用資産（純資産）",
                format!("{} 億円", oku(g.nav().0)),
                format!("累積 {}", percent(ret)),
            )}{stat("先月の運用成績", percent(monthly), "月末に確定・売買の中間価格で評価".into())}
            {stat("指数との差（累積）", percent(ret - bnav), format!("比較指数 {}", percent(bnav)))}
            {stat(
                "現金",
                format!("{} 億円", oku(g.portfolio.cash.0)),
                format!(
                    "推定清算価値 {} 億円",
                    oku(portfolio::liquidating_nav(&g.portfolio, &g.bonds).0),
                ),
            )}
        </section>
        <section class="window">
            <div class="titlebar">
                <h2>"市場の状況"</h2>
                <span>{match g.macro_state.regime { Regime::Goldilocks => "安定成長", Regime::Inflation => "インフレ", Regime::Recession => "景気後退", Regime::CreditCrisis => "信用危機" }}</span>
            </div>
            <div class="macro-strip">
                <span>{format!("2年国債 {:.2}%", g.treasury.two.value() * 100.)}</span>
                <span>{format!("5年国債 {:.2}%", g.treasury.five.value() * 100.)}</span>
                <span>{format!("10年国債 {:.2}%", g.treasury.ten.value() * 100.)}</span>
                <span>{format!("GDP {:.1}%", g.macro_state.gdp_growth.value() * 100.)}</span>
                <span>{format!("物価上昇率 {:.1}%", g.macro_state.inflation.value() * 100.)}</span>
                <span>{format!("売買のしやすさ {:.0}/100", g.macro_state.liquidity * 100.)}</span>
                <span>{format!("市場の不安度 {:.0}", g.macro_state.volatility)}</span>
            </div>
        </section>
        <div class="section-label">
            <h2>"注目の3社"</h2>
            <a href="/?tab=issuers">"債券一覧を見る →"</a>
        </div>
        <div class="bond-grid">
            {focus.into_iter().map(|b| bond_card(g, b, token, revision)).collect_view()}
        </div>
        <div class="lower-grid">
            <section class="window">
                <div class="titlebar">
                    <h2>"重要ニュース"</h2>
                    <a href="/?tab=news">"すべて"</a>
                </div>
                {news_list(g, true)}
            </section>
            <section class="window">
                <div class="titlebar">
                    <h2>"今月の損益の理由"</h2>
                    <a href="/?tab=analysis">"詳細"</a>
                </div>
                {attribution_summary(g)}
            </section>
        </div>
    }
    .into_any()
}
fn news_list(g: &GameState, short: bool) -> AnyView {
    let mut items: Vec<_> = g
        .news
        .iter()
        .filter(|n| !short || n.turn == g.turn)
        .collect();
    let mandatory = |n: &&NewsItem| {
        n.severity >= 3
            && n.issuer.as_ref().is_some_and(|id| {
                g.portfolio
                    .positions
                    .keys()
                    .any(|bond| g.bonds[bond].issuer == *id)
            })
    };
    items.sort_by(|a, b| {
        b.turn
            .cmp(&a.turn)
            .then(mandatory(b).cmp(&mandatory(a)))
            .then(b.severity.cmp(&a.severity))
    });
    if short {
        let selected: Vec<_> = items
            .iter()
            .enumerate()
            .filter(|(index, n)| *index < 3 || mandatory(n) || n.event_id == "market_comment")
            .map(|(_, n)| *n)
            .collect();
        items = selected;
    }
    view! {
        <div class="news-list">
            {if items.is_empty() {
                Some(
                    view! {
                        <p class="empty">
                            "新しい重要開示はありません。価格と保有リスクを確認できます。"
                        </p>
                    },
                )
            } else {
                None
            }}
            {items
                .into_iter()
                .map(|n| {
                    view! {
                        <article class="news-item">
                            <span class="eyebrow">
                                {format!(
                                    "{:02}か月目 / {}",
                                    n.turn,
                                    n.issuer.as_deref().unwrap_or("市場"),
                                )}
                            </span>
                            <h3>{credit_presentation::news::headline(n)}</h3>
                            {credit_presentation::news::lead(&n.event_id).map(|text| view! { <p class="news-lead">{text}</p> })}
                            <p class="news-numbers">{credit_presentation::news::detail(n)}</p>
                        </article>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}
fn attribution_summary(g: &GameState) -> AnyView {
    let a = g
        .history
        .last()
        .map(|r| r.attribution.clone())
        .unwrap_or_default();
    let mut rows = vec![
        ("利息などの収益", a.carry),
        ("金利変動の損益", a.rates),
        ("信用評価の変化", a.spread),
        ("破綻・回収の損益", a.default_loss),
        ("売買コスト", a.trading_cost),
    ];
    rows.sort_by_key(|(_, v)| -v.abs());
    view! {
        <div class="attribution">
            {rows
                .into_iter()
                .map(|(name, value)| {
                    view! {
                        <div>
                            <span>{name}</span>
                            <strong class=sign(
                                value as f64,
                            )>{format!("{:+.3} 億円", value as f64 / 100_000_000.)}</strong>
                        </div>
                    }
                })
                .collect_view()}
            <small>
                {format!(
                    "分解しきれない差額 {} 円 / 前月末比。初月は初期売買コストを含む。",
                    a.residual,
                )}
            </small>
        </div>
    }
    .into_any()
}
fn issuers_page(g: &GameState, page: &Page, token: &str, revision: u64) -> AnyView {
    let selected = g.issuers.get(&page.issuer);
    let query = page.query.to_lowercase();
    let mut bonds: Vec<_> = g
        .bonds
        .values()
        .filter(|b| {
            (!b.settled || page.all)
                && (page.issuer.is_empty() || b.issuer == page.issuer)
                && (query.is_empty()
                    || b.id.to_lowercase().contains(&query)
                    || g.issuers[&b.issuer].name.to_lowercase().contains(&query))
        })
        .collect();
    match page.sort.as_str() {
        "yield" => bonds.sort_by(|a, b| {
            b.yield_rate
                .map_or(0., |v| v.value())
                .total_cmp(&a.yield_rate.map_or(0., |v| v.value()))
        }),
        "spread" => {
            bonds.sort_by(|a, b| b.market_spread.value().total_cmp(&a.market_spread.value()))
        }
        "duration" => bonds.sort_by(|a, b| a.duration.value().total_cmp(&b.duration.value())),
        _ => {}
    }
    view! {
        <div class="section-label">
            <h2>"企業・債券を探す"</h2>
            <a href="/?tab=issuers">"選択解除"</a>
        </div>
        <div class="issuer-chips">
            {g
                .issuers
                .values()
                .map(|i| {
                    view! {
                        <a href=format!(
                            "/?tab=issuers&issuer={}",
                            i.ticker,
                        )>{format!("{} · {}", i.ticker, i.agency.published.label())}</a>
                    }
                })
                .collect_view()}
        </div>
        {selected
            .map(|i| {
                let f = &i.disclosed;
                view! {
                    <section class="window issuer-detail">
                        <div class="titlebar">
                            <h2>{i.name.clone()}</h2>
                            <span>
                                {format!(
                                    "{} / {} {}",
                                    i.agency.published.label(),
                                    match i.agency.outlook { Outlook::Positive => "見通し：改善方向", Outlook::Stable => "見通し：安定", Outlook::Negative => "見通し：悪化方向" },
                                    if i.agency.watch { "格付け見直し中" } else { "" },
                                )}
                            </span>
                        </div>
                        <p>{i.profile.clone()}</p>
                        <div class="macro-strip">
                            <span>{format!("負債倍率 {:.2}倍", f.leverage())}</span>
                            <span>{format!("自由に使える現金 {:.1} 億円", f.fcf)}</span>
                            <span>{format!("利払い余力 {:.2}倍", f.coverage())}</span>
                            <span>{format!("財務情報の開示：{}か月目", i.disclosure_turn)}</span>
                        </div>
                        <details>
                            <summary>"財務・借入の返済期限・信用分析"</summary>
                            <dl class="mini-grid">
                                <div>
                                    <dt>"売上 / 利払い・税・償却前利益"</dt>
                                    <dd>{format!("{:.1} / {:.1} 億円", f.revenue, f.ebitda)}</dd>
                                </div>
                                <div>
                                    <dt>"現金 / 有利子負債 / 差引負債"</dt>
                                    <dd>
                                        {format!(
                                            "{:.1} / {:.1} / {:.1}",
                                            f.cash,
                                            f.debt,
                                            f.net_debt(),
                                        )}
                                    </dd>
                                </div>
                                <div>
                                    <dt>"利益率 / 自由に使える現金の比率"</dt>
                                    <dd>
                                        {format!(
                                            "{:.1}% / {:.1}%",
                                            f.margin() * 100.,
                                            f.fcf_margin() * 100.,
                                        )}
                                    </dd>
                                </div>
                                <div>
                                    <dt>"借入の返済期限別残高（1〜5年）"</dt>
                                    <dd>
                                        {format!("{:?} 億円", f.debt_maturity.map(|v| v.round()))}
                                    </dd>
                                </div>
                                <div>
                                    <dt>"短期の支払い余力"</dt>
                                    <dd>{format!("{:.2}x", f.liquidity())}</dd>
                                </div>
                                <div>
                                    <dt>"財務情報の開示"</dt>
                                    <dd>
                                        "前回開示値。市場は新しい情報を先行して反映する場合があります。"
                                    </dd>
                                </div>
                            </dl>
                            <p>
                                "信用スコアは負債倍率、利払い余力、支払い余力、自由に使える現金、事業の安定性、景気、イベントリスクから推計。公表格付けは継続した証拠を確認してから変更されます。"
                            </p>
                        </details>
                    </section>
                }
            })}
        {selected.map(|i| issuer_history(g, i))}
        <form method="get" class="search">
            <input type="hidden" name="tab" value="issuers" />
            <input type="hidden" name="issuer" value=page.issuer.clone() />
            <label>
                "検索 "<input name="q" value=page.query.clone() placeholder="銘柄コード・会社名" />
            </label>
            <label>
                "並べ替え "<select name="sort">
                    <option value="id" selected=page.sort == "id">
                        "銘柄コード"
                    </option>
                    <option value="yield" selected=page.sort == "yield">
                        "利回りが高い順"
                    </option>
                    <option value="spread" selected=page.sort == "spread">
                        "上乗せ利回りが大きい順"
                    </option>
                    <option value="duration" selected=page.sort == "duration">
                        "金利への敏感さが小さい順"
                    </option>
                </select>
            </label>
            <label>
                <input type="checkbox" name="all" value="1" checked=page.all />
                "償還済みも表示"
            </label>
            <button>"適用"</button>
        </form>
        {if selected.is_some() {
            view! {
                <div class="bond-grid">
                    {bonds.into_iter().map(|b| bond_card(g, b, token, revision)).collect_view()}
                </div>
            }
                .into_any()
        } else {
            bond_table(g, &bonds)
        }}
    }.into_any()
}

fn bond_table(g: &GameState, bonds: &[&BondState]) -> AnyView {
    view! {
        <section class="window table-scroll">
            <table class="bond-table">
                <caption>
                    "債券一覧 / 償還済み銘柄は履歴として残ります"
                </caption>
                <thead>
                    <tr>
                        <th>"債券"</th>
                        <th>"企業の格付け"</th>
                        <th>"価格"</th>
                        <th>"利回り"</th>
                        <th>"市場の上乗せ幅（bp）"</th>
                        <th>"妥当な上乗せ幅（bp・推計）"</th>
                        <th>"金利への敏感さ"</th>
                        <th>"売却 / 購入価格（利息込み）"</th>
                        <th>"返済の優先順位"</th>
                    </tr>
                </thead>
                <tbody>
                    {bonds
                        .iter()
                        .map(|b| {
                            let i = &g.issuers[&b.issuer];
                            let estimate = desk_fair(g, b);
                            let range = match g.config.difficulty {
                                Difficulty::Rookie => 25.,
                                Difficulty::Analyst => 40.,
                                _ => 70.,
                            };
                            view! {
                                <tr>
                                    <td>
                                        <a href=format!(
                                            "/?tab=issuers&issuer={}",
                                            b.issuer,
                                        )>
                                            {format!(
                                                "{} {:.2} / {}",
                                                b.id,
                                                b.coupon.value() * 100.,
                                                2030 + b.maturity_month / 12,
                                            )}
                                        </a>
                                    </td>
                                    <td>{i.agency.published.label()}</td>
                                    <td>{format!("{:.2}", b.clean_price.value())}</td>
                                    <td>
                                        {b
                                            .yield_rate
                                            .map_or(
                                                "—".into(),
                                                |y| format!("{:.2}%", y.value() * 100.),
                                            )}
                                    </td>
                                    <td>
                                        {if i.defaulted {
                                            "—".into()
                                        } else {
                                            format!("{:.0}", b.market_spread.value())
                                        }}
                                    </td>
                                    <td>
                                        {if i.defaulted {
                                            "破綻後の債権".into()
                                        } else {
                                            format!(
                                                "{:.0}–{:.0}",
                                                (estimate - range).max(25.),
                                                estimate + range,
                                            )
                                        }}
                                    </td>
                                    <td>{format!("{:.2}y", b.duration.value())}</td>
                                    <td>
                                        {format!("{:.2} / {:.2}", b.bid().value(), b.ask().value())}
                                    </td>
                                    <td>{seniority_label(b.seniority)}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </section>
    }
    .into_any()
}
fn portfolio_page(g: &GameState, token: &str, revision: u64) -> AnyView {
    let nav = g.nav().0 as f64;
    let market_value = nav - g.portfolio.cash.0 as f64;
    let weighted = |f: fn(&BondState) -> f64| {
        if market_value > 0. {
            g.portfolio
                .positions
                .iter()
                .map(|(id, p)| p.face.value_at(g.bonds[id].dirty()).0 as f64 * f(&g.bonds[id]))
                .sum::<f64>()
                / market_value
        } else {
            0.
        }
    };
    let duration = weighted(|b| b.duration.value());
    let sd = weighted(|b| b.spread_duration.value());
    let unrealized = g
        .portfolio
        .positions
        .iter()
        .map(|(id, p)| p.face.value_at(g.bonds[id].dirty()).0 - p.cost.0)
        .sum::<i64>();
    view! {
        <div class="section-label">
            <h2>"保有資産・運用状況"</h2>
            <span>"額面と評価額は異なります"</span>
        </div>
        <section class="stats">
            {stat(
                "保有債券の時価",
                format!("{} 億円", oku(market_value as i64)),
                format!("Cash {} 億円", oku(g.portfolio.cash.0)),
            )}{stat("金利への敏感さ", format!("{duration:.2} y"), format!("信用評価への敏感さ {sd:.2}年"))}
            {stat(
                "未確定の損益",
                format!("{} 億円", oku(unrealized)),
                format!("確定済み損益 {} 億円", oku(g.portfolio.realized.0)),
            )}
            {stat(
                "SPREAD DV01",
                format!("{:.0} 円/bp", market_value * sd * 0.0001),
                format!("取引コスト {} 億円", oku(g.portfolio.trading_cost.0)),
            )}
        </section>
        {if g.portfolio.positions.is_empty() {
            Some(
                view! {
                    <p class="empty">
                        "保有債券はありません。MARKETまたはISSUERSで注文できます。Cashで待つことも選択です。"
                    </p>
                },
            )
        } else {
            None
        }}
        <div class="bond-grid">
            {g
                .portfolio
                .positions
                .keys()
                .map(|id| bond_card(g, &g.bonds[id], token, revision))
                .collect_view()}
        </div>
        {portfolio_details(g)}
        <details class="window padded">
            <summary>"売買履歴"</summary>
            <div class="table-scroll">
                <table>
                    <thead>
                        <tr>
                            <th>"月"</th>
                            <th>"債券"</th>
                            <th>"売買"</th>
                            <th>"額面（億円）"</th>
                            <th>"約定Dirty"</th>
                            <th>"Cost（円）"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {g
                            .trades
                            .iter()
                            .rev()
                            .map(|t| {
                                view! {
                                    <tr>
                                        <td>{t.turn}</td>
                                        <td>{t.bond.clone()}</td>
                                        <td>{if t.side == Side::Buy { "購入" } else { "売却" }}</td>
                                        <td>{oku(t.face.0)}</td>
                                        <td>{format!("{:.3}", t.price.value())}</td>
                                        <td>{t.cost.0}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </details>
    }.into_any()
}

fn issuer_history(g: &GameState, i: &IssuerState) -> AnyView {
    let mut disclosed = i.clone();
    disclosed.financials = i.disclosed.clone();
    disclosed.event_stress = 0.;
    disclosed.score = credit::score(&disclosed, &g.macro_state);
    let pd = credit::annual_pd(&disclosed, &g.macro_state, g.config.difficulty);
    let rows: Vec<_> = g
        .history
        .iter()
        .filter(|r| {
            r.turn <= i.disclosure_turn
                && r.turn.is_multiple_of(g.config.difficulty.disclosure_lag())
        })
        .collect();
    view! {
        <details class="window padded">
            <summary>"財務の推移・信用力の推計"</summary>
            <p>
                {format!(
                    "開示ベースの推計Score {:.1}/100 / 年率PD推計 {:.2}% / Sector補正 {:+.1}点",
                    disclosed.score,
                    pd.value() * 100.,
                    credit::sector_score_adjustment(i.sector),
                )}
            </p>
            <p>
                "いずれもモデル推計です。負債倍率24%、利払い余力18%、支払い余力17%、自由に使える現金14%、事業安定性12%、景気8%、イベントリスク7%を加重し、業種特性を補正。公表格付けとは時点・方法が異なります。"
            </p>
            <div class="table-scroll">
                <table>
                    <thead>
                        <tr>
                            <th>"開示月"</th>
                            <th>"Revenue 億円"</th>
                            <th>"EBITDA"</th>
                            <th>"FCF"</th>
                            <th>"負債倍率"</th>
                            <th>"利払い余力"</th>
                            <th>"格付け"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {rows
                            .into_iter()
                            .map(|r| {
                                let f = &r.financials[&i.ticker];
                                view! {
                                    <tr>
                                        <td>{r.turn}</td>
                                        <td>{format!("{:.1}", f.revenue)}</td>
                                        <td>{format!("{:.1}", f.ebitda)}</td>
                                        <td>{format!("{:.1}", f.fcf)}</td>
                                        <td>{format!("{:.2}x", f.leverage())}</td>
                                        <td>{format!("{:.2}x", f.coverage())}</td>
                                        <td>{r.ratings[&i.ticker].label()}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </details>
    }.into_any()
}
fn analysis_page(g: &GameState) -> AnyView {
    view! {
        <div class="section-label">
            <h2>"運用分析・損益の理由"</h2>
            <span>"金額単位：億円"</span>
        </div>
        {nav_chart(g)}
        {stress_panel(g)}
        <div class="lower-grid">
            <section class="window">
                <div class="titlebar">
                    <h2>"今月の成績"</h2>
                </div>
                {attribution_summary(g)}
            </section>
            <section class="window padded">
                <h2>"計算方法について"</h2>
                <p>
                    "価格は国債金利と市場の上乗せ利回りを使い、将来の利息と元本を現在の価値に割り引いて計算します。100bp＝1%。"
                </p>
                <p>
                    "利息などの収益 → 金利変動 → 信用評価の変化 → 破綻 → 売買コストの順で分解します。金利と信用評価が同時に動く効果は信用評価側に含めます。"
                </p>
                <p>
                    "Desk fairは前回開示に基づく推計。真の適正価格や売買推奨を意味しません。"
                </p>
            </section>
        </div>
        {bond_attribution(g)}
        <div class="window table-scroll">
            <table>
                <caption>"月ごとの運用成績"</caption>
                <thead>
                    <tr>
                        <th>"月"</th>
                        <th>"純資産"</th>
                        <th>"運用成績"</th>
                        <th>"利息などの収益"</th>
                        <th>"金利変動の損益"</th>
                        <th>"信用評価"</th>
                        <th>"破綻"</th>
                        <th>"コスト"</th>
                        <th>"差額（円）"</th>
                    </tr>
                </thead>
                <tbody>
                    {g
                        .history
                        .iter()
                        .rev()
                        .map(|r| {
                            view! {
                                <tr>
                                    <td>{r.turn}</td>
                                    <td>{oku(r.nav.0)}</td>
                                    <td class=sign(
                                        r.monthly_return,
                                    )>{percent(r.monthly_return)}</td>
                                    <td>{oku(r.attribution.carry)}</td>
                                    <td>{oku(r.attribution.rates)}</td>
                                    <td>{oku(r.attribution.spread)}</td>
                                    <td>{oku(r.attribution.default_loss)}</td>
                                    <td>{oku(r.attribution.trading_cost)}</td>
                                    <td>{r.attribution.residual}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
        {if g.evaluation_complete() { Some(final_panel(g)) } else { None }}
    }.into_any()
}

fn exposure_table(title: &str, items: &std::collections::BTreeMap<String, f64>) -> AnyView {
    view! {
        <div>
            <h3>{title.to_string()}</h3>
            <dl class="exposures">
                {items
                    .iter()
                    .map(|(name, weight)| {
                        view! {
                            <div>
                                <dt>{name.clone()}</dt>
                                <dd>{format!("{:.1}% NAV", weight * 100.)}</dd>
                            </div>
                        }
                    })
                    .collect_view()}
            </dl>
        </div>
    }
    .into_any()
}
fn portfolio_details(g: &GameState) -> AnyView {
    let m = credit_core::analysis::metrics(g);
    view! {
        <details class="window padded">
            <summary>"投資先の内訳・利回り・平均格付け"</summary>
            <p>
                {format!(
                    "利回り {:.2}% / 平均格付け {} / 債券への投資割合 {:.1}% / 上乗せ利回り1bpあたりの価格感応度 {:.0} 円",
                    m.yield_rate * 100.,
                    m.average_rating.map_or("—", |r| r.label()),
                    m.invested_weight * 100.,
                    m.spread_dv01,
                )}
            </p>
            <div class="exposure-grid">
                {exposure_table("企業別", &m.issuer_exposure)}
                {exposure_table("業種別", &m.sector_exposure)}
                {exposure_table("格付け", &m.rating_exposure)}
                {exposure_table("返済期限別", &m.maturity_exposure)}
            </div>
            <p>
                "平均格付けは時価で重みを付けた平均です。利回りは破綻していない債券を対象とし、破綻後の債権は0として集計。確定済み・未確定の損益では利息収入を別に扱います。"
            </p>
        </details>
    }.into_any()
}
fn stress_panel(g: &GameState) -> AnyView {
    let stresses = credit_core::analysis::scenarios(g);
    let m = credit_core::analysis::metrics(g);
    view! {
        <section class="window">
            <div class="titlebar">
                <h2>"もし市場が動いたら"</h2>
                <span>"即時Stress / 売買推奨ではありません"</span>
            </div>
            <div class="padded">
                <p>
                    {if m.duration > 5. {
                        "金利上昇に対する感応度が大きい構成です。"
                    } else if m.invested_weight < 0.1 {
                        "現在の価格変動への感応度は小さく、Cash比率が高い構成です。"
                    } else {
                        "金利と信用評価の変化が、保有債券の価格に影響します。"
                    }}
                </p>
                <div class="table-scroll">
                    <table>
                        <thead>
                            <tr>
                                <th>"想定する変化"</th>
                                <th>"Mid P/L 億円"</th>
                                <th>"売却時の追加費用（億円）"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {stresses
                                .into_iter()
                                .map(|s| {
                                    view! {
                                        <tr title=s.description>
                                            <td>{s.name}</td>
                                            <td class=sign(s.mid_pnl.0 as f64)>{oku(s.mid_pnl.0)}</td>
                                            <td>{oku(s.liquidation_cost.0)}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
                <p>
                    "景気後退：金利−75bp・上乗せ幅＋150bp・売買価格差2倍。信用危機：−100bp・＋400bp・4倍。将来の破綻と利息収入は含みません。上乗せ幅の下限は0bpです。"
                </p>
            </div>
        </section>
    }.into_any()
}
fn bond_attribution(g: &GameState) -> AnyView {
    let rows = g
        .history
        .last()
        .map(|r| r.bonds.clone())
        .unwrap_or_default();
    view! {
        <details class="window padded">
            <summary>"Bond別 P/L attribution（億円）"</summary>
            <div class="table-scroll">
                <table>
                    <thead>
                        <tr>
                            <th>"債券"</th>
                            <th>"利息などの収益"</th>
                            <th>"金利変動の損益"</th>
                            <th>"信用評価"</th>
                            <th>"破綻"</th>
                            <th>"コスト"</th>
                            <th>"合計"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {rows
                            .into_iter()
                            .map(|(id, a)| {
                                view! {
                                    <tr>
                                        <td>{id}</td>
                                        <td>{oku(a.carry)}</td>
                                        <td>{oku(a.rates)}</td>
                                        <td>{oku(a.spread)}</td>
                                        <td>{oku(a.default_loss)}</td>
                                        <td>{oku(a.trading_cost)}</td>
                                        <td>{oku(a.total())}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </details>
    }
    .into_any()
}
fn nav_chart(g: &GameState) -> AnyView {
    let start = g.initial_capital.0 as f64 / 100_000_000.;
    let mut nav = vec![start];
    nav.extend(g.history.iter().map(|r| r.nav.0 as f64 / 100_000_000.));
    let mut benchmark = vec![start];
    benchmark.extend(
        g.history
            .iter()
            .map(|r| r.benchmark_nav.0 as f64 / 100_000_000.),
    );
    let low = nav.iter().chain(&benchmark).copied().fold(start, f64::min) - 1.;
    let high = nav.iter().chain(&benchmark).copied().fold(start, f64::max) + 1.;
    let points = |values: &[f64]| {
        values
            .iter()
            .enumerate()
            .map(|(n, v)| {
                format!(
                    "{:.1},{:.1}",
                    20. + n as f64 / g.turn.max(g.config.maximum_turns) as f64 * 760.,
                    170. - (v - low) / (high - low) * 140.
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    view! {
        <section class="window padded">
            <h2>{format!("純資産の推移 / 初期{}億円", oku(g.initial_capital.0))}</h2>
            <svg
                class="nav-chart"
                viewBox="0 0 800 200"
                role="img"
                aria-label="自分の純資産は緑、比較指数は灰色の推移。正確な数値は下の月次表に表示。"
            >
                <line x1="20" y1="170" x2="780" y2="170" stroke="#b7c7c0" />
                <polyline
                    points=points(&benchmark)
                    fill="none"
                    stroke="#788980"
                    stroke-width="2"
                    stroke-dasharray="5 4"
                />
                <polyline points=points(&nav) fill="none" stroke="#145848" stroke-width="3" />
                <text x="20" y="193" font-size="11">
                    "Month 0"
                </text>
                <text x="720" y="193" font-size="11">
                    {format!("{}か月目", g.turn.max(g.config.maximum_turns))}
                </text>
            </svg>
            <p>
                "緑：自分の運用 / 破線：比較用の社債指数（満期の元本は同一企業の新債へ再投資、利息は現金で保有、取引費用なし）"
            </p>
        </section>
    }.into_any()
}
fn final_panel(g: &GameState) -> AnyView {
    let s = credit_core::analysis::final_score(g);
    view! {
        <section class="window padded game-over">
            <span class="eyebrow">{format!("{}か月時点の運用結果・継続運用できます", g.turn)}</span>
            <h2>{format!("{} / SCORE {:.1}", s.rank, s.score)}</h2>
            <section class="stats">
                {stat(
                    "累積の運用成績",
                    percent(s.total_return),
                    format!(
                        "比較指数 {} / 指数との差 {}",
                        percent(s.benchmark_return),
                        percent(s.excess),
                    ),
                )}
                {stat(
                    "最大の資産下落率",
                    percent(-s.max_drawdown),
                    format!("Risk-adjusted excess {}", percent(s.risk_adjusted)),
                )}
                {stat(
                    "リスクに対する収益",
                    s.sharpe.map_or("—".into(), |v| format!("{v:.2}")),
                    format!("月次から年率換算 / {}標本の参考値", g.history.len()),
                )}
                {stat(
                    "破綻による損失",
                    format!("{} 億円", oku(s.default_loss.0)),
                    format!("売買コスト {} 億円", oku(s.trading_cost.0)),
                )}
            </section>
            <p>
                {format!(
                    "Best month {} / Worst month {} / Average duration {:.2}y / Average rating {}",
                    percent(s.best_month),
                    percent(s.worst_month),
                    s.average_duration,
                    s.average_rating.map_or("—", |r| r.label()),
                )}
            </p>
            <p>
                "評価は指数との差、リスクに対する収益、資産の下落率、投資先の集中度、破綻損失を総合します。最高ランクには継続した投資と分散も必要です。"
            </p>

        </section>
    }.into_any()
}

fn beginner_guide() -> AnyView {
    view! {
        <details class="window beginner-guide" id="how-to-play">
            <summary>"遊び方：30秒版 / 詳しい版"</summary>
            <section aria-labelledby="quick-guide-title">
                <h2 id="quick-guide-title">"30秒でわかる遊び方"</h2>
                <p>"あなたは債券ファンドの運用担当。新規ゲームは65億円で開始し、まず24か月で比較指数を上回る運用を目指します。その後も同じ資産で継続できます。"</p>
                <ol>
                    <li>"「市場」でニュースと注目の3社を見る。"</li>
                    <li>"気になる銘柄を開き、買う・売る・見送るを決める。最初は少額でもOK。"</li>
                    <li>"「翌月へ」で進め、「今月の損益の理由」を確認する。"</li>
                </ol>
                <p class="guide-hint">"迷ったら現金で待つこともできます。高い利回りは、高いリスクへの見返りかもしれません。"</p>
            </section>
            <details class="guide-long">
                <summary>"詳しい遊び方（3〜5分）"</summary>
                <h3>"1. 債券を買うと、何が起きる？"</h3>
                <p>"債券は企業にお金を貸す証券です。保有すると契約に沿った利息を受け取り、満期には元本が返ります。ただし、企業が破綻すると全額は戻らないことがあります。途中で売るときの価格も変動します。満期を迎えた債券は償還され、破綻していない企業の新しい借り換え債が登場します。新債を買うかは自由です。"</p>
                <h3>"2. 最初の1か月を動かしてみる"</h3>
                <p>"「企業・債券」で1社を選び、2本の債券を見比べます。額面欄は円単位で、100000000は1億円。注文は1,000万円刻みです。「買う」で約定し、「保有資産」に反映されます。売却は保有額面まで、購入は現金の範囲内です。"</p>
                <p>"額面は満期に返る元本の基準額です。価格100は額面と同じ、価格95は額面の95%。実際の支払額には経過利息も加わります。画面の購入・売却価格は利息込みです。"</p>
                <p>"買った直後に少し損が出ても、売却価格と購入価格の差が理由の場合があります。注文後は「翌月へ」で市場を進めます。売買せず月を進めることもできます。"</p>
                <h3>"3. はじめての方向け：数字の読み方"</h3>
                <dl class="guide-terms">
                    <dt>"格付け"</dt><dd>"信用力の目安。企業の変化に遅れて動くことがあります。高格付けでも価格下落は起こります。"</dd>
                    <dt>"満期までの利回り"</dt><dd>"現在の価格で買い、契約どおりの利息と元本を受け取る場合の年率の目安。来月の利益や実現する収益の保証ではありません。"</dd>
                    <dt>"上乗せ利回り（スプレッド）"</dt><dd>"国債金利への上乗せ幅です。100bp＝1%。市場の幅は今の価格に織り込まれた値、妥当な幅は開示情報からの推計で、どちらも将来を保証しません。"</dd>
                    <dt>"負債倍率・利払い余力"</dt><dd>"負債倍率は純負債が年間の稼ぐ力の何倍か。利払い余力は利益が利息の何倍あるかを表します。"</dd>
                    <dt>"自由に使える現金（FCF）"</dt><dd>"事業と設備投資の後に残る現金。赤字でも成長投資中の場合があり、理由をニュースや財務から確認できます。"</dd>
                    <dt>"金利への敏感さ（デュレーション）"</dt><dd>"例えば4なら、金利が1%上がると価格はおよそ4%下がる目安です。信用評価への敏感さは、上乗せ幅が動いたときの影響を表します。"</dd>
                </dl>
                <h3>"4. 良い会社と、良い投資は別"</h3>
                <p>"例えば市場の上乗せ幅250bp、妥当な幅180bpなら、市場は推計より厚い見返りを提示しています。一方で、未開示の悪化や売りにくさを織り込んでいる可能性もあります。差だけで売買を決めず、ニュース、負債、現金、満期も合わせて見ると判断が深まります。"</p>
                <h3>"5. 勝ち負けの理由を見る"</h3>
                <p>"「運用分析」で、利息などの収益・金利変動・信用評価の変化・売買コスト・破綻による損益を確認できます。金利や上乗せ幅が上がると価格は通常下がります。利息を受け取っていても、価格下落が上回れば損失です。"</p>
                <p>"純資産は現金と保有債券の評価額の合計。「指数との差」は同じ市場で動く比較用の社債指数との差です。評価には収益だけでなく、大きな下落や投資先の偏りも影響します。"</p>
                <h3>"6. 保存・難易度・実績"</h3>
                <p>"操作が成功すると自動保存します。下の「セーブ・設定・新しいゲーム」では手動保存と読み込みもできます。入門は説明と開示が多め。シナリオ番号（Seed）が同じなら、同じ設定で市場を再現できます。"</p>
                <p>"「実績図鑑」は売買・イベント・完走などの記念です。市場や収益を有利にする報酬はありません。語録は設定でOFFにできます。"</p>
            </details>
        </details>
    }.into_any()
}

pub fn render(
    g: &GameState,
    p: &PresentationState,
    page: &Page,
    token: &str,
    revision: u64,
    message: &str,
) -> String {
    let content = match page.tab.as_str() {
        "portfolio" => portfolio_page(g, token, revision),
        "issuers" => issuers_page(g, page, token, revision),
        "news" => news_list(g, false),
        "analysis" => analysis_page(g),
        _ => market_page(g, token, revision),
    };
    let badges = credit_presentation::achievements::CATALOG;
    let unlocked = badges
        .iter()
        .filter(|a| {
            p.achievements.contains_key(a.id) || credit_presentation::achievements::earned(a, g)
        })
        .count();
    let html = view! {
        <html lang="ja">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width,initial-scale=1" />
                <title>"TDN CREDIT — 債券運用デスク"</title>
                <link rel="stylesheet" href="/assets/style.css" />
            </head>
            <body>
                <a class="skip" href="#main">
                    "本文へ移動"
                </a>
                <header>
                    <div class="brand">
                        <span class="logo-mark">"▥"</span>
                        <div>
                            <h1>"TDN CREDIT"</h1>
                            <span class="eyebrow">"架空の市場、本気の運用判断"</span>
                        </div>
                    </div>
                    <div class="session">
                        <span class="live-dot"></span>
                        <span>
                            {format!(
                                "ローカル · シナリオ {} · {}",
                                g.config.seed,
                                match g.config.difficulty { Difficulty::Rookie => "入門", Difficulty::Analyst => "標準", Difficulty::Pm => "運用のプロ", Difficulty::Distressed => "危機相場", Difficulty::Beast => "野獣級" },
                            )}
                        </span>
                        <strong>
                            {format!("{:02}か月目 / 評価目安 {}か月", g.turn, g.config.maximum_turns)}
                        </strong>
                    </div>
                </header>
                <nav aria-label="メインナビゲーション">
                    {[
                        ("market", "市場"),
                        ("portfolio", "保有資産"),
                        ("issuers", "企業・債券"),
                        ("news", "ニュース"),
                        ("analysis", "運用分析"),
                    ]
                        .into_iter()
                        .map(|(id, label)| {
                            view! {
                                <a
                                    class=if page.tab == id
                                        || (page.tab.is_empty() && id == "market")
                                    {
                                        "active"
                                    } else {
                                        ""
                                    }
                                    aria-current=if page.tab == id { Some("page") } else { None }
                                    href=format!("/?tab={id}")
                                >
                                    {label}
                                </a>
                            }
                        })
                        .collect_view()}
                    <div class="nav-end">
                        {action(
                            token,
                            revision,
                            "next",
                            "翌月へ →",
                        )}
                    </div>
                </nav>
                <main id="main">
                    {if !message.is_empty() {
                        Some(
                            view! {
                                <p class="notice" role="status">
                                    {message.to_string()}
                                </p>
                            },
                        )
                    } else {
                        None
                    }}
                    {if g.turn == g.config.maximum_turns && page.tab != "analysis" {
                        Some(final_panel(g))
                    } else {
                        None
                    }}
                    {content}
                    <details class="window settings">
                        <summary>{format!("実績図鑑：{unlocked} / {} 達成", badges.len())}</summary>
                        <p>"報酬なしの記念実績です。イベント実績は保有不要。新しいゲームへ引き継ぎ、セーブに保存します。古いセーブを読むと、その時点の記録に戻ります。"</p>
                        <div class="achievement-grid">
                            {badges.iter().map(|a| {
                                let done = p.achievements.contains_key(a.id) || credit_presentation::achievements::earned(a, g);
                                view! { <article class=if done { "achievement earned" } else { "achievement" }>
                                    <strong>{format!("{} {}", if done { "✓" } else { "○" }, if p.meme_enabled { a.title } else { a.description })}</strong>
                                    <p>{a.description}</p>
                                </article> }
                            }).collect_view()}
                        </div>
                    </details>
                    {beginner_guide()}
                    <details class="window settings">
                        <summary>"セーブ・設定・新しいゲーム"</summary>
                        <div class="settings-grid">
                            <div>
                                <h3>"ローカル保存"</h3>
                                <div class="button-pair">
                                    {action(token, revision, "save", "セーブする")}
                                    {action(token, revision, "load", "セーブを読み込む")}
                                </div>
                                <p>
                                    <a href="/export">"セーブデータを書き出す（JSON）"</a>
                                </p>
                            </div>
                            <div>
                                <h3>"演出設定"</h3>
                                {action(
                                    token,
                                    revision,
                                    "meme",
                                    if p.meme_enabled {
                                        "語録テロップ：表示中 → 消す"
                                    } else {
                                        "語録テロップ：非表示 → 出す"
                                    },
                                )}
                                <p>"表示設定は市場シナリオを変えません。"</p>
                            </div>
                            <form method="post" action="/command">
                                {hidden(token, revision)}
                                <h3>"新しいゲーム"</h3>
                                <label>
                                    "シナリオ番号（Seed） "
                                    <input
                                        name="seed"
                                        type="number"
                                        min="0"
                                        value="1919"
                                        required
                                    />
                                </label>
                                <label>
                                    "難易度 "<select name="difficulty">
                                        <option value="Rookie">"入門（Rookie）"</option>
                                        <option value="Analyst" selected>"標準（Analyst）"</option>
                                        <option value="PM">"運用のプロ（PM）"</option>
                                        <option value="Distressed">"危機相場（Distressed）"</option>
                                        <option value="Beast">"野獣級"</option>
                                    </select>
                                </label>
                                <button name="action" value="new">
                                    "新しく始める（現在の運用を置換）"
                                </button>
                            </form>
                        </div>
                    </details>
                </main>
                <footer>
                    <span>"オフライン対応・データ送信なし・MIT・LLM開発支援使用"</span>
                    <span>
                        {p.current_meme.clone().filter(|_| p.meme_enabled).unwrap_or_default()}
                    </span>
                    <span>"架空市場のゲーム / 投資助言ではありません"</span>
                    <div class="disclaimer">
                        <p>"ゲーム内の資金・損益は架空です。実際のお金は使用せず、換金もできません。"</p>
                        <details>
                            <summary>"免責事項"</summary>
                            <p>"TDN CREDITは、架空の企業・証券・市場を使用する教育・娯楽目的のゲームです。実在の企業や金融商品とは関係ありません。"</p>
                            <p>"金融モデルはゲーム用に簡略化されています。表示される価格、利回り、格付け、運用成績は現実の市場や将来の収益を示すものではなく、投資助言・勧誘を目的としません。"</p>
                            <p>"実際の資金の入金・運用・払戻しは行いません。ゲーム内の資産や実績に金銭的価値はありません。"</p>
                        </details>
                    </div>
                </footer>
            </body>
        </html>
    }
    .to_html();
    format!("<!doctype html>{html}")
}
