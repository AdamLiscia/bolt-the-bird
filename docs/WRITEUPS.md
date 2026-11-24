# Where Bolt the Bird is described

Every public or semi-public description of the project. When a result changes, the order is:

1. The result file in `results/` and the log (`docs/engine/LOG.md` or `docs/play/LOG.md`).
2. `PLAN.md`: the status row and, if a choice was made, a decision-log row.
3. `docs/headline.json`, if a quoted number changed.
4. `python scripts/check_writeups.py`, then fix every writeup it flags.
5. Adam's career files: `~/life/career/background.md` § Bolt the Bird first (with a `src:` pointer), then
   the resumes. Resume and LinkedIn wording, the website and anything public need Adam's OK.

| Writeup | Audience | Owner of the wording |
|---|---|---|
| `README.md` | GitHub visitors, technical | This repo |
| `docs/MODEL_CARD.md`, `docs/engine/VALIDATION.md` | Technical detail | This repo |
| adamliscia.com/bolt/ (`~/life/career/site/bolt/index.html`) | Everyone: Easy / Hobbyist / Technical, plus an FAQ that doubles as Adam's study guide | Adam's site; push to `adamliscia.github.io` with his OK |
| adamliscia.com home, case study 02 (`~/life/career/site/index.html`) | Recruiters | Adam's site |
| Resumes (`~/life/career/resume/master.md`, variants `ai`, `ml`, `fde`) | Recruiters | Adam; every figure traces to `background.md` |
| LinkedIn (draft: `~/life/career/linkedin-update-*.md`) | Recruiters | Adam; applied in his browser with his OK |
| "Inside Bolt the Bird" (private claude.ai page) | Adam | Updated alongside the site page |

Wording rules (from `background.md`): say "smart player", never "search player"; +3.9 is always a
prediction; never say the bot's decks and the human model's decks played each other in the 62% test
(both play the same fields), and never that it beats human players; Nerd Leagues play is admin-only,
"in testing".
