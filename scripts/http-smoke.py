#!/usr/bin/env python3
"""Destructive to the test session: use a dedicated portable save directory."""
import json
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

base = (sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:3000").rstrip("/")


def page(path="/"):
    return urllib.request.urlopen(base + path, timeout=30).read().decode()


def credentials():
    html = page()
    return {"token": re.search(r'name="token" value="([^"]+)"', html)[1],
            "revision": re.search(r'name="revision" value="([^"]+)"', html)[1]}


def command(action, **fields):
    data = {**credentials(), "action": action, **fields}
    req = urllib.request.Request(base + "/command", urllib.parse.urlencode(data).encode(),
                                 headers={"Origin": base})
    return urllib.request.urlopen(req, timeout=30).read().decode()


def snapshot():
    return json.loads(page("/export"))


for tab in ["market", "portfolio", "issuers", "news", "analysis"]:
    assert "TDN CREDIT" in page("/?tab=" + tab)

command("new", seed=1919, difficulty="Analyst")
initial = snapshot()["game"]
assert initial["portfolio"]["cash"] == 6_500_000_000 and not initial["portfolio"]["positions"]
assert len(initial["issuers"]) == 10 and len(initial["bonds"]) == 20

# CSRF rejection must not touch the market or cash.
try:
    urllib.request.urlopen(urllib.request.Request(base + "/command",
                           urllib.parse.urlencode({"token": "wrong", "revision": 0, "action": "next"}).encode()))
    raise AssertionError("invalid token was accepted")
except urllib.error.HTTPError as error:
    assert error.code == 403
assert snapshot()["game"] == initial

stale = credentials()
command("buy", bond="TDN-1", amount=1_000_000_000)
bought = snapshot()["game"]
assert bought["portfolio"]["positions"]["TDN-1"]["face"] == 1_000_000_000
assert bought["trades"][-1]["side"] == "Buy"
request = urllib.request.Request(base + "/command", urllib.parse.urlencode({**stale, "action": "next"}).encode())
assert "古い画面" in urllib.request.urlopen(request).read().decode()
assert snapshot()["game"]["turn"] == 0

command("sell", bond="TDN-1", amount=100_000_000)
assert snapshot()["game"]["portfolio"]["positions"]["TDN-1"]["face"] == 900_000_000
command("save")
saved = snapshot()
command("next")
command("load")
assert snapshot() == saved

if snapshot()["presentation"]["meme_enabled"]:
    command("meme")
command("new", seed=1919, difficulty="Analyst")
assert not snapshot()["presentation"]["meme_enabled"], "New Game reset presentation preference"

# Full market path identical with memes on/off. Trades are identical in both runs.
paths = []
for memes in [True, False]:
    command("new", seed=1919, difficulty="Analyst")
    if snapshot()["presentation"]["meme_enabled"] != memes:
        command("meme")
    for bond in ["TDN-1", "TNOK-1", "YJSN-1", "DB-2"]:
        command("buy", bond=bond, amount=1_000_000_000)
    for month in range(1, 25):
        html = command("next")
        save = snapshot()
        assert save["game"]["turn"] == month
        assert abs(save["game"]["history"][-1]["attribution"]["residual"]) <= 20
        if not memes:
            assert save["presentation"]["current_meme"] is None
    paths.append(snapshot()["game"])
    assert "24か月時点の運用結果" in html
    assert snapshot()["game"]["turn"] == 24
assert paths[0] == paths[1], "presentation changed financial scenario"
print("PASS: five pages, universe, CSRF, stale orders, Ask/Bid trading, save/load, 24 turns, final score, meme-independent replay")
