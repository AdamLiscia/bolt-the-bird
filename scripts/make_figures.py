"""Draw every chart in the README from the committed results/*.json files.

    python scripts/make_figures.py

Each figure is written twice, docs/figures/<name>-light.png and -dark.png, and the README picks
one with <picture> so the charts match the reader's GitHub theme. No number is typed in here:
if a chart changes, it's because a result file changed.
"""

import json

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

from bolt.data import ROOT  # noqa: E402

RES = ROOT / "results"
OUT = ROOT / "docs" / "figures"

THEMES = {
    "light": {
        "bg": "#ffffff", "ink": "#1f2328", "ink2": "#59636e", "grid": "#d1d9e0", "muted": "#c4ccd6",
        "s1": "#2a78d6", "s2": "#eb6834", "s3": "#1baf7a",
    },
    "dark": {
        "bg": "#0d1117", "ink": "#e6edf3", "ink2": "#9198a1", "grid": "#30363d", "muted": "#3d444d",
        "s1": "#3987e5", "s2": "#d95926", "s3": "#199e70",
    },
}  # fmt: skip


def load(name):
    return json.loads((RES / f"{name}.json").read_text())


def style(ax, t, grid_axis="x"):
    ax.set_facecolor(t["bg"])
    for side in ("top", "right", "left", "bottom"):
        ax.spines[side].set_visible(False)
    ax.tick_params(colors=t["ink2"], length=0, labelsize=10)
    ax.grid(axis=grid_axis, color=t["grid"], linewidth=0.8)
    ax.set_axisbelow(True)


def figure(t, w=8, h=3.6):
    fig, ax = plt.subplots(figsize=(w, h), dpi=200)
    fig.patch.set_facecolor(t["bg"])
    return fig, ax


def title(fig, t, text, sub):
    fig.text(0.012, 0.97, text, ha="left", va="top", fontsize=13, fontweight="bold", color=t["ink"])
    fig.text(0.012, 0.895, sub, ha="left", va="top", fontsize=9.5, color=t["ink2"])


def save(fig, name, theme):
    OUT.mkdir(parents=True, exist_ok=True)
    fig.savefig(OUT / f"{name}-{theme}.png", facecolor=fig.get_facecolor())
    plt.close(fig)


def scoreboard(t, theme):
    base = {r["bot"]: r for r in load("baselines")}
    rows = [
        ("Random", base["random"]["acc"], t["muted"]),
        ("Ratings bot", base["ratings bot (GIH WR)"]["acc"], t["muted"]),
        ("Pick-order bot", base["pick-order bot"]["acc"], t["s2"]),
        ("Pool-aware MLP", load("mlp")["acc"], t["muted"]),
        ("DraftTransformer", load("transformer")["acc"], t["s1"]),
    ]
    fig, ax = figure(t, h=3.4)
    y = np.arange(len(rows))[::-1]
    ax.barh(y, [r[1] * 100 for r in rows], color=[r[2] for r in rows], height=0.62)
    for yi, (_, v, _) in zip(y, rows, strict=True):
        ax.text(v * 100 + 1, yi, f"{v:.1%}", va="center", fontsize=10, color=t["ink"], fontweight="bold")
    ax.set_yticks(y, [r[0] for r in rows], color=t["ink"], fontsize=10.5)
    ax.set_xlim(0, 100)
    ax.set_xticks([0, 25, 50, 75, 100], ["0%", "25%", "50%", "75%", "100%"])
    style(ax, t)
    n = load("transformer")["n"]
    title(fig, t, "Top-1 agreement with the human's pick", f"{n:,} picks from held-out drafts, Foundations")
    fig.subplots_adjust(left=0.2, right=0.97, top=0.8, bottom=0.1)
    save(fig, "scoreboard", theme)


def by_pick(t, theme):
    base = {r["bot"]: r for r in load("baselines")}
    series = [
        ("DraftTransformer", load("transformer")["acc_by_pick"], t["s1"]),
        ("Pool-aware MLP", load("mlp")["acc_by_pick"], t["s3"]),
        ("Pick-order bot (ignores your pool)", base["pick-order bot"]["acc_by_pick"], t["s2"]),
    ]
    fig, ax = figure(t, h=3.8)
    x = np.arange(1, 15)
    for name, v, c in series:
        ax.plot(x, np.array(v) * 100, color=c, linewidth=2, marker="o", markersize=4, label=name)
    ax.set_xticks(x)
    ax.set_xlabel("Pick number within a pack (averaged over packs 1–3)", color=t["ink2"], fontsize=9.5)
    ax.set_ylim(0, 102)
    ax.set_yticks([0, 25, 50, 75, 100], ["0%", "25%", "50%", "75%", "100%"])
    style(ax, t, "y")
    leg = ax.legend(loc="lower right", frameon=False, fontsize=9.5)
    for txt in leg.get_texts():
        txt.set_color(t["ink"])
    gap = (np.array(series[0][1]) - np.array(series[2][1]))[1:12] * 100  # picks 2-12
    title(fig, t, f"Seeing your pool adds {gap.min():.0f}–{gap.max():.0f} points per pick",
          "Top-1 agreement by pick, held-out drafts. Picks 2–12, transformer vs. pick-order bot")  # fmt: skip
    fig.subplots_adjust(left=0.08, right=0.98, top=0.8, bottom=0.15)
    save(fig, "by-pick", theme)


def rare_drafting(t, theme):
    r = load("rare_drafting")
    groups = [("Strong drafters", r["strong"]), ("Everyone else", r["others"])]
    fig, ax = figure(t, h=3.2)
    y = np.arange(len(groups))[::-1]
    h = 0.34
    for yi, (_, g) in zip(y, groups, strict=True):
        ax.barh(yi + h / 2, g["rare/mythic"] * 100, height=h, color=t["s2"])
        ax.barh(yi - h / 2, g["common/uncommon"] * 100, height=h, color=t["muted"])
        ax.text(g["rare/mythic"] * 100 + 0.2, yi + h / 2, f"{g['rare/mythic']:.1%}", va="center", fontsize=9.5,
                color=t["ink"], fontweight="bold")  # fmt: skip
        ax.text(g["common/uncommon"] * 100 + 0.2, yi - h / 2, f"{g['common/uncommon']:.1%}", va="center",
                fontsize=9.5, color=t["ink"])  # fmt: skip
    ax.set_yticks(y, [g[0] for g in groups], color=t["ink"], fontsize=10.5)
    ax.set_xlim(0, 14)
    ax.set_xticks([0, 5, 10], ["0%", "5%", "10%"])
    style(ax, t)
    ax.legend(
        handles=[plt.Rectangle((0, 0), 1, 1, color=t["s2"]), plt.Rectangle((0, 0), 1, 1, color=t["muted"])],
        labels=["Off-color rare or mythic", "Off-color common or uncommon"], loc="lower right", frameon=False,
        fontsize=9, labelcolor=t["ink"],
    )  # fmt: skip
    title(fig, t, "Humans rare-draft, and good players do it too", "How often an off-color card is taken when offered, packs 2–3")  # fmt: skip
    fig.subplots_adjust(left=0.19, right=0.97, top=0.78, bottom=0.12)
    save(fig, "rare-drafting", theme)


def calibration(t, theme):
    rows = load("judge")["calibration_by_judged_quintile"]
    pred = np.array([r["judged"] for r in rows]) * 100
    act = np.array([r["actual"] for r in rows]) * 100
    fig, ax = figure(t, w=5.2, h=4.4)
    lo, hi = 44, 65
    ax.plot([lo, hi], [lo, hi], color=t["grid"], linewidth=1.5, linestyle="--", label="Perfect calibration")
    ax.plot(pred, act, color=t["s1"], linewidth=2, marker="o", markersize=7, label="The judge, held-out games")
    ax.set_xlim(lo, hi)
    ax.set_ylim(lo, hi)
    ax.set_xlabel("Predicted win rate", color=t["ink2"], fontsize=9.5)
    ax.set_ylabel("Actual win rate", color=t["ink2"], fontsize=9.5)
    ticks = [45, 50, 55, 60, 65]
    ax.set_xticks(ticks, [f"{v}%" for v in ticks])
    ax.set_yticks(ticks, [f"{v}%" for v in ticks])
    style(ax, t, "both")
    ax.legend(loc="upper left", frameon=False, fontsize=9, labelcolor=t["ink"])
    n = sum(r["games"] for r in rows)
    title(fig, t, "The judge's predictions hold up", f"{n:,} held-out games, grouped into fifths by prediction")
    fig.subplots_adjust(left=0.14, right=0.96, top=0.8, bottom=0.12)
    save(fig, "judge-calibration", theme)


def rl_history():
    """Evaluations from the first run, then any continuation runs, on one iteration axis."""
    evals, offset = [], 0
    for name in ("rl", "rl_long"):
        path = RES / f"{name}.json"
        if not path.exists():
            continue
        hist = [h["eval"] for h in json.loads(path.read_text())["history"] if "eval" in h]
        for e in hist:
            if offset and e["iter"] == 0:
                continue  # a continuation's iteration 0 repeats the previous run's last eval
            evals.append({**e, "iter": e["iter"] + offset})
        offset = evals[-1]["iter"]
    return evals


def rl_curve(t, theme):
    ev = rl_history()
    it = np.array([e["iter"] for e in ev])
    gain = np.array([e["gain"] for e in ev]) * 100
    se = np.array([e["gain_se"] for e in ev]) * 100
    rares = np.array([e["bot_late_rares"] for e in ev])
    human_rares = ev[0]["human_model_late_rares"]

    fig, (a1, a2) = plt.subplots(1, 2, figsize=(9, 3.6), dpi=200)
    fig.patch.set_facecolor(t["bg"])
    a1.fill_between(it, gain - 1.96 * se, gain + 1.96 * se, color=t["s1"], alpha=0.18, linewidth=0)
    a1.plot(it, gain, color=t["s1"], linewidth=2, marker="o", markersize=4)
    a1.axhline(0, color=t["ink2"], linewidth=1)
    a1.set_title("Judged win-rate gain over the human model", color=t["ink"], fontsize=10, loc="left")
    a1.set_ylabel("Points (judge B, 95% CI)", color=t["ink2"], fontsize=9)
    a1.annotate(f"+{gain[-1]:.2f}", (it[-1], gain[-1]), textcoords="offset points", xytext=(-4, 8), ha="right",
                color=t["ink"], fontweight="bold", fontsize=10)  # fmt: skip
    a2.plot(it, rares, color=t["s2"], linewidth=2, marker="o", markersize=4, label="Learner")
    a2.axhline(human_rares, color=t["ink2"], linewidth=1, linestyle="--", label="Human model")
    a2.set_title("Rares taken after pack 1, per draft", color=t["ink"], fontsize=10, loc="left")
    a2.legend(frameon=False, fontsize=9, labelcolor=t["ink"], loc="lower left")
    for ax in (a1, a2):
        style(ax, t, "y")
        ax.set_xlabel("Training iteration (128 simulated drafts each)", color=t["ink2"], fontsize=9)
    fig.text(0.012, 0.97, "Learning to win, not to copy", ha="left", va="top", fontsize=13, fontweight="bold",
             color=t["ink"])  # fmt: skip
    fig.subplots_adjust(left=0.07, right=0.98, top=0.78, bottom=0.16, wspace=0.25)
    save(fig, "rl-curve", theme)


def pilot_ladder(t, theme):
    lad = load("engine_pilot_ladder")
    rows = lad["pilots"]
    x = np.arange(len(rows))
    gap = np.array([r["gap"] for r in rows]) * 100
    ci = np.array([r["gap_se"] for r in rows]) * 196
    judged = lad["judge_b_gap"] * 100
    fig, ax = figure(t, w=8, h=3.9)
    style(ax, t, "y")
    ax.axhline(0, color=t["ink2"], linewidth=1)
    ax.axhline(judged, color=t["s2"], linewidth=1.5, linestyle="--")
    ax.text(len(rows) - 0.55, judged + 0.18, f"Judge B predicted +{judged:.2f}", ha="right", va="bottom",
            fontsize=9.5, color=t["ink"])  # fmt: skip
    ax.errorbar(x, gap, yerr=ci, color=t["s1"], linewidth=2, marker="o", markersize=7, capsize=0, elinewidth=1.5)
    for xi, g in zip(x, gap, strict=True):
        ax.annotate(f"{g:+.2f}", (xi, g), textcoords="offset points", xytext=(10, 4), ha="left",
                    color=t["ink"], fontweight="bold", fontsize=10)  # fmt: skip
    labels = [f"{r['label'].replace(' x ', ' × ')}\nbeats rules-based {r['beats_baseline'] * 100:g}%" if r["pilot"] != "baseline"
              else f"{r['label']}\n(the reference)" for r in rows]  # fmt: skip
    ax.set_xticks(x, labels, color=t["ink"], fontsize=9)
    ax.set_xlim(-0.4, len(rows) - 0.4)
    ax.set_ylim(-2, 4.6)
    ax.set_ylabel("Points of played win rate (95% CI)", color=t["ink2"], fontsize=9)
    title(fig, t, "The bot's edge disappears once players think ahead",
          f"Bot decks minus human-model decks, both sides played by the same pilot; "
          f"{lad['games_per_pilot']:,} games per pilot")  # fmt: skip
    fig.subplots_adjust(left=0.08, right=0.98, top=0.8, bottom=0.17)
    save(fig, "pilot-ladder", theme)


def played_reward(t, theme):
    """The judge-trained bot and the played-reward bot at each pilot strength."""
    lad = load("engine_pilot_ladder")
    new = load("engine_pilot_rl_played250")
    rows = lad["pilots"]
    x = np.arange(len(rows))
    judged = lad["judge_b_gap"] * 100
    fig, ax = figure(t, w=8, h=4.1)
    style(ax, t, "y")
    ax.axhline(0, color=t["ink2"], linewidth=1)
    ax.axhline(judged, color=t["s2"], linewidth=1.5, linestyle="--")
    ax.text(-0.3, judged + 0.2, f"Judge B's prediction for the old bot: +{judged:.2f}", ha="left", va="bottom",
            fontsize=9, color=t["ink"])  # fmt: skip
    old = np.array([r["gap"] for r in rows]) * 100
    old_ci = np.array([r["gap_se"] for r in rows]) * 196
    ax.errorbar(x, old, yerr=old_ci, color=t["ink2"], linewidth=2, marker="o", markersize=7, capsize=0,
                elinewidth=1.5, label="Trained on the judge's score")  # fmt: skip
    have = [i for i, r in enumerate(rows) if "all" in new.get(r["pilot"], {})]
    ng = np.array([new[rows[i]["pilot"]]["all"]["gap"] for i in have]) * 100
    nci = np.array([new[rows[i]["pilot"]]["all"]["gap_se"] for i in have]) * 196
    ax.errorbar(have, ng, yerr=nci, color=t["s1"], linewidth=2.5, marker="o", markersize=8, capsize=0,
                elinewidth=1.5, label="Trained on played games")  # fmt: skip
    for xi, g in zip(have, ng, strict=True):
        ax.annotate(f"{g:+.2f}", (xi, g), textcoords="offset points", xytext=(10, 4), ha="left",
                    color=t["ink"], fontweight="bold", fontsize=10)  # fmt: skip
    for xi, g in zip(x, old, strict=True):
        ax.annotate(f"{g:+.2f}", (xi, g), textcoords="offset points", xytext=(10, -14), ha="left",
                    color=t["ink2"], fontsize=9)  # fmt: skip
    labels = [f"{r['label'].replace(' x ', ' × ')}\nbeats rules-based {r['beats_baseline'] * 100:g}%" if r["pilot"] != "baseline"
              else f"{r['label']}\n(the reference)" for r in rows]  # fmt: skip
    ax.set_xticks(x, labels, color=t["ink"], fontsize=9)
    ax.set_xlim(-0.4, len(rows) - 0.4)
    ax.set_ylim(-2, max(10, float((ng + nci).max()) + 1.5))
    ax.set_ylabel("Points of played win rate (95% CI)", color=t["ink2"], fontsize=9)
    ax.legend(frameon=False, fontsize=9, labelcolor=t["ink"], loc="center left", bbox_to_anchor=(0.0, 0.5))
    title(fig, t, "Trained on real games, the bot's edge holds up",
          "Bot decks minus human-model decks, both sides played by the same pilot, three fields of opponents")
    fig.subplots_adjust(left=0.08, right=0.98, top=0.8, bottom=0.16)
    save(fig, "played-reward", theme)


def main():
    for theme, t in THEMES.items():
        for draw in (scoreboard, by_pick, rare_drafting, calibration, rl_curve, pilot_ladder, played_reward):
            draw(t, theme)
    print("wrote", sorted(p.name for p in OUT.glob("*.png")))


if __name__ == "__main__":
    main()
