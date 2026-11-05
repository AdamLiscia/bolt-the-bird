"""How much rare-drafting is in the human data?

For each draft, the drafter's main colors are the two colors with the most cards in their final
pool (hindsight, on purpose: it tells us what deck they ended up building). From pack 2 onward,
when colors are usually settled, we ask how often a card that doesn't fit those colors gets taken,
split by rarity and by how good the drafter is.

    python scripts/rare_drafting.py
"""

import json

import numpy as np

from bolt.data import ASSETS, DATA, STRONG_WR, load_picks

COLORS = "WUBRG"


def main():
    info = json.loads((ASSETS / "FDN_card_info.json").read_text())
    n = len(info)
    cmask = np.zeros((n + 1, 5), bool)  # card id+1 -> colors
    for i, c in enumerate(info):
        for ch in c["color"]:
            cmask[i + 1, COLORS.index(ch)] = True
    is_rare = np.array([False] + [c["rarity"] in ("rare", "mythic") for c in info])
    colored = cmask.any(1)

    p = load_picks()
    # Final pool per draft = the pool at its last recorded pick + that pick.
    order = np.lexsort((p["pick_number"], p["pack_number"], p["draft"]))
    last = order[np.r_[np.flatnonzero(np.diff(p["draft"][order])), len(order) - 1]]
    final_counts = np.zeros((p["draft"].max() + 1, 5))
    final_counts[p["draft"][last]] = cmask[p["pool"][last]].sum(1) + cmask[p["pick"][last].astype(int) + 1]
    complete = np.zeros(len(final_counts), bool)
    complete[p["draft"][last]] = (p["pack_number"][last] == 2) & (p["pick_number"][last] >= 12)
    top2 = np.argsort(-final_counts, 1)[:, :2]
    main = np.zeros_like(final_counts, bool)
    np.put_along_axis(main, top2, True, 1)

    late = (p["pack_number"] >= 1) & complete[p["draft"]]
    strong = p["user_wr"] >= STRONG_WR
    dmain = main[p["draft"]]

    def off(ids):  # (n, k) card ids+1 -> off-color (colored and not inside main colors)
        c = cmask[ids]
        return colored[ids] & (c & ~dmain[:, None, :]).any(-1) if ids.ndim == 2 else colored[ids] & (c & ~dmain).any(-1)

    pack = p["pack"].astype(int)
    pick = p["pick"].astype(int) + 1
    pack_off = off(pack) & (pack > 0)
    took = pack == pick[:, None]

    out = {}
    for label, who in [("all", np.ones_like(strong)), ("strong", strong), ("others", ~strong)]:
        m = late & who
        r = {}
        for rar_label, rar in [("rare/mythic", is_rare[pack]), ("common/uncommon", ~is_rare[pack])]:
            seen = (pack_off & rar)[m]
            r[rar_label] = float((took[m] & seen).sum() / seen.sum())
        took_off_rare = (off(pick) & is_rare[pick])[m]
        r["share_of_late_picks_off_color_rare"] = float(took_off_rare.mean())
        out[label] = r
        print(
            f"{label:<7} off-color take rate, pack 2-3: rares {r['rare/mythic']:.1%}  "
            f"commons/uncommons {r['common/uncommon']:.1%}   "
            f"late picks that are off-color rares: {r['share_of_late_picks_off_color_rare']:.2%}"
        )

    # Do drafters who rare-draft more win less? (Correlation only: skill confounds it.)
    off_rare_pick = late & off(pick) & is_rare[pick]
    per_draft = np.bincount(p["draft"][off_rare_pick], minlength=len(final_counts))
    wins = np.zeros(len(final_counts))
    wins[p["draft"][last]] = p["event_wins"][last]
    groups = {"0": per_draft == 0, "1": per_draft == 1, "2+": per_draft >= 2}
    out["event_wins_by_late_off_color_rares"] = {}
    for k, g in groups.items():
        g = g & complete
        out["event_wins_by_late_off_color_rares"][k] = {"drafts": int(g.sum()), "avg_wins": float(wins[g].mean())}
        print(f"drafts with {k:>2} late off-color rare picks: {g.sum():>7,}  avg match wins {wins[g].mean():.2f}")

    res = DATA.parent / "results"
    res.mkdir(exist_ok=True)
    (res / "rare_drafting.json").write_text(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
