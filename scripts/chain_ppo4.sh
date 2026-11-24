#!/usr/bin/env bash
# 2026-11-23 night: the learned player now sees the spell it is choosing targets for ("choosing" in the
# observation, a stack token in the encoding). After P4's fields b/c finish (they run on the installed
# engine), install the new engine, run the tests, then continue self-play PPO from iteration 850 for 10
# hours (ppo4; looser leash, since the old targeting habit is what it should unlearn), then evaluate it,
# including the targeting check. One job at a time, 3 GB caps, every iteration kept.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH=/opt/homebrew/opt/rustup/bin:$PATH
while pgrep -f "scripts/chain_p4b.sh" >/dev/null; do sleep 60; done
echo "== install engine $(date +%T)"
make engine-py 2>&1 | grep -E "Installed|error|failed"
.venv/bin/python -c "import bolt_engine as be; o=be.Game([0]*40,[0]*40,seed=0).observation(0); assert 'choosing' in o, 'engine without choosing'" || exit 1
echo "== tests $(date +%T)"
scripts/guard.sh 2400 5000 -- .venv/bin/python -m pytest -q -p no:warnings tests/ 2>&1 | tail -2
scripts/guard.sh 2400 5000 -- .venv/bin/python -m pytest -q -p no:warnings tests/ >/dev/null 2>&1 || { echo "tests failed: stopping"; exit 1; }
echo "== ppo4 $(date +%T)"
scripts/guard.sh 36000 3000 -- .venv/bin/python scripts/train_ppo_play.py --name ppo4 --resume \
  --init checkpoints/play/ppo3/iter_850.pt --lam 0.95 --games 640 --lr 1e-5 --kl 0.03 --kl-final 0.01 --kl-iters 50
echo "== training stopped (exit $?) $(date +%T)"
echo "== eval $(date +%T)"
scripts/guard.sh 3600 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/ppo4/best.pt \
  --pairs 500 --workers 4 --no-control --name play_eval_ppo4_best
scripts/guard.sh 10800 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/ppo4/best.pt \
  --pairs 500 --workers 4 --opponents smart --smart 4 6 --no-control --name play_eval_ppo4_best_smart46
echo "== target check $(date +%T)"
scripts/guard.sh 1800 2000 -- .venv/bin/python scripts/target_check.py network 200 --ckpt checkpoints/play/ppo4/best.pt
scripts/guard.sh 1800 2000 -- .venv/bin/python scripts/target_check.py network 200 --harm --ckpt checkpoints/play/ppo4/best.pt
echo "== done $(date +%T)"
