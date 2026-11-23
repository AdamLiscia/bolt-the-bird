# Bolt service API (v1)

The contract between the Bolt service (this repo) and Nerd Leagues. This file is the source of truth;
change it here first. Status: **contract only — the service is not built yet** (2026-11-22).

Scope: Foundations (FDN) only. Draft against Bolt, get a bot-drafted deck, and play a game against
Bolt. Admin-only on the Nerd Leagues side (NL owns sign-in, gating, the UI and card images). Bolt never
touches the NL database; NL never imports Bolt code. Star Trek (1v1 Commander) comes later as a new
`set` value.

## Service

- FastAPI, one process, `127.0.0.1:8766` (models load once; torch is ~300 MB of RAM).
- Every request carries `X-Bolt-Key: <secret>` (from the env var `BOLT_KEY`). Missing or wrong → 401.
- All paths are under `/v1`; each is also served without the prefix (e.g. `/draft/picks`), for clients
  already written that way. Bodies and responses are JSON.
- Requests that touch the models are served one at a time (a lock), so callers should expect queuing,
  not parallelism.

### Errors

| Status | When | Body |
|---|---|---|
| 401 | missing or wrong `X-Bolt-Key` | `{"error": "unauthorized"}` |
| 404 | unknown or expired `game_id` | `{"error": "no such game"}` |
| 409 | `version` mismatch (stale or repeated action), or not that seat's turn | `{"error": "...", "state": <current state>}` |
| 422 | unknown card names, bad deck, invalid action index, unsupported set | `{"error": "...", "unknown_names": [...]}` (names when relevant) |
| 429 | too many live games | `{"error": "too many games", "limit": N}` |

### `GET /v1/health`

```json
{"ok": true, "api": "v1", "sets": ["FDN"],
 "models": {"drafter": "bot_ppo.pt", "human_drafter": "human_model.pt", "player": "play_v2_ppo3_i850.pt"},
 "engine": "bolt-engine 0.1.0", "games_live": 3, "games_limit": 20}
```

## Card identity

Exact English card names as Scryfall spells them (front face for double-faced cards), e.g.
`"Savannah Lions"`, `"Llanowar Elves"`. The FDN vocabulary is the 286 cards in
`GET /v1/cards?set=FDN` (returns `{"cards": [{"name", "rarity", "colors", "mana_value", "type_line"}]}`),
which includes the basic lands. Any name outside it → 422 with `unknown_names`; nothing is dropped
silently. Inside a game, objects carry both an object `id` and the card `name` (see below).

## Drafting

FDN drafts are 3 packs × 14 picks (42). `pack_number` is 1–3 and `pick_number` is 1–14.

### `POST /v1/draft/pick` (stateless)

```json
{"set": "FDN", "pack": ["Savannah Lions", "..."], "pool": ["..."], "pack_number": 1, "pick_number": 3,
 "bot": "bolt"}
```

- `bot`: `"bolt"` (the trained drafter, `bot_ppo.pt`) or `"human"` (the imitation model, drafts like a
  17lands player; use it for the other bot seats).
- `pack` must be non-empty; duplicates are fine (they're separate copies).

Response: every pack card, best first. `score` is the model's probability of that pick (sums to 1 over
the pack; duplicates share one card's probability, listed once with `count`).

```json
{"ranked": [{"name": "Savannah Lions", "score": 0.41, "count": 1}, {"name": "...", "score": 0.12, "count": 2}]}
```

### `POST /v1/draft/picks` (stateless, batched; what Nerd Leagues' bot loop calls)

One request per draft step, for every bot seat at once (they share one forward pass, so 7 seats take
about as long as one: well under NL's 2 s timeout).

```json
{"set": "FDN", "bot": "bolt",
 "seats": [{"seat": 2, "pack": ["..."], "pool": ["..."], "pack_number": 1, "pick_number": 1}]}
```

- `pack_number` 1–3 and `pick_number` 1–14 (within the pack), both 1-based; all seats are normally on the
  same pick, but each seat's numbers are used as given.
- `bot` applies to every seat: `"bolt"` or `"human"`.

Response: one entry per seat, in request order. `name` is byte-identical to a name in that seat's
`pack` (the request's own string is returned). `ranked` is the same list `/draft/pick` returns.

```json
{"picks": [{"seat": 2, "name": "Savannah Lions", "ranked": [{"name": "Savannah Lions", "score": 0.41, "count": 1}]}],
 "model": "bot_ppo.pt"}
```

Any unknown name in any seat → 422 for the whole batch, listing every offending name (NL then uses its
own fallback for that step).

### `POST /v1/deck/build`

```json
{"set": "FDN", "pool": ["...42 names..."]}
```

Builds a 40-card deck with Bolt's deck builder (tries all ten color pairs, adds spells by judged
win rate, 16 or 17 lands). Response:

```json
{"main": [{"name": "Savannah Lions", "qty": 2}], "lands": {"W": 8, "R": 9},
 "colors": "WR", "cards": 40, "judged_win_rate": 0.58}
```

`main` holds the 23–24 spells and any nonbasic lands; `lands` the basics by color. `judged_win_rate` is
the judge's estimate (a model, not played games); NL may hide it.

### `POST /v1/deck/random`

```json
{"set": "FDN", "seed": 1234, "seat": 0, "drafters": "human"}
```

Runs a full 8-seat draft with real FDN packs: seat `seat` is drafted by Bolt (`bot_ppo`), the other 7 by
`drafters` (`"human"` default, or `"bolt"`). For "skip the draft" mode. Same seed → same draft.
Response: `{"picks": [{"pack_number", "pick_number", "name"}] (42), "deck": <as /deck/build>}`.

## Games

One game is one human seat against one bot seat, 20 life, 40-card decks (at least 40, FDN names only,
any number of copies, as in Limited). The engine plays every FDN card's rules; the
service keeps game state in memory.

### Things the game screen must know about how the engine works

- **Mana is paid automatically.** There are no mana-tapping decisions: a spell or ability shows up in
  `legal_actions` only when it can be paid for, and choosing it taps the lands. `pool` in the view is
  floating mana (rarely non-zero).
- **One decision at a time.** Attacks are declared one creature per action (`Attack`), then
  `FinishAttacks`. Blocks are one pair per action (`Block`: blocker → attacker), with `ResetBlocks`
  and `FinishBlocks`. Targets are chosen as a separate decision after casting (`pending.kind =
  "ChooseTarget"`), and so are yes/no choices (`"Decide"`), amounts (`"Divide"`), discards and
  mulligans.
- **Passing priority is an action** (`PassPriority`), offered at every priority point. The service
  auto-passes points where passing is the only legal action; it does not auto-pass anything else.
- **Object ids** are integers, unique within a game, and **stable across zone changes** (a creature that
  dies keeps its id in the graveyard). Tokens get new ids. Ids are meaningless across games.
- **Hidden information.** A seat's view shows its own hand, both battlefields, both graveyards, exile,
  the stack, and the opponent's hand *count*, life and library count. When a choice involves hidden
  cards (searching your library), those cards appear only in that seat's `legal_actions`.

### `POST /v1/game`

```json
{"set": "FDN", "deck_a": [{"name": "Savannah Lions", "qty": 2}], "deck_b": [{"name": "Plains", "qty": 8}],
 "human_seat": 0, "pilot": "network", "seed": 99, "first": 0}
```

- `deck_a` is seat 0's deck, `deck_b` seat 1's. `first`: 0 or 1, who plays first (default random from
  `seed`).
- `pilot` (the bot's player): `"network"` (the trained player, strongest per move, fast), `"smart"`
  (the engine's smart player, 1 × 2), `"smart_strong"` (4 × 6: slower; `network` beats it 55% head to head), or
  `"rules"` (rules-based, weakest).

Response: `{"game_id": "g_8f3a...", "state": <state for human_seat>}`. The bot has already acted up
to the human's first decision.

### `GET /v1/game/{game_id}?seat=N` → state

```json
{
  "game_id": "g_8f3a", "version": 17, "seat": 0,
  "turn": 3, "step": "Main1", "active": 0, "to_act": 0,
  "pending": {"kind": "Priority", "player": 0},
  "over": false, "result": null,
  "view": {
    "players": [
      {"seat": 0, "life": 20, "hand_size": 5, "library": 30, "lands_played": 1, "pool": 0,
       "hand": [{"id": 12, "name": "Savannah Lions"}],
       "graveyard": [{"id": 7, "name": "Shock"}]},
      {"seat": 1, "life": 18, "hand_size": 6, "library": 31, "lands_played": 0, "pool": 0,
       "hand": null, "graveyard": []}
    ],
    "battlefield": [
      {"id": 51, "name": "Llanowar Elves", "controller": 1, "owner": 1, "tapped": false,
       "creature": true, "land": false, "power": 1, "toughness": 1, "damage": 0, "counters": 0,
       "loyalty": 0, "attached_to": null, "entered_turn": 2, "entered_this_turn": false}
    ],
    "stack": [{"id": 60, "name": "Shock", "controller": 1}],
    "exile": [],
    "combat": {"attackers": [51], "blocks": [[12, 51]]}
  },
  "legal_actions": [
    {"index": 0, "kind": "PassPriority", "label": "Pass", "objects": [], "cards": []},
    {"index": 1, "kind": "Cast", "label": "Cast Savannah Lions", "objects": [12], "cards": ["Savannah Lions"]},
    {"index": 2, "kind": "Target", "label": "Target Llanowar Elves", "objects": [51], "cards": ["Llanowar Elves"]},
    {"index": 3, "kind": "TargetPlayer", "label": "Target opponent", "objects": [], "cards": [], "player": 1},
    {"index": 4, "kind": "CastX", "label": "Cast Fireball with X = 3", "objects": [33], "cards": ["Fireball"], "number": 3}
  ]
}
```

- `version` increases with every action; send it back with `act`.
- `to_act` is the seat that must decide; it is the human's seat unless `over`.
- `pending.kind`: `Mulligan`, `Bottom`, `Priority`, `DeclareAttackers`, `DeclareBlockers`, `Discard`,
  `ChooseTarget`, `Decide`, `Divide`, `RemoveCounters`, `FreeCast`, `GameOver`.
- `step`: `PreGame`, `Untap`, `Upkeep`, `Draw`, `Main1`, `BeginCombat`, `DeclareAttackers`,
  `DeclareBlockers`, `FirstStrikeDamage`, `CombatDamage`, `EndCombat`, `Main2`, `End`, `Cleanup`.
- `legal_actions[].kind` is one of the engine's move kinds: `Keep`, `Mulligan`, `Bottom`,
  `PassPriority`, `PlayLand`, `Cast`, `CastKicked`, `CastWith`, `CastFlashback`, `CastModal`,
  `CastAlternative`, `CastX`, `CastFromGraveyard`, `CastFromGraveyardAs`, `Activate`, `ActivateWith`,
  `ActivateX`, `Loyalty`, `Attack`, `AttackPlaneswalker`, `FinishAttacks`, `Block`, `ResetBlocks`,
  `FinishBlocks`, `Target`, `TargetPlayer`, `Yes`, `No`, `Pick`, `Mode`, `Amount`, `Discard`.
  `objects` lists the object ids the move names, in order (e.g. `Block`: `[blocker, attacker]`);
  `cards` the matching names; `number` the amount, mode or X where there is one; `player` the
  targeted seat for `TargetPlayer`. `label` is a ready-to-show English string; NL may build its own.
- `result`, when `over`: `{"winner": 0 | 1 | null, "outcome": "win" | "draw" | "timeout"}` (the engine reports who won, not why; the log shows the last life change or draw).

### `POST /v1/game/{game_id}/act` → state

```json
{"seat": 0, "action_index": 1, "version": 17}
```

Applies the human's move, then lets the bot act (and auto-passes forced passes) until the human must
decide again or the game ends. Returns the new state. Wrong `version` or not that seat's turn → 409 with
the current state; an index outside `legal_actions` → 422.

### `GET /v1/game/{game_id}/log?since=0` → events

```json
{"events": [
  {"n": 0, "turn": 1, "kind": "TurnBegan", "player": 0},
  {"n": 1, "turn": 1, "kind": "Moved", "id": 12, "name": "Savannah Lions", "from": "Hand", "to": "Battlefield"},
  {"n": 2, "turn": 2, "kind": "Damage", "source": 51, "target": {"player": 0}, "amount": 1},
  {"n": 3, "turn": 2, "kind": "LifeLost", "player": 0, "amount": 1}
], "next": 4}
```

Ordered and append-only; `since` returns only newer events. Kinds come from the engine: `TurnBegan`,
`Drew`, `Moved`, `Cast`, `Resolved`, `Fizzled`, `Triggered`, `Damage`, `LifeGained`, `LifeLost`, `Lost`,
`GameOver`, plus `Action` entries for each decision (seat, kind, label), so the log alone can replay a
game. A `Drew` event in the opponent's view carries no card name.

### `DELETE /v1/game/{game_id}`

Ends and frees a game.

### Limits

- At most 20 live games (configurable); a game idle for 6 hours is dropped (404 afterwards).
- Games longer than 200 turns, or 20,000 decisions, end as `"timeout"`.
- Bot moves: `network`, `smart` and `rules` take milliseconds per decision; `smart_strong` takes longer (tens of milliseconds, more on big boards).

## Versioning

Breaking changes get `/v2`; additive fields can appear in v1 at any time, so ignore fields you don't
know.
