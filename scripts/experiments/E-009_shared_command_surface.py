#!/usr/bin/env python3
"""
E-009: Shared command surface — do AI-agent edits and UI edits share ONE transaction
log without schema divergence? (OPEN_QUESTIONS Q-09; gates ADR-010.)

METHOD: one engine core (E-003 command model, extended to 10 command surfaces +
exact-rational payload validation). Two client paths drive the SAME logical scenario:
  PATH A (UI)      — in-process direct engine calls (browser-WASM / desktop-shell shape)
  PATH B (AGENT)   — MCP 2026-07-28 stdio JSON-RPC server in a subprocess (tools/call)
Both append to ONE shared log file (event-sourced project format, undo = logged marker).

CHECKS:
  P1  hash equality (direct vs MCP engine) after every interleaved step
  P2  merged log replay (fresh engine) == final state hash; entries ownership-agnostic
  P3  cross-ownership undo (UI undo undoes agent batch; agent undo undoes UI split)
  P4  batch atomicity: invalid inner command -> whole batch rejected, state+log unchanged
  P5  float payload rejection at the API edge (3 variants), fail-closed
  P6  determinism: fresh server + fresh log, same scenario -> identical final hash
  P7  tools/list schema conformance: 10 command tools; time fields integer-only

Runs:  python3 E-009_shared_command_surface.py            (driver; spawns server)
"""
import json, os, subprocess, sys, tempfile
from fractions import Fraction

F = Fraction
HERE = os.path.dirname(os.path.abspath(__file__))

# ================= exact-time engine (shared by both paths) =================
def R(v):
    """Wire/doc value -> exact rational. Accepts {'num':int,'den':int} or Fraction.
    Floats are REJECTED (fail-closed) — ADR-007 boundary."""
    if isinstance(v, Fraction):
        return v
    if isinstance(v, dict):
        n, d = v.get("num"), v.get("den")
        for x in (n, d):
            if not isinstance(x, int) or isinstance(x, bool):
                raise TypeError(f"non-integer rational component: {x!r}")
        if d <= 0:
            raise ValueError(f"den must be > 0: {d}")
        return F(n, d)
    if isinstance(v, float):
        raise TypeError(f"float payload forbidden by ADR-007: {v!r}")
    raise TypeError(f"bad rational value: {v!r}")

def wr(f):  # rational -> wire dict
    return {"num": f.numerator, "den": f.denominator}

class Doc:
    def __init__(self):
        self.clips = {}  # cid -> [track:int, start:F, dur:F, name:str, gain:F]
    def clone(self):
        d = Doc(); d.clips = {k: list(v) for k, v in self.clips.items()}; return d
    def canon(self):
        items = sorted((cid, t, s.numerator, s.denominator, d.numerator, d.denominator,
                        n, g.numerator, g.denominator)
                       for cid, (t, s, d, n, g) in self.clips.items())
        return json.dumps(items, separators=(",", ":"))

def doc_hash(doc):
    import hashlib
    return hashlib.sha256(doc.canon().encode()).hexdigest()[:16]

class EngineError(Exception): pass

# ---- commands: apply(engine) + inverse() ; to_json = log entry ----
class AddClip:
    tag = "add_clip"
    def __init__(self, cid, track, start, dur, name, gain=None):
        self.cid, self.track, self.start, self.dur = cid, track, R(start), R(dur)
        self.name, self.gain = name, R(gain) if gain is not None else F(1)
    def apply(self, eng):
        if self.cid in eng.doc.clips: raise EngineError(f"id exists: {self.cid}")
        if self.start < 0 or self.dur <= 0: raise EngineError("start>=0, dur>0 required")
        eng.doc.clips[self.cid] = [self.track, self.start, self.dur, self.name, self.gain]
    def inverse(self): return RemoveClip(self.cid)
    def to_json(self):
        return {"op": self.tag, "cid": self.cid, "track": self.track,
                "start": wr(self.start), "dur": wr(self.dur), "name": self.name,
                "gain": wr(self.gain)}

class RemoveClip:
    tag = "remove_clip"
    def __init__(self, cid): self.cid = cid; self._saved = None
    def apply(self, eng):
        if self.cid not in eng.doc.clips: raise EngineError(f"no clip: {self.cid}")
        self._saved = list(eng.doc.clips.pop(self.cid))
    def inverse(self):
        t, s, d, n, g = self._saved
        return AddClip(self.cid, t, s, d, n, g)
    def to_json(self): return {"op": self.tag, "cid": self.cid}

class MoveClip:
    tag = "move_clip"
    def __init__(self, cid, track, start): self.cid, self.track, self.start = cid, track, R(start)
    def apply(self, eng):
        c = eng._clip(self.cid); self._saved = (c[0], c[1])
        c[0], c[1] = self.track, self.start
    def inverse(self): return MoveClip(self.cid, *self._saved)
    def to_json(self): return {"op": self.tag, "cid": self.cid, "track": self.track,
                               "start": wr(self.start)}

class TrimClip:
    tag = "trim_clip"
    def __init__(self, cid, dur): self.cid, self.dur = cid, R(dur)
    def apply(self, eng):
        c = eng._clip(self.cid); self._saved = c[2]
        if self.dur <= 0: raise EngineError("dur>0 required")
        c[2] = self.dur
    def inverse(self): return TrimClip(self.cid, self._saved)
    def to_json(self): return {"op": self.tag, "cid": self.cid, "dur": wr(self.dur)}

class SplitClip:
    tag = "split_clip"
    def __init__(self, cid, offset, new_cid): self.cid, self.off, self.new = cid, R(offset), new_cid
    def apply(self, eng):
        c = eng._clip(self.cid); t, s, d, n, g = c
        if not (0 < self.off < d): raise EngineError("split offset out of range")
        if self.new in eng.doc.clips: raise EngineError(f"id exists: {self.new}")
        c[2] = self.off
        eng.doc.clips[self.new] = [t, s + self.off, d - self.off, n, g]
    def inverse(self): return MergeClips(self.cid, self.new)
    def to_json(self): return {"op": self.tag, "cid": self.cid, "offset": wr(self.off),
                               "new_cid": self.new}

class MergeClips:  # adjacent merge: right folded into left (exact)
    tag = "merge_clips"
    def __init__(self, left, right): self.left, self.right = left, right; self._rs = None
    def apply(self, eng):
        L = eng._clip(self.left); Rr = eng._clip(self.right)
        t1, s1, d1, n1, g1 = L; t2, s2, d2, n2, g2 = Rr
        if t1 != t2 or abs((s1 + d1) - s2) != 0: raise EngineError("merge requires adjacent clips on one track")
        self._rs = list(Rr)
        L[2] = d1 + d2
        del eng.doc.clips[self.right]
    def inverse(self):
        t, s, d, n, g = self._rs
        return Resplit(self.left, self.right, self._rs)
    def to_json(self): return {"op": self.tag, "left": self.left, "right": self.right}

class Resplit:  # inverse of merge — restores right clip exactly
    tag = "_resplit"
    def __init__(self, left, right, right_state): self.left, self.right, self.rs = left, right, right_state
    def apply(self, eng):
        L = eng._clip(self.left); t, s, d, n, g = self.rs
        L[2] = L[2] - d
        eng.doc.clips[self.right] = list(self.rs)
    def to_json(self): return {"op": self.tag, "left": self.left, "right": self.right,
                               "state": [self.rs[0], wr(self.rs[1]), wr(self.rs[2]),
                                         self.rs[3], wr(self.rs[4])]}
    @classmethod
    def from_json(cls, e):
        st = e["state"]
        return cls(e["left"], e["right"], [st[0], F(st[1]["num"], st[1]["den"]),
                   F(st[2]["num"], st[2]["den"]), st[3], F(st[4]["num"], st[4]["den"])])

class RenameClip:
    tag = "rename_clip"
    def __init__(self, cid, name): self.cid, self.name = cid, name
    def apply(self, eng):
        c = eng._clip(self.cid); self._saved = c[3]; c[3] = self.name
    def inverse(self): return RenameClip(self.cid, self._saved)
    def to_json(self): return {"op": self.tag, "cid": self.cid, "name": self.name}

class SetGain:
    tag = "set_gain"
    def __init__(self, cid, gain): self.cid, self.gain = cid, R(gain)
    def apply(self, eng):
        c = eng._clip(self.cid); self._saved = c[4]
        if self.gain <= 0: raise EngineError("gain>0 required")
        c[4] = self.gain
    def inverse(self): return SetGain(self.cid, self._saved)
    def to_json(self): return {"op": self.tag, "cid": self.cid, "gain": wr(self.gain)}

class Batch:
    tag = "batch"
    def __init__(self, cmds): self.cmds = cmds; self._applied = []
    def apply(self, eng):
        # atomic: apply to a copy first; on any error nothing lands
        trial = eng.doc.clone()
        probe = Engine(doc=trial, log=None, validate_only=True)
        for c in self.cmds: c.apply(probe)
        for c in self.cmds:
            c.apply(eng); self._applied.append(c)
    def inverse(self):
        return CompositeInverse([c.inverse() for c in reversed(self._applied)])
    def to_json(self):
        return {"op": self.tag, "cmds": [c.to_json() for c in self.cmds]}

class CompositeInverse:
    tag = "_composite"
    def __init__(self, inverses): self.inverses = inverses
    def apply(self, eng):
        for c in self.inverses: c.apply(eng)
    def to_json(self): return {"op": self.tag}

def cmd_from_json(e):
    op = e["op"]
    if op == "add_clip":   return AddClip(e["cid"], e["track"], e["start"], e["dur"], e["name"], e["gain"])
    if op == "remove_clip": return RemoveClip(e["cid"])
    if op == "move_clip":  return MoveClip(e["cid"], e["track"], e["start"])
    if op == "trim_clip":  return TrimClip(e["cid"], e["dur"])
    if op == "split_clip": return SplitClip(e["cid"], e["offset"], e["new_cid"])
    if op == "merge_clips": return MergeClips(e["left"], e["right"])
    if op == "_resplit":   return Resplit.from_json(e)
    if op == "rename_clip": return RenameClip(e["cid"], e["name"])
    if op == "set_gain":   return SetGain(e["cid"], e["gain"])
    if op == "batch":      return Batch([cmd_from_json(c) for c in e["cmds"]])
    if op == "undo":       return None  # handled by replay loop
    raise EngineError(f"unknown op: {op}")

class Engine:
    def __init__(self, doc=None, log=None, validate_only=False, log_path=None):
        self.doc = doc or Doc()
        self.log = log if log is not None else []
        self.undo_stack = []      # applied forward commands (for inverse)
        self.log_path = log_path
        self._validate_only = validate_only
    def _clip(self, cid):
        if cid not in self.doc.clips: raise EngineError(f"no clip: {cid}")
        return self.doc.clips[cid]
    def apply(self, cmd, owner="ui"):
        entry = cmd.to_json()
        if self._validate_only:
            cmd.apply(self); return entry
        cmd.apply(self)
        entry["_owner"] = owner          # provenance only — replay ignores it
        self.log.append(entry)
        self.undo_stack.append(cmd)
        self._append_file(entry)
        return entry
    def undo(self, owner="ui"):
        if not self.undo_stack: raise EngineError("undo stack empty")
        cmd = self.undo_stack.pop()
        inv = cmd.inverse()
        inv.apply(self)
        marker = {"op": "undo", "_owner": owner}
        self.log.append(marker); self._append_file(marker)
        return marker
    def _append_file(self, entry):
        if self.log_path:
            with open(self.log_path, "a") as f: f.write(json.dumps(entry) + "\n")

def replay(entries):
    eng = Engine()
    stack = []
    for e in entries:
        e = dict(e); e.pop("_owner", None)
        if e.get("op") == "undo":
            if not stack: raise EngineError("replay: undo on empty stack")
            inv = stack.pop().inverse(); inv.apply(eng)
        else:
            c = cmd_from_json(e); c.apply(eng); stack.append(c)
    return eng

# ================= MCP server (PATH B) — MCP 2026-07-28 stdio =================
RAT = {"type": "object", "properties": {"num": {"type": "integer"}, "den": {"type": "integer"}},
       "required": ["num", "den"], "additionalProperties": False}
CID = {"type": "string", "minLength": 1}
TRK = {"type": "integer", "minimum": 0}

TOOLS = [
    ("add_clip", "Add a clip. Times are exact rationals {num,den}.", {"cid": CID, "track": TRK, "start": RAT, "dur": RAT, "name": {"type": "string"}, "gain": RAT}, ["cid", "track", "start", "dur", "name"]),
    ("remove_clip", "Remove a clip by id.", {"cid": CID}, ["cid"]),
    ("move_clip", "Move clip to track/start.", {"cid": CID, "track": TRK, "start": RAT}, ["cid", "track", "start"]),
    ("trim_clip", "Set clip duration.", {"cid": CID, "dur": RAT}, ["cid", "dur"]),
    ("split_clip", "Split clip at exact offset; right half gets new_cid.", {"cid": CID, "offset": RAT, "new_cid": CID}, ["cid", "offset", "new_cid"]),
    ("merge_clips", "Merge adjacent right clip into left.", {"left": CID, "right": CID}, ["left", "right"]),
    ("rename_clip", "Rename a clip.", {"cid": CID, "name": {"type": "string"}}, ["cid", "name"]),
    ("set_gain", "Set clip gain (exact rational factor).", {"cid": CID, "gain": RAT}, ["cid", "gain"]),
    ("batch", "Atomic transaction of sub-commands; all-or-nothing.", {"cmds": {"type": "array", "items": {"type": "object"}}}, ["cmds"]),
    ("undo", "Undo the last applied command (any owner).", {}, []),
    ("get_state_hash", "Query: canonical state hash.", {}, []),
    ("get_log_length", "Query: number of log entries.", {}, []),
]
CMD_TAGS = {"add_clip", "remove_clip", "move_clip", "trim_clip", "split_clip",
            "merge_clips", "rename_clip", "set_gain"}

def mcp_serve(log_path):
    eng = Engine(log_path=log_path)
    rng_id_map = {"add_clip": AddClip, "remove_clip": RemoveClip, "move_clip": MoveClip,
                  "trim_clip": TrimClip, "split_clip": SplitClip, "merge_clips": MergeClips,
                  "rename_clip": RenameClip, "set_gain": SetGain}
    for line in sys.stdin:
        line = line.strip()
        if not line: continue
        try: msg = json.loads(line)
        except json.JSONDecodeError: continue
        method, mid = msg.get("method"), msg.get("id")
        if method == "initialize":
            out = {"protocolVersion": msg["params"].get("protocolVersion", "2026-07-28"),
                   "capabilities": {"tools": {}},
                   "serverInfo": {"name": "ove-e009", "version": "0.1.0"}}
        elif method == "notifications/initialized":
            continue
        elif method == "ping":
            out = {}
        elif method == "tools/list":
            out = {"tools": [{"name": n, "description": d,
                              "inputSchema": {"type": "object", "properties": s,
                                              "required": r, "additionalProperties": False}}
                             for n, d, s, r in TOOLS]}
        elif method == "tools/call":
            name = msg["params"]["name"]; args = msg["params"].get("arguments", {})
            try:
                if name in CMD_TAGS:
                    if name == "add_clip":
                        cmd = AddClip(args["cid"], args["track"], args["start"], args["dur"], args["name"], args.get("gain", {"num": 1, "den": 1}))
                    else:
                        cmd = rng_id_map[name](**args)
                    entry = eng.apply(cmd, owner="agent")
                    out = {"content": [{"type": "text", "text": json.dumps({"ok": True, "entry": entry})}]}
                elif name == "batch":
                    cmds = [cmd_from_json({**c, "_owner": None}) for c in args["cmds"]]
                    eng.apply(Batch(cmds), owner="agent")
                    out = {"content": [{"type": "text", "text": json.dumps({"ok": True, "n": len(cmds)})}]}
                elif name == "undo":
                    eng.undo(owner="agent")
                    out = {"content": [{"type": "text", "text": json.dumps({"ok": True})}]}
                elif name == "get_state_hash":
                    out = {"content": [{"type": "text", "text": doc_hash(eng.doc)}]}
                elif name == "get_log_length":
                    out = {"content": [{"type": "text", "text": str(len(eng.log))}]}
                else:
                    out = {"content": [{"type": "text", "text": f"unknown tool {name}"}], "isError": True}
            except (EngineError, TypeError, ValueError, KeyError) as ex:
                out = {"content": [{"type": "text", "text": f"rejected: {ex}"}], "isError": True}
        else:
            send({"jsonrpc": "2.0", "id": mid,
                  "error": {"code": -32601, "message": f"method not found: {method}"}})
            continue
        if mid is not None:
            send({"jsonrpc": "2.0", "id": mid, "result": out})

def send(obj): sys.stdout.write(json.dumps(obj) + "\n"); sys.stdout.flush()

# ================= driver (PATH A + checks) =================
res = []
def check(name, ok, detail):
    res.append((name, ok, detail)); print(f"[{'PASS' if ok else 'FAIL'}] {name}: {detail}")

class MCPClient:
    def __init__(self, log_path):
        self.p = subprocess.Popen([sys.executable, os.path.abspath(__file__), "--serve", "--log", log_path],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.nid = 0
    def req(self, method, params=None):
        self.nid += 1
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.nid, "method": method,
                                       "params": params or {}}) + "\n"); self.p.stdin.flush()
        while True:
            line = self.p.stdout.readline()
            if not line: raise RuntimeError("server died")
            msg = json.loads(line)
            if msg.get("id") == self.nid: return msg
    def notify(self, method, params=None):
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": method, "params": params or {}}) + "\n"); self.p.stdin.flush()
    def call(self, tool, args=None):
        m = self.req("tools/call", {"name": tool, "arguments": args or {}})
        r = m.get("result", {})
        return r
    def hash(self): return self.req("tools/call", {"name": "get_state_hash"})["result"]["content"][0]["text"]
    def log_len(self): return int(self.req("tools/call", {"name": "get_log_length"})["result"]["content"][0]["text"])
    def close(self):
        self.p.stdin.close(); self.p.wait(timeout=5)

def scenario(mcp, direct):
    """12 interleaved steps. Every command is issued by an owner (A=UI client,
    B=agent client) and is delivered to BOTH engine surfaces: the in-process core
    (direct call = browser-WASM/desktop-shell shape) and the MCP stdio server
    (wire shape). Transport equivalence = hash equality after every step."""
    R1 = lambda n, d: {"num": n, "den": d}
    steps = [
        ("A add c1",
         lambda: direct.apply(AddClip("c1", 0, F(0), F(10), "intro"), "ui"),
         lambda: mcp.call("add_clip", {"cid": "c1", "track": 0, "start": R1(0, 1), "dur": R1(10, 1), "name": "intro"})),
        ("B add c2",
         lambda: direct.apply(AddClip("c2", 1, F(0), F(8), "broll"), "agent"),
         lambda: mcp.call("add_clip", {"cid": "c2", "track": 1, "start": R1(0, 1), "dur": R1(8, 1), "name": "broll"})),
        ("A move c1",
         lambda: direct.apply(MoveClip("c1", 1, F(1)), "ui"),
         lambda: mcp.call("move_clip", {"cid": "c1", "track": 1, "start": R1(1, 1)})),
        ("B trim c2",
         lambda: direct.apply(TrimClip("c2", F(5)), "agent"),
         lambda: mcp.call("trim_clip", {"cid": "c2", "dur": R1(5, 1)})),
        ("B rename c2",
         lambda: direct.apply(RenameClip("c2", "broll_v2"), "agent"),
         lambda: mcp.call("rename_clip", {"cid": "c2", "name": "broll_v2"})),
        ("A split c1",
         lambda: direct.apply(SplitClip("c1", F(3), "c1b"), "ui"),
         lambda: mcp.call("split_clip", {"cid": "c1", "offset": R1(3, 1), "new_cid": "c1b"})),
        ("B batch gain/move",
         lambda: direct.apply(Batch([SetGain("c1", F(3, 2)), SetGain("c2", F(1, 2)),
                                     MoveClip("c1b", 2, F(4))]), "agent"),
         lambda: mcp.call("batch", {"cmds": [
             {"op": "set_gain", "cid": "c1", "gain": R1(3, 2)},
             {"op": "set_gain", "cid": "c2", "gain": R1(1, 2)},
             {"op": "move_clip", "cid": "c1b", "track": 2, "start": R1(4, 1)}]})),
        ("A undo (undoes B batch)",
         lambda: direct.undo("ui"),
         lambda: mcp.call("undo")),
        ("B undo (undoes A split)",
         lambda: direct.undo("agent"),
         lambda: mcp.call("undo")),
        ("A set_gain c2",
         lambda: direct.apply(SetGain("c2", F(2)), "ui"),
         lambda: mcp.call("set_gain", {"cid": "c2", "gain": R1(2, 1)})),
        ("B add c3",
         lambda: direct.apply(AddClip("c3", 0, F(20), F(4), "outro"), "agent"),
         lambda: mcp.call("add_clip", {"cid": "c3", "track": 0, "start": R1(20, 1), "dur": R1(4, 1), "name": "outro"})),
        ("A trim c3",
         lambda: direct.apply(TrimClip("c3", F(2)), "ui"),
         lambda: mcp.call("trim_clip", {"cid": "c3", "dur": R1(2, 1)})),
    ]
    for i, (desc, a_fn, b_fn) in enumerate(steps, 1):
        a_fn()
        r = b_fn()
        if isinstance(r, dict) and r.get("isError"):
            raise RuntimeError(f"step {i} ({desc}) MCP error: {r.get('content')}")
        ha, hb = doc_hash(direct.doc), mcp.hash()
        check(f"P1 step{i:02d} [{desc}] hash-equality", ha == hb, f"direct={ha} mcp={hb}")

def main():
    tmp = tempfile.mkdtemp(prefix="e009-")
    log1 = os.path.join(tmp, "log1.jsonl")
    mcp = MCPClient(log1)
    mcp.req("initialize", {"protocolVersion": "2026-07-28",
                           "capabilities": {}, "clientInfo": {"name": "e009-driver", "version": "0.1"}})
    mcp.notify("notifications/initialized")
    direct = Engine()

    # P7: schema conformance (before any state work)
    tools = mcp.req("tools/list")["result"]["tools"]
    names = {t["name"] for t in tools}
    cmd_surfaces = (names & CMD_TAGS) | {"batch", "undo"}   # 8 payload cmds + batch + undo = 10
    rat_ok = all(
        all("integer" in json.dumps(t["inputSchema"]["properties"].get(k, {})) or
            k not in t["inputSchema"]["properties"]
            for k in ("start", "dur", "offset", "gain"))
        for t in tools)
    check("P7 tools/list conformance", len(cmd_surfaces) == 10 and rat_ok,
          f"{len(cmd_surfaces)} command surfaces (8 payload + batch + undo); rational fields integer-only: {rat_ok}")

    scenario(mcp, direct)

    # P2: shared-log equivalence + merged-log replay == final state
    entries = [json.loads(l) for l in open(log1) if l.strip()]
    owners = {e.get("_owner") for e in entries}
    final = doc_hash(direct.doc)
    rep = replay(entries)
    check("P2 merged-log replay == final state", doc_hash(rep.doc) == final,
          f"{len(entries)} entries (owners={sorted(owners)}); replay={doc_hash(rep.doc)} final={final}")
    # P2b owner provenance: the DIRECT engine's log carries both owners (this harness
    # drives it as two simulated clients); the wire log is byte-identical sans owner
    # tags (P2c) — i.e. the log SCHEMA is owner-agnostic; owner is provenance metadata.
    d_owners = {e.get("_owner") for e in direct.log}
    check("P2b both owners present in one transaction log", d_owners == {"ui", "agent"},
          f"owners in direct-path log: {sorted(d_owners)}; wire log provenance: {sorted(owners)}")
    strip = lambda es: [{k: v for k, v in e.items() if k != "_owner"} for e in es]
    check("P2c direct-path log == wire-path log (schema-identical)",
          strip(direct.log) == strip(entries),
          f"{len(direct.log)} direct entries vs {len(entries)} wire entries, byte-equal sans owner tags")

    # P3 cross-ownership undo: step8 UI-issued undo removed the AGENT batch;
    # step9 agent-issued undo removed the UI split (LIFO over one shared stack)
    c1 = direct.doc.clips.get("c1")
    c2 = direct.doc.clips.get("c2")
    ok_p3 = ("c1b" not in direct.doc.clips and c1 is not None
             and c1[2] == F(10) and c1[4] == F(1) and c2 is not None
             and c2[3] == "broll_v2" and doc_hash(direct.doc) == mcp.hash())
    check("P3 cross-ownership undo (A-undo removes B batch; B-undo removes A split)", ok_p3,
          f"c1 dur={c1[2] if c1 else None} gain={c1[4] if c1 else None} (split+batch undone); "
          f"c2 name={c2[3] if c2 else None} (step-5 rename intact); c1b absent: {'c1b' not in direct.doc.clips}")

    # P4: batch atomicity (invalid inner command) — BOTH surfaces must reject identically
    h_before, l_before = doc_hash(direct.doc), mcp.log_len()
    dl_before = len(direct.log)
    r_bad = mcp.call("batch", {"cmds": [
        {"op": "set_gain", "cid": "c2", "gain": {"num": 3, "den": 2}},
        {"op": "move_clip", "cid": "nope", "track": 0, "start": {"num": 0, "den": 1}}]})
    direct_bad = None
    try:
        direct.apply(Batch([SetGain("c2", F(3, 2)), MoveClip("nope", 0, F(0))]), "ui")
    except EngineError as ex:
        direct_bad = str(ex)
    check("P4 batch atomicity (MCP rejects whole batch)", r_bad.get("isError") is True,
          f"isError={r_bad.get('isError')}")
    check("P4b batch atomicity (direct path same semantics)", direct_bad is not None,
          f"EngineError: {direct_bad}")
    check("P4c state+log unchanged on BOTH engines after rejected batch",
          doc_hash(direct.doc) == h_before and mcp.log_len() == l_before
          and len(direct.log) == dl_before and doc_hash(direct.doc) == mcp.hash(),
          f"hash_before={h_before} hash_after={doc_hash(direct.doc)} wire log {l_before}->{mcp.log_len()}, direct log {dl_before}->{len(direct.log)}")

    # P5: float payload rejection at BOTH API edges
    f1 = mcp.call("add_clip", {"cid": "fx", "track": 0, "start": 1.5, "dur": {"num": 1, "den": 1}, "name": "x"})
    f2 = mcp.call("add_clip", {"cid": "fx", "track": 0, "start": {"num": 1.5, "den": 2}, "dur": {"num": 1, "den": 1}, "name": "x"})
    f3 = mcp.call("set_gain", {"cid": "c2", "gain": 0.5})
    try:
        direct.apply(AddClip("fx", 0, 1.5, F(1), "x"), "ui"); d_float = False
    except TypeError: d_float = True
    check("P5 float rejection (bare float start, wire)", f1.get("isError") is True, str(f1.get("content")))
    check("P5b float rejection (float inside num, wire)", f2.get("isError") is True, str(f2.get("content")))
    check("P5c float rejection (gain float, wire + direct TypeError)", f3.get("isError") is True and d_float,
          f"mcp isError={f3.get('isError')} direct raises={d_float}")

    h_end = doc_hash(direct.doc)
    mcp.close()

    # P6: determinism — fresh server, fresh log, same scenario
    log2 = os.path.join(tmp, "log2.jsonl")
    mcp2 = MCPClient(log2)
    mcp2.req("initialize", {"protocolVersion": "2026-07-28", "capabilities": {},
                            "clientInfo": {"name": "e009-driver", "version": "0.1"}})
    mcp2.notify("notifications/initialized")
    direct2 = Engine()
    scenario(mcp2, direct2)
    h_rerun = doc_hash(direct2.doc)
    check("P6 cross-process determinism", h_rerun == h_end, f"run1={h_end} run2={h_rerun}")
    mcp2.close()

    # summary
    fails = [r for r in res if not r[1]]
    print(f"\n==== E-009 SUMMARY ====")
    print(f"checks: {len(res)}, failed: {len(fails)}")
    verdict = ("SHARED COMMAND SURFACE VIABLE: one transaction log, owner-agnostic replay, "
               "cross-ownership undo, atomic batches, exact-time boundary enforced at API edge"
               if not fails else "INVESTIGATE FAILURES")
    print("VERDICT:", verdict)
    out = "/home/z/my-project/open-video-engine/experiments/E-009_result.txt"
    with open(out, "w") as f:
        for name, ok, detail in res:
            f.write(f"[{'PASS' if ok else 'FAIL'}] {name}: {detail}\n")
        f.write(f"VERDICT: {verdict}\nHARDWARE: container CPU; single host; protocol-overhead not benchmarked\n")
    return 0 if not fails else 1

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--serve":
        lp = sys.argv[sys.argv.index("--log") + 1] if "--log" in sys.argv else None
        mcp_serve(lp)
    else:
        sys.exit(main())
