#!/usr/bin/env python3
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os

ACCOUNT = "a" * 32
ZONE = "b" * 32

class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass
    def send(self, result, status=200, errors=None):
        body = json.dumps({"success": status < 300, "result": result, "errors": errors or []}).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def do_GET(self):
        path = self.path
        if path == f"/accounts/{ACCOUNT}/tokens/verify":
            return self.send({"status": "active"})
        if path.startswith(f"/zones/{ZONE}/ssl/certificate_packs"):
            return self.send([{"id":"cert1","type":"advanced","status":"active","hosts":["molehill.cloud","login.oteryn.molehill.cloud"]}])
        if f"/zones/{ZONE}/settings/" in path:
            name = path.rsplit("/",1)[-1]
            values = {"always_use_https":"off","min_tls_version":"1.3","security_level":"under_attack","browser_check":"on","security_header":{"strict_transport_security":{"enabled":False,"max_age":0}}}
            return self.send({"id":name,"value":values[name],"editable":True})
        if path == f"/zones/{ZONE}/rulesets":
            return self.send([{"id":"r1","phase":"http_request_dynamic_redirect","kind":"zone","name":"redirects"},{"id":"r2","phase":"http_request_firewall_custom","kind":"zone","name":"waf"}])
        if path == f"/zones/{ZONE}/rulesets/r1":
            return self.send({"rules":[{"id":"x1","ref":"oteryn-http-redirect","action":"redirect","enabled":True,"expression":"http.host eq \"oteryn.molehill.cloud\""}]})
        if path == f"/zones/{ZONE}/rulesets/r2":
            return self.send({"rules":[]})
        if path == f"/zones/{ZONE}/bot_management":
            return self.send({"fight_mode":True,"enable_js":True})
        if path.startswith(f"/accounts/{ACCOUNT}/access/apps"):
            return self.send([{"id":"app1","domain":"oteryn.molehill.cloud","type":"self_hosted"}])
        return self.send(None, status=404, errors=[{"code":1000,"message":"not found"}])

HTTPServer(("127.0.0.1", int(os.environ.get("PORT","18080"))), Handler).serve_forever()
