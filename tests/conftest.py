"""Shared fixtures. Everything here is synthetic, so the suite runs in seconds with no data download."""

import sys
from pathlib import Path

import pytest
import torch

from bolt.cards import CardInfo
from bolt.models import to_vocab

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

COLORS = "WUBRG"


def tiny_set() -> list[dict]:
    """A 38-card toy set: 6 creatures per color, one dual land, one fixer, an artifact, 5 basics."""
    info = []
    for c in COLORS:
        for i in range(6):
            info.append(
                {
                    "name": f"{c} Creature {i}",
                    "color": c,
                    "rarity": "rare" if i == 0 else "common",
                    "types": ["Creature"],
                }
            )
    info.append({"name": "WU Dual", "color": "", "rarity": "common", "types": ["Land"], "produces": "WU"})
    info.append({"name": "Wilds", "color": "", "rarity": "common", "types": ["Land"], "produces": "WUBRG"})
    info.append({"name": "Golem", "color": "", "rarity": "common", "types": ["Artifact Creature"]})
    for name in ["Plains", "Island", "Swamp", "Mountain", "Forest"]:
        info.append({"name": name, "color": "", "rarity": "basic", "types": ["Basic Land"]})
    return info


@pytest.fixture
def cards() -> CardInfo:
    return CardInfo(info=tiny_set())


class RandomDrafter:
    """Stands in for a draft model: random scores, same signature, same masking."""

    def __init__(self, n_cards: int, seed: int = 0):
        self.n = n_cards
        self.g = torch.Generator().manual_seed(seed)

    def __call__(self, pool, pack, pack_number, pick_number):
        return to_vocab(torch.randn(pack.shape, generator=self.g), pack, self.n)


class ValueJudge:
    """Stands in for the judge: a deck's score is the sum of fixed per-card values."""

    def __init__(self, values: torch.Tensor):
        self.values = values

    def win_rate(self, deck: torch.Tensor) -> torch.Tensor:
        return torch.sigmoid(deck @ self.values / 40)
