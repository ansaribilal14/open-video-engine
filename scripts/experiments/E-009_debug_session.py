#!/usr/bin/env python3
"""Debug: manual MCP session against E-009 server."""
import json, subprocess, sys, tempfile, os

tmp = tempfile.mkdtemp(prefix="e009dbg-")
log = os.path.join(tmp, "dbg.jsonl")
p = subprocess.Popen([sys.executable, "/home/z/my-project/open-video-engine/scripts/experiments/E-009_shared_command_surface.py",
                      "--serve", "--log", log],
                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

def req(method, params=None, expect_id=[0]):
    expect_id[0] += 1
    p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": expect_id[0], "method": method, "params": params or {}}) + "\n")
    p.stdin.flush()
    line = p.stdout.readline()
    return json.loads(line) if line else None

print("INIT:", json.dumps(req("initialize", {"protocolVersion": "2026-07-28", "capabilities": {}, "clientInfo": {"name": "dbg", "version": "0"}}))[:200])
p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n"); p.stdin.flush()

r = req("tools/call", {"name": "add_clip", "arguments": {"cid": "c1", "track": 0, "start": {"num": 0, "den": 1}, "dur": {"num": 10, "den": 1}, "name": "intro"}})
print("ADD c1:", json.dumps(r))
r = req("tools/call", {"name": "get_state_hash"})
print("HASH:", r["result"]["content"][0]["text"])
p.stdin.close()
err = p.stderr.read()
if err: print("SERVER STDERR:", err[:2000])
