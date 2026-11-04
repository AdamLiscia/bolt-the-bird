"""The judge: predicts how often a 40-card deck wins a game.

Player skill and play/draw go in as separate inputs. Good players build good decks *and* play
them well; without the skill input, the judge would credit the deck for the pilot. When we score a
deck, we hold skill fixed so only the deck can move the prediction.
"""
import torch
from torch import nn

SKILL_REF = 0.54   # the median player's win-rate bucket: the "average pilot" we judge decks with


class LinearJudge(nn.Module):
    """Baseline: every card adds or subtracts a fixed amount, regardless of the rest of the deck."""

    def __init__(self, n_cards: int):
        super().__init__()
        self.card = nn.Linear(n_cards, 1)
        self.ctx = nn.Linear(2, 1, bias=False)

    def forward(self, deck, skill, on_play):
        return (self.card(deck) + self.ctx(torch.stack([skill - SKILL_REF, on_play - .5], 1))).squeeze(1)


class DeckJudge(nn.Module):
    """Sum of learned card embeddings -> MLP. Can learn that cards are worth more together
    (a curve, a color pair, enough creatures) than apart."""

    def __init__(self, n_cards: int, d: int = 96, hidden: int = 256, dropout: float = 0.1):
        super().__init__()
        self.emb = nn.Linear(n_cards, d, bias=False)
        self.card_bias = nn.Linear(n_cards, 1)
        self.mlp = nn.Sequential(nn.Linear(d, hidden), nn.GELU(), nn.Dropout(dropout),
                                 nn.Linear(hidden, hidden), nn.GELU(), nn.Dropout(dropout), nn.Linear(hidden, 1))
        self.ctx = nn.Linear(2, 1, bias=False)

    def forward(self, deck, skill, on_play):
        h = self.emb(deck) / 8
        deck_score = self.mlp(h) + self.card_bias(deck)
        return (deck_score + self.ctx(torch.stack([skill - SKILL_REF, on_play - .5], 1))).squeeze(1)

    @torch.no_grad()
    def win_rate(self, deck: torch.Tensor) -> torch.Tensor:
        """Predicted game win rate for an average pilot, averaged over play and draw."""
        n = deck.shape[0]
        skill = torch.full((n,), SKILL_REF, device=deck.device)
        p = [torch.sigmoid(self(deck, skill, torch.full((n,), op, device=deck.device))) for op in (0., 1.)]
        return (p[0] + p[1]) / 2


JUDGES = {"linear": LinearJudge, "mlp": DeckJudge}
