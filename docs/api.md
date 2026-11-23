# Bolt service API (v1)

The contract between the Bolt service (this repo) and Nerd Leagues. This file is the source of truth;
change it here first. Status: **built and tested** (`src/bolt/service.py`, `tests/test_service.py`), run locally only (2026-11-22).

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
| 409 | `version` mismatch (stale or repeated action), or not that seat's turn | `{"error": "...", "state": <current state>}`: the state is nested under `"state"` |
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
`GET /v1/cards?set=FDN` (returns `{"cards": [{"card_id", "name", "rarity", "colors", "types", "image",
"token"}]}`; `image` is Scryfall's "normal" image URL for hotlinking, or `null` if a card has none),
which includes the basic lands. The list also has the engine's 23 tokens (`"token": true`, `"rarity":
"token"`, plus `power` and `toughness`), with images from Scryfall's FDN token printings (set tfdn). Two
token names repeat with different stats (Cat 1/1 and 2/2, Dragon 4/4 and 5/5), so match game objects on
**`card_id`**, which every card in this list and every object in a game view carries; names alone are
ambiguous for those. Tokens can't be drafted or put in decks. Any name outside the 286 → 422 with
`unknown_names`; nothing is dropped
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
- **Passing priority is an action** (`PassPriority`), offered at every priority point. The service plays
  any decision with exactly one legal move by itself (a forced pass, a forced "done blocking"); every real
  choice comes to you.
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
- When `pending.kind` is `Decide`, `pending.choice` says what it's about, and moves get real labels
  (needs the engine build after 2026-11-23; older builds omit these and label `Yes`/`No`/`Choose mode N`):

  | `choice` | Extra `pending` fields | Move labels |
  |---|---|---|
  | `May` | `source`, `source_text` | "Wary Thespian: yes" / "…: no" (show `source_text` as the prompt) |
  | `Surveil` | `source`, `cards` (decider only: the top card first; one decision per card) | "Put Forest into your graveyard" / "Keep Forest on top" |
  | `Tuck` | `card` (decider only) | "Put Forest on top of your library" / "Put Forest on the bottom" |
  | `MayPay` | `source`, `cost` | "Pay {2} (Card)" / "Don't pay" |
  | `PayWard`, `PayWardLife` | `card` (the targeted permanent), `cost` or `life` | "Pay ward {1} for X" / "Don't pay (X is countered)" |
  | `ChooseColor` | `card` | "Heraldic Banner: choose Red" (`number` 0–4 = White, Blue, Black, Red, Green) |
  | `ChooseCreatureType` | `card`, `options` | "Heraldic Banner: choose Elf" |
  | `SplitPiles`, `ChoosePile` | `card`, `pile_a`, `pile_b` | "Put X in pile A / B", "Choose pile A / B" |
  | `Search`, `LookPick` | `source`, `what` / `cards` (decider only) | "Take X" / "Find nothing", "Take nothing" |
  | `SacrificeCreatures`, `SacrificeAnother`, `KeepOfType`, `GenesisPick`, `PickDiscard` | `left`, `card`, `cards`, `x` | "Sacrifice X", "Keep X", "Put onto the battlefield: X", "Make them discard X"; `No`: "Don't sacrifice", "Done choosing" |
  | `Discard`, `LoseOrDiscard` | `left`, `optional`, `life` | "Discard X"; `No`: "Don't discard" / "Lose N life instead" |
  | `ChooseMode` | `source` | "Choose mode N" |

  `source`, `card` and each entry of `cards`, `pile_a`, `pile_b` are `{"id", "name", "card_id"}`. Yes/No moves
  about one card (surveil, tuck, ward) carry its id in `objects`. FDN has no scry; when Star Trek's scry
  arrives it will be one decision per card, like surveil.
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

### Frames: the board between your decisions (optional)

Add `"frames": true` to `POST /v1/game` or `POST /v1/game/{id}/act`, and the response gains
`"frames": [state, ...]`: the human seat's view after your own move (lands tapped, spell cast), then
after each move the bot or a forced move makes until your next decision, through steps and the next
turn's untap. Each frame has the same shape as a state, with `legal_actions` empty and an `event_n`:
the log index (`n`) of the next event at that moment, so a recap can be synced to the frames.
A new frame is taken whenever the view or the turn changes (combat damage, a land, a cast, an untap); a
step change with nothing visible changing doesn't make one, though each frame carries its `step`; at most 40 are
returned (the first 20 and the last 20 when there are more). The final state is the usual response
body, not a frame. Without `"frames"`, responses are unchanged.

### `GET /v1/game/{game_id}/log?since=0` → events

```json
{"events": [
  {"n": 0, "turn": 1, "kind": "Action", "seat": 0, "move": "Keep", "label": "Keep hand"},
  {"n": 1, "turn": 1, "kind": "TurnBegan", "player": 0},
  {"n": 2, "turn": 1, "kind": "Action", "seat": 0, "move": "PlayLand", "label": "Play Plains"},
  {"n": 3, "turn": 1, "kind": "Moved", "id": 12, "name": "Plains", "owner": 0, "from": "Hand", "to": "Battlefield"},
  {"n": 4, "turn": 2, "kind": "Drew", "player": 1},
  {"n": 5, "turn": 2, "kind": "Cast", "id": 51, "name": "Shock"},
  {"n": 6, "turn": 2, "kind": "Damage", "id": 51, "name": "Shock", "target_player": 0, "amount": 2},
  {"n": 7, "turn": 2, "kind": "LifeLost", "player": 0, "amount": 2}
], "next": 8}
```

Ordered and append-only from the human seat's point of view; `since` returns only newer events, and
`next` is the index to ask for next time. Each `Action` (one decision, either seat) is followed by the
engine events it caused:

| `kind` | Fields |
|---|---|
| `Action` | `seat`, `move` (a legal-action kind), `label` (the bot's hidden choices read "Puts a card on the bottom" / "Chooses a card") |
| `TurnBegan` | `player` (whose turn) |
| `Drew` | `player`; `id`, `name` only when the human drew it |
| `Moved` | `owner`, `from`, `to` (zones: `Library`, `Hand`, `Battlefield`, `Graveyard`, `Stack`, `Exile`, `Gone` for tokens that left); `id`, `name` unless the move is hidden from the human (the bot's library and hand) |
| `Cast`, `Resolved`, `Triggered`, `Fizzled` | `id`, `name` (the spell or the source) |
| `Damage` | `id`, `name` (source), `amount`, and `target_player` or `target`/`target_name` |
| `LifeGained`, `LifeLost` | `player`, `amount` |
| `Lost` | `player` (lost the game) |

The opening hands aren't logged (only the mulligan decisions), and a runaway game's engine events stop
being recorded after 200,000.

### `DELETE /v1/game/{game_id}`

Ends and frees a game.

### Limits

- At most 20 live games (configurable); a game idle for 6 hours is dropped (404 afterwards).
- Games longer than 200 turns, or 20,000 decisions, end as `"timeout"`.
- Bot moves: `network`, `smart` and `rules` take milliseconds per decision; `smart_strong` takes longer (tens of milliseconds, more on big boards).

## Versioning

Breaking changes get `/v2`; additive fields can appear in v1 at any time, so ignore fields you don't
know.
