"""python -m practice [port]  (set PRACTICE_PASSWORD to require a password)"""

import os
import sys

import uvicorn

from practice.app import create_app, load_kit

port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
print("loading models...", flush=True)
app = create_app(load_kit(), password=os.environ.get("PRACTICE_PASSWORD") or None)
print(f"Bolt draft practice on http://localhost:{port}", flush=True)
uvicorn.run(app, host="127.0.0.1", port=port, log_level="warning")
