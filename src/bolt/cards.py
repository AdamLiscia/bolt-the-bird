"""Card facts the simulator and deck builder need: colors, rarity, which cards are basic lands."""
import json
from itertools import combinations

import numpy as np

from .data import DATA

COLORS = "WUBRG"
BASIC_OF = {"W": "Plains", "U": "Island", "B": "Swamp", "R": "Mountain", "G": "Forest"}
PAIRS = ["".join(p) for p in combinations(COLORS, 2)]


class CardInfo:
    def __init__(self, set_code="FDN"):
        self.info = json.loads((DATA / f"{set_code}_card_info.json").read_text())
        self.names = [c["name"] for c in self.info]
        self.n = len(self.info)
        self.index = {n: i for i, n in enumerate(self.names)}
        self.colors = np.zeros((self.n, 5), bool)
        for i, c in enumerate(self.info):
            for ch in c["color"]:
                self.colors[i, COLORS.index(ch)] = True
        self.basic = np.array([c["rarity"] == "basic" for c in self.info])
        self.rare = np.array([c["rarity"] in ("rare", "mythic") for c in self.info])
        self.land = np.array(["Land" in " ".join(c["types"]) for c in self.info])
        self.basic_id = {col: self.index[name] for col, name in BASIC_OF.items()}

    def castable_in(self, pair: str) -> np.ndarray:
        """(n_cards,) spells a deck in these two colors can play: on-color or colorless. No lands."""
        allowed = np.zeros(5, bool)
        for ch in pair:
            allowed[COLORS.index(ch)] = True
        return ~self.land & ~(self.colors & ~allowed).any(1)

    def fixers_for(self, pair: str) -> np.ndarray:
        """(n_cards,) nonbasic lands that make this pair's colors: its dual lands, plus any-color
        lands like Evolving Wilds. Colorless utility lands are left out."""
        out = np.zeros(self.n, bool)
        for i, c in enumerate(self.info):
            makes = c.get("produces", "")
            if self.land[i] and not self.basic[i] and makes and (set(makes) <= set(pair) or len(makes) == 5):
                out[i] = True
        return out
