"""python -m practice [port]

PRACTICE_PASSWORD=... requires a password. PRACTICE_HOST=0.0.0.0 lets other computers on the same
Wi-Fi connect (http://<this computer's IP>:port); the default only allows this computer.
"""

import os
import sys

import uvicorn

from practice.app import create_app, load_kit

port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
print("loading models...", flush=True)
app = create_app(load_kit(), password=os.environ.get("PRACTICE_PASSWORD") or None)
print(f"Bolt draft practice on http://localhost:{port}", flush=True)
host = os.environ.get("PRACTICE_HOST", "127.0.0.1")
uvicorn.run(app, host=host, port=port, log_level="warning")
