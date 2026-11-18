# Phase 4 log

- 2026-11-17 — Phase 4 started: plan in docs/play/PLAN.md. Downloading the FDN replay data (438 MB) and 17lands' card id list (data/raw/cards.csv).
- 2026-11-17 — Replay data: 791,159 games (one per row, the 17lands user's side). Median 9 turns, max 29; the user wins 55%. Per turn, casts, lands, attackers, blocked/unblocked creatures, blockers, kills, the user's end-of-turn hand and both boards are **card lists** (17lands card ids); life totals, mana spent and the opponent's hand size are numbers. All 595 card ids seen in the first 5,000 games map (via data/raw/cards.csv names) to engine cards: 100% of occurrences. Targets, action order and blocker-to-attacker pairings aren't recorded. Next: P0 parser, per-turn decision records in engine card numbers.
