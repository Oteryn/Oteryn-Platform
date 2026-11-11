"""Capture reference outputs without modifying legacy/services/game-gateway.

Run from this directory with a Go compiler on PATH. The generated vectors include
all calls made by the reference wire, error, issuer, limiter and handler tests.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
SOURCE = HERE.parents[3] / "legacy/services/game-gateway"

with tempfile.TemporaryDirectory(prefix="gateway-reference-") as directory:
    root = Path(directory)
    shutil.copytree(SOURCE, root, dirs_exist_ok=True)
    native = root / "internal/nativelogin"
    wrappers = []
    definitions = [
        ("wire.go", "ParseRequest", "body []byte", "(Request, *Refusal)", "body", '''r, e := referenceParseRequest(body)
        row := map[string]any{"kind":"request", "input":string(body)}
        if e != nil { row["code"] = e.Code; row["status"] = e.Status(); row["output"] = encoded(e.Body(AttemptRefEcho(body))) } else { b, _ := EncodeIssuerRequest(r); row["output"] = string(b) }
        capture(row); return r, e'''),
        ("wire.go", "IsNativeRequest", "body []byte", "bool", "body", '''r := referenceIsNativeRequest(body); capture(map[string]any{"kind":"native", "input":string(body), "result":r}); return r'''),
        ("wire.go", "AttemptRefEcho", "body []byte", "*string", "body", '''r := referenceAttemptRefEcho(body); capture(map[string]any{"kind":"echo", "input":string(body), "result":r}); return r'''),
        ("wire.go", "parseSuccess", "body []byte, request Request", "(Success, error)", "body, request", '''r, e := referenceparseSuccess(body, request); row := map[string]any{"kind":"success", "input":string(body), "request":request, "valid":e == nil}; if e == nil {row["output"] = encoded(r)}; capture(row); return r, e'''),
        ("errors.go", "parseIssuerError", "status int, body []byte, request Request", "(*Refusal, error)", "status, body, request", '''r, e := referenceparseIssuerError(status, body, request); row := map[string]any{"kind":"error", "input":string(body), "status":status, "request":request, "valid":e == nil}; if e == nil {row["code"] = r.Code; row["output"] = encoded(r.Body(&request.AttemptRef))}; capture(row); return r, e'''),
    ]
    for filename, name, arguments, result, call, implementation in definitions:
        path = native / filename
        path.write_text(path.read_text().replace("func " + name + "(", "func reference" + name + "("))
        wrappers.append(f"func {name}({arguments}) {result} {{ {implementation} }}")
    path = native / "limiter.go"
    path.write_text(path.read_text().replace("func (l *windowLimiter) allow(", "func (l *windowLimiter) referenceAllow("))
    wrappers.append('''func (l *windowLimiter) allow(key string) (bool, int) {
        ok, retry := l.referenceAllow(key)
        capture(map[string]any{"kind":"limiter", "instance":fmt.Sprintf("%p", l), "limit":l.limit, "window":int64(l.window), "max_keys":l.maxKeys, "now":l.now().UnixNano(), "key":fmt.Sprintf("%x", []byte(key)), "allowed":ok, "retry":retry})
        return ok, retry
    }''')
    path = native / "handler_test.go"
    path.write_text(path.read_text().replace("return recorder", '''capture(map[string]any{"kind":"handler", "instance":fmt.Sprintf("%p", handler), "input":body, "remote":remoteAddr, "now":handler.sources.now().UnixNano(), "status":recorder.Code, "output":recorder.Body.String(), "retry":recorder.Header().Get("Retry-After"), "calls":handler.issuer.(*fakeAdmitter).calls, "success":handler.issuer.(*fakeAdmitter).success, "refusal":handler.issuer.(*fakeAdmitter).refusal})
    return recorder''').replace('"context"', '"context"\n"fmt"'))
    output = root / "vectors.jsonl"
    (native / "capture_test.go").write_text('''package nativelogin
import ("bytes"; "encoding/json"; "fmt"; "os"; "sync")
var captureLock sync.Mutex
func encoded(v any) string { var b bytes.Buffer; e := json.NewEncoder(&b); e.SetEscapeHTML(false); _ = e.Encode(v); return b.String() }
func capture(v any) { captureLock.Lock(); defer captureLock.Unlock(); f, err := os.OpenFile(os.Getenv("GATEWAY_VECTOR_OUTPUT"), os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0600); if err != nil { panic(err) }; defer f.Close(); _ = json.NewEncoder(f).Encode(v) }
''' + "\n".join(wrappers))
    subprocess.run(["go", "test", "./internal/nativelogin", "-count=1"], cwd=root,
                   env={**os.environ, "GATEWAY_VECTOR_OUTPUT": str(output)}, check=True)
    # Retain limiter/handler order; deduplicate stateless vectors.
    rows, seen = [], set()
    for line in output.read_text().splitlines():
        row = json.loads(line)
        canonical = json.dumps(row, sort_keys=True, ensure_ascii=True)
        if row["kind"] in ("limiter", "handler") or canonical not in seen:
            rows.append(canonical)
            seen.add(canonical)
    (HERE / "fixtures").mkdir(exist_ok=True)
    (HERE / "fixtures/go_vectors.jsonl").write_text("\n".join(rows) + "\n")
    print(f"Captured {len(rows)} reference vectors")
