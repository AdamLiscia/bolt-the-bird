# Bolt the Bird. Every result in the README comes from one of these targets.
# `make help` lists them. Training targets take hours; `make test` takes a second.

PY ?= .venv/bin/python
DEVICE ?= auto

.PHONY: help install test lint format demo data prepare baselines draft-models rare-drafting judges builder rl archetypes card-audit engine-test figures reproduce

help:  ## List targets
	@grep -E '^[a-z-]+:.*##' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "} {printf "  \033[1m%-15s\033[0m %s\n", $$1, $$2}'

install:  ## Create .venv and install the package with dev tools
	python3 -m venv .venv
	$(PY) -m pip install -U pip
	$(PY) -m pip install -e ".[dev]"

test:  ## Run the test suite (synthetic data, seconds)
	$(PY) -m pytest

lint:  ## Lint and check formatting
	$(PY) -m ruff check .
	$(PY) -m ruff format --check .

format:  ## Auto-format
	$(PY) -m ruff format .
	$(PY) -m ruff check --fix .

demo:  ## Watch the trained bot draft a table (needs the released checkpoints)
	$(PY) scripts/demo.py

data:  ## Download the 17lands FDN datasets (~250 MB)
	$(PY) scripts/download.py

prepare: data  ## Stream the CSVs into compact arrays (~1 min)
	$(PY) scripts/prepare.py
	$(PY) scripts/card_info.py

baselines:  ## Random, ratings and pick-order bots
	$(PY) scripts/baselines.py

draft-models:  ## MLP and transformer draft models (~4 h on an M4)
	$(PY) scripts/train.py --model mlp --epochs 2
	$(PY) scripts/train.py --model transformer --epochs 3

rare-drafting:  ## The off-color rare analysis
	$(PY) scripts/rare_drafting.py

judges:  ## Full judge + judges A and B on disjoint halves (~3 min)
	$(PY) scripts/train_judge.py
	$(PY) scripts/train_judge.py --half 0
	$(PY) scripts/train_judge.py --half 1

builder:  ## Deck builder vs. real players' decks
	$(PY) scripts/eval_builder.py

rl:  ## Teach the bot to win: 150 iterations, then 400 more (~7 h on CPU)
	$(PY) scripts/train_rl.py --device $(DEVICE)
	$(PY) scripts/train_rl.py --device $(DEVICE) --resume checkpoints/bot_rl.pt --name rl_long --iters 400 --tables 128 --eval-every 25

archetypes:  ## Win rate by color pair in real games, and what each drafter drafts
	$(PY) scripts/archetypes.py

card-audit:  ## Tier every card by rules complexity, and the build order
	$(PY) scripts/card_audit.py

engine-test:  ## Format check, lint and test the Rust engine
	cd engine && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --no-run
	cd engine && ../scripts/guard.sh 600 2000 -- cargo test

figures:  ## Regenerate every chart in docs/figures from results/
	$(PY) scripts/make_figures.py

reproduce: prepare baselines draft-models rare-drafting judges builder rl archetypes figures  ## Everything, end to end
