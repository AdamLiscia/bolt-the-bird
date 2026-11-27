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
 "features": ["frames", "tokens", "card_id", "human_seats", "wait", "concede", "decision_details"],
 "engine": "bolt-engine 0.1.0", "games_live": 3, "games_limit": 20}
```

`features` lists what this build supports; `decision_details` (pending.choice and the readable
decision labels) appears only once the engine build that reports decisions is installed.

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
{"set": "FDN", "seed": 1234, "drafter": "human", "drafters": "human", "seat": 0, "colors": ["B"]}
```

Runs a full 8-seat draft with real FDN packs and returns one seat's 42 picks and built deck, for "skip
the draft" mode. Same request and seed → same result.

- `drafter` (seat 0): `"bolt"` (default; the strongest drafter, but it leans green-white: 34 of 40 deals) or
  `"human"` (the imitation model, sampling its picks the way people vary: all 10 color pairs, none more
  than 7 of 40).
- `drafters` (the other 7 seats): `"human"` (default) or `"bolt"`.
- `seat`: which seat's deck to return, 0–7 or `"random"` (default 0, the `drafter`'s).
- `colors`: an optional soft preference, one color or a pair: the returned deck is whichever of the 8
  (or the chosen seat's, if it matches) shares the most of those colors. With `["B"]`, 20 of 20 deals
  were black.

Response: `{"seed", "seat", "drafter" (who drafted that seat), "picks": [{"pack_number", "pick_number",
"name"}] (42), "deck": <as /deck/build>, "colors_matched": true | false | null}`.

### Drafting against 7 bots

`POST /v1/draft {"set": "FDN", "seed"?, "human_seat": 0, "bots": "mix"}` → `{"draft_id", "seed", "state"}`.
`bots`: `"human"` (7 human-style seats), `"bolt"` (7 Bolts) or `"mix"` (default: seat 1 Bolt, seats 2–7
human-style, like Bolt's own test tables). The person is always seat 0. Kept in memory for 6 hours idle,
at most 50 drafts (the oldest is dropped).

State (`GET /v1/draft/{id}`, and every pick's response), the person's seat only:

```json
{"draft_id": "d_…", "version": 3, "done": false, "picks_left": 39, "pack_number": 1, "pick_number": 4,
 "pack": [{"id": 0, "name": "Savannah Lions", "card_id": 210}],
 "pool": [{"name": "…", "card_id": 17}]}
```

`pack[].id` is the card's position in the pack. When `done`, `pack` is empty and `pool` holds the 42 picks
(in pick order): pass their names to `/v1/deck/build` for the Auto-build button, then to `/v1/game`.

`POST /v1/draft/{id}/pick {"card_id": 210, "version": 3}` (or `"id"`: the position): the bots pick at the
same time, packs pass, and the next state comes back. A stale or repeated `version` → 409 with the
current state; a card not in your pack → 422.

`GET /v1/draft/{id}/table`, once the draft is done (409 before): `{"seats": [{"seat": 1–7, "drafter",
"picks", "deck" (as /deck/build), "colors", "highlights"}]}`: what the bots took and their decks, to
choose an opponent.

### Choosing from a whole bot table

`POST /v1/draft/table {"set": "FDN", "seed"?, "drafters": "mix"}` → `{"seed", "seats": [{"seat": 0–7,
"drafter", "colors", "cards", "deck" (as /deck/build, with `judged_win_rate`), "picks", "highlights"}]}`.
`drafters`: `"mix"` (seat 0 Bolt, seats 1–7 human-style), `"human"` or `"bolt"`. `highlights` are the deck's
three standouts (rarest first, then the earliest picked). Same seed → same table.

`/v1/deck/build` takes any pool of FDN card names (any draft: NL's Battlefield drafts too), as long as
it holds enough playables for 40 cards (basics are added).

## Games

A game is one human seat against one bot seat, or two people with no bot (`human_seats: [0, 1]`, below); 20 life, 40-card decks (at least 40, FDN names only,
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
       "loyalty": 0, "attached_to": null, "entered_turn": 2, "entered_this_turn": false,
       "battle": false, "defense": 0, "protector": null}
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
- A battle (`battle` true) shows its defense counters in `defense` and its `protector` (a seat). It's attacked
  with an `AttackPlaneswalker` move, labeled "Attack the battle ...".
- `to_act` is the seat that must decide; it is the human's seat unless `over`.
- When `pending.kind` is `Bottom` (London mulligan), `pending.left` is how many cards are still to go on the
  bottom.
- When `pending.kind` is `ChooseTarget` (or `Divide`), `pending.source` (`{"id", "name", "card_id"}`),
  `pending.source_text` and `pending.spell` say which spell or ability the targets are for (it isn't on
  the stack until its targets are chosen). Needs the engine build after 2026-11-23.
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
| `Action` | `seat`, `move` (a legal-action kind), `label`, `objects` and `cards` (the ids and names the move named: targets, attackers, blocker and attacker, the card cast), `player` for `TargetPlayer`. The other seat's hidden choices read "Puts a card on the bottom" / "Chooses a card" with empty `objects` |
| `TurnBegan` | `player` (whose turn) |
| `Drew` | `player`; `id`, `name` only when the human drew it |
| `Moved` | `owner`, `from`, `to` (zones: `Library`, `Hand`, `Battlefield`, `Graveyard`, `Stack`, `Exile`, `Gone` for tokens that left); `id`, `name` unless the move is hidden from the human (the bot's library and hand) |
| `Cast`, `Resolved`, `Triggered`, `Fizzled` | `id`, `name` (the spell or the source) |
| `Damage` | `id`, `name` (source), `amount`, and `target_player` or `target`/`target_name` |
| `LifeGained`, `LifeLost` | `player`, `amount` |
| `Lost` | `player` (lost the game) |

The opening hands aren't logged (only the mulligan decisions), and a runaway game's engine events stop
being recorded after 200,000.

### Two people at one game

`POST /v1/game` with `"human_seats": [0, 1]` (and no `pilot`) makes a game the engine never plays for;
forced single-move decisions still play themselves. The response adds `"human_seats": [0, 1]`. Then:

- `GET /v1/game/{id}?seat=N`: either seat's state, its own hidden information only. The seat that isn't
  deciding gets `legal_actions: []`.
- `POST /v1/game/{id}/act` is accepted only from the seat that must decide (`to_act`) with the current
  `version`; anything else is a 409 whose `state` is that seat's view. (In a game against the bot,
  acting as the bot's seat is a 409 too.)
- `GET /v1/game/{id}/frames?seat=N&since=V` → `{"frames": [...], "version"}`: that seat's boards after
  version `V` (the same frames as above, at most 40, deduplicated, from the last 300 kept per seat), so
  the waiting player can play back the other's turn.
- `GET /v1/game/{id}/log?seat=N&since=…`: the log as seat N may see it (the other's draws and hidden
  moves without names; their hidden choices masked). `seat` defaults to the first human seat.
- `GET /v1/game/{id}/version` → `{"version", "to_act", "over"}`, cheap.
- `GET /v1/game/{id}/wait?version=V&timeout=25` → the same plus `"changed"`: a long-poll that returns as soon
  as the version differs from `V` or the game ends, else after `timeout` seconds (at most 30).
- `POST /v1/game/{id}/concede {"seat": N}`: ends the game; `result` is
  `{"winner": 1 - N, "outcome": "concede", "conceded": N}`, and the log gets a `Conceded` entry. Works in
  bot games too.

### `DELETE /v1/game/{game_id}`

Ends and frees a game.

### Limits

- At most 20 live games (configurable); a game idle for 6 hours is dropped (404 afterwards).
- Games longer than 200 turns, or 20,000 decisions, end as `"timeout"`.
- Bot moves: `network`, `smart` and `rules` take milliseconds per decision; `smart_strong` takes longer (tens of milliseconds, more on big boards).


### Paying mana

Every service game uses the **smart auto-payer** for both seats: floating mana is spent first, colored
costs take the narrowest sources, and generic costs take the sources whose colors the rest of the hand
(and your permanents' activated abilities) need least, so an Island stays up for the blue card still in
hand. Research runs keep the engine's older payer, so stored results replay identically.

**Manual mana** (human seats): `POST /v1/game {..., "manual_mana_seats": [0]}` (default `[]`), or switch
any time with `POST /v1/game/{id}/mana {"seat": 0, "manual": true|false}` (returns the state; the version
bumps because the legal moves change; 409 for a seat that isn't human). While a manual seat has
priority, `legal_actions` also offer:

```json
{"index": 7, "kind": "TapForMana", "label": "Tap Forest for {G}", "objects": [41], "cards": ["Forest"], "color": "G"}
```

- One move per source and color it can make: a dual land offers two, Evolving Wilds none (not a mana
  ability). `color` is one of `W U B R G C`. Treasure reads "Sacrifice Treasure for {R}". Mana creatures
  are offered when they can tap. Sources whose mana is restricted (only for some spells) stay automatic.
- The mana floats in the pool until the step ends (CR 500.4). Casting or activating spends floating mana
  first and auto-pays the rest from untapped sources, so Manual never blocks a cast: cast moves are legal
  whenever pool plus untapped sources can pay.
- Tap moves alone aren't a decision: when a manual seat could only pass or tap, the pass is forced as
  before.
- `view.players[n].pool` is the total floating mana; `pool_colors` is
  `{"W", "U", "B", "R", "G", "C", "any"}` counts ("any": mana that can be several colors).
- Bots never get TapForMana, and auto seats never see it.

### Player counters (names reserved; added as the engine models them)

For stream overlays. A field appears only once the engine models it, so a missing field means "not in this
game"; show an icon only when present and active.

| Field | Where | Status |
|---|---|---|
| `commander_damage` | per player: a list of `{id, name, card_id, owner, damage}`, one per commander object (see *Commander games*) | Ships with Commander games (see *Commander games* below, Bloomburrow first); the engine already tracks it, Limited games omit it |
| `poison`, `energy`, `experience` | per player, integers | Not modeled (no Foundations or Tarkir card uses them) |
| `monarch`, `initiative` | game level: the seat that has it, or null | Not modeled |
| `city_blessing` | game level: list of seats | Not modeled |
| `day_night` | game level: `"day"`, `"night"` or null | Not modeled |

### The Tarkir service (`set: "TDM"`, 127.0.0.1:8767)

Status (2026-11-25): **running, for admin-vs-admin Tarkir games.** A second process with the same key and
the same endpoints, on the tarkir engine; 8766 keeps serving FDN. `GET /v1/health` reports
`"sets": ["TDM"]`, so route by set.

- `GET /v1/cards?set=TDM`: the 286 cards plus 40 tokens, same fields. Card `image` is Scryfall's
  by-name image link (it redirects to the card image, so it hotlinks like FDN's); tokens have `image: null`.
- `POST /v1/deck/random {"set": "TDM", ...}`: drafted by the Tarkir drafter (trained on 17lands human
  picks; `"bolt"` and `"human"` are the same model for now) from real Tarkir opening packs. Decks are built
  by a simple rule (the two colors with the most playable cards, rares first, basics by pips); no judge,
  so `judged_win_rate` is `null`. `deck.lands` is keyed by colour (`{"B": 9, "G": 8}`), as for FDN.
- `POST /v1/game {"set": "TDM", "human_seats": [0, 1], ...}`: every game endpoint works as for FDN. A
  game with a bot seat is 422 `"TDM bot play isn't trained yet: use human_seats [0, 1]"`.
- New decisions, with readable labels: `ChooseCastX` and `ChooseDelve` (answered with `Amount` moves),
  `EnterAsCopy` (`Pick` a creature or `No`), `PayWardDiscard` (`Discard` a card or `No`, then it's
  countered).

### The Commander service (`set: "CMD"`, 127.0.0.1:8768)

Status (2026-11-25): **for admin-vs-admin Commander games with real decks.** The card pool is every
card the engine knows across sets (`GET /v1/cards?set=CMD`: FDN, TDM, BLC and the playgroup decks'
cards, 3,031 plus tokens); a card plays only once it's scripted. No drafting (422), no bot (422).

`POST /v1/deck/validate`:

```json
{"format": "commander", "commander": "Xenagos, God of Revels", "deck": [{"name": "Forest", "qty": 30}, ...]}
```

returns `{"ok": bool, "problems": [...], "missing": [...]}`. `problems` are the deck rules (one or two
commanders, 100 cards with them, singleton, color identity) and names not in the pool; `missing` are the cards
the engine doesn't play yet, names outside the pool included (those also appear in `problems`). `ok` means the deck can be played now. A listed commander is
counted once.

`POST /v1/game` with `"set": "CMD", "format": "commander"`, `commander_a`/`commander_b` (a name or a
list of two), `deck_a`/`deck_b`, `"human_seats": [0, 1]`, and optionally `starting_life` (default 40) and
`auto_command_zone` (`[bool, bool]`). It's refused with 422 and the seat if a deck has `missing` cards
(`{"error": "cards the engine doesn't play yet", "seat": 0, "missing": [...]}`) or `problems`
(`{"error": "invalid_deck", "seat": 0, "problems": [...]}`). Every game endpoint works as for FDN.

The view adds, per player, `command` (`[{"id", "name", "card_id", "tax", "commander", "owner"}]`) and
`commander_damage` (`[{"id", "name", "card_id", "owner", "damage"}]`), and `"commander": true` with
`"owner"` on commander objects in any zone. A commander going to a graveyard or exile asks its owner
`pending.kind = "CommanderToCommandZone"` (Yes moves it; No leaves it), unless that seat's
`auto_command_zone` is on.

### Commander games (Bloomburrow Commander, `set: "BLC"`): planned contract

Status (2026-11-25): **the engine rules are built and tested on the `tarkir` branch; the service does not
serve this yet.** It reaches the live service together with Tarkir, after the trained players are padded
for the new cards. Everything below is additive: Limited games are unchanged and ignore it.

Request: `POST /v1/game` with `"format": "commander"` and a commander per seat. Decks are the other 99 cards.

```json
{"set": "BLC", "format": "commander",
 "commander_a": "Bello, Bard of the Brambles", "deck_a": [{"name": "Sol Ring", "qty": 1}, ...],
 "commander_b": "...", "deck_b": [...], "human_seat": 0, "pilot": "smart", "seed": 7,
 "starting_life": 40, "auto_command_zone": [false, true]}
```

- `commander_a` / `commander_b`: a name **or a list of names** (partners, for later sets; BLC has none).
  The deck is the other cards, so with two commanders it is 98.
- `starting_life`: optional, default 40 (Commander's rule). Duel-style 1v1 tables often use 20–30.
- `auto_command_zone`: optional, per seat, default `false` for a human seat. `true` answers the
  command-zone choice (below) with "move it" automatically, so it never prompts. Bot seats always move
  their commander unless a later version says otherwise.

- Validation runs before the game exists. Any problem returns 422 `{"error": "invalid_deck", "seat": 0,
  "problems": ["..."]}` listing every problem, not just the first. The checks: the commander is a
  legendary creature; the deck plus the commander is exactly 100 cards; it's singleton (basic lands, and
  cards that say any number may be in a deck, are exempt); every card is inside the commander's color
  identity (mana symbols in the cost and rules text, both halves of an Omen card).
- Both players start at **40 life**. Commanders start in the command zone.
- Casting a commander from the command zone is an ordinary `Cast` legal action; the cost includes
  **commander tax**, {2} more for each earlier cast from the command zone.
- When a commander would go to the graveyard or exile, it goes there; its owner is then asked a
  yes/no decision, `pending.kind = "CommanderToCommandZone"` (labels "Move to the command zone" /
  "Leave it"), answered with the usual `act`. Hand or library: it goes to the command zone
  automatically. "Dies" triggers see a commander die.
- 21 or more combat damage from one commander makes that player lose.

State additions (per player in `view.players[n]`):

| Field | Meaning |
|---|---|
| `command` | `[{"id", "name", "card_id", "tax"}]`: the cards in that player's command zone; `tax` is the generic mana the next cast adds |
| `commander_damage` | `[{"id", "name", "card_id", "owner", "damage"}]`: combat damage this player took from each commander, one entry per commander **object** (`id`), so a mirror match (both seats on Bello) stays two entries; `owner` is the seat that commander belongs to |
| `life` | starts at `starting_life` (40 by default) |

Every game object that is a commander also carries `"commander": true` and `"owner": <seat>`, in every
zone the view shows (battlefield, stack, graveyard, exile, command), so the page can badge it without
matching names.

**Double-faced cards** (2026-11-25; Ojer Kaslem, Vorinclex). A transforming card keeps its one
`card_id` and its full `"Front // Back"` name on both faces. Additive fields:

- `GET /v1/cards`: a double-faced card's entry also has `back_name` (the back face's name) and
  `back_image` (Scryfall's by-name image link with `face=back`).
- Every view object of a double-faced card, in every zone, has `face` (`"front"`, or `"back"` while
  a permanent is transformed) and `back` (`{"name", "image"}`). Off the battlefield it's always
  `"front"`; a transformed permanent that leaves turns front face up.

**Changing targets** (Deflecting Swat: "You may choose new targets for target spell or ability").
`pending.choice` `ChooseNewTarget`, asked once per target of that spell or ability that has another
legal target, with `card` the spell or ability's source. Answered with the usual moves: `Target` /
`TargetPlayer` labelled "Change the target of <X> to <Y>", or `No`, "Keep the targets". An ability on
the stack is targeted through its source: a `Target` move naming the source object means that
source's topmost ability on the stack. `PutPicksOntoBattlefield` (Ojer Kaslem's revealed cards, The
Grand Evolution's milled ones): `cards`, `left`; `Pick` moves "Put onto the battlefield: X", `No`
"Done choosing".

**Face-down permanents** (2026-11-25; manifest dread, Zimone). A face-down permanent is a 2/2
creature with no name. Battlefield objects carry `face_down` (true/false). The controller sees the
card behind it as usual; for the other seat its `name` is `"Face-down creature"` and `card_id` is
`null`, everywhere the view or a move names it (moves keep the object `id`). The event log doesn't
name a card while it's face down. Turning a manifested creature card face up for its mana cost is an
`Activate` move on it (ability 1; it doesn't use the stack). New decisions: `ManifestDread` (`Pick`
which of the top two to manifest) and `TurnFaceUp` (`Pick` a face-down permanent, or `No`).

`GET /v1/cards?set=BLC` lists the 300 non-basic cards of the four precons plus their tokens, with the
same fields as FDN. `GET /v1/precons?set=BLC` lists the four decks (name, commander, 99 cards) as the
handoff file has them, so Nerd Leagues can offer "play a precon" without building decks itself.

### Spectating a seat (stream mode)

`GET /v1/game/{id}?seat=N&spectate=1` returns seat N's view, the bot's seat included, read-only:
`legal_actions` is `[]` and `"spectate": true`. `view.players[N].hand` holds that seat's cards
(`id`, `name`, `card_id`); the other seat's hand stays hidden as usual. It needs `X-Bolt-Key` like every
call: the caller decides who may see it (Nerd Leagues shows Bolt's hand only when the streamer opts in).
Without `spectate`, a seat that isn't a human seat is still 422.

## Versioning

Breaking changes get `/v2`; additive fields can appear in v1 at any time, so ignore fields you don't
know.
