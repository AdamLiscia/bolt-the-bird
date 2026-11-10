# Draft practice

Draft against Bolt's bots on your own machine: you take one seat at an 8-player Foundations table,
the other seven seats draft like people. Build your deck, then Bolt grades it by playing it out:

- **Your deck's win rate** against the other seven decks at your table, played in the rules engine
  with the smart player on both sides.
- **Bolt drafting your seat:** the same table redrafted from the same opening packs with Bolt in your
  seat, its deck playing the same games, so the two numbers compare directly.
- **Pick by pick:** what you took against what Bolt would have taken, and how much it liked each.

It grades the deck, not your play, and the opponents are bots. Foundations only. Nothing here talks
to Nerd Leagues.

## Run it

    pip install -e ".[practice]"     # once (plus `make engine-py` for the Rust engine)
    python -m practice               # http://localhost:8765

Needs the checkpoints in `checkpoints/` (`human_model.pt`, `judge_b.pt` and the played-reward bot
`bot_rl_played_it135.pt`). Drafts live in memory and are gone when the server stops.

## Share it with friends (ngrok)

    # Same Wi-Fi: PRACTICE_HOST=0.0.0.0 python -m practice, then friends open http://<your IP>:8765
    PRACTICE_PASSWORD=pick-a-password python -m practice
    ngrok http 8765                  # in a second terminal; send friends the https link + password

Your computer has to stay on and awake. Grading plays a few hundred games and takes ~15 s; people
grading at the same moment wait their turn.
