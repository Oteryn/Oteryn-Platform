#!/usr/bin/env python3
"""GET-only sanitized audit of the remaining Oteryn Cloudflare edge controls."""
from __future__ import annotations
import json, os, re, sys, urllib.error, urllib.request
from datetime import datetime, timezone
from pathlib import Path

BASE=os.getenv("CLOUDFLARE_API_BASE_URL","https://api.cloudflare.com/client/v4").rstrip("/")
TOKEN=os.getenv("CLOUDFLARE_API_TOKEN",""); ACCOUNT=os.getenv("CLOUDFLARE_ACCOUNT_ID",""); ZONE=os.getenv("CLOUDFLARE_ZONE_ID","")
WWW="oteryn.molehill.cloud"; LOGIN="login.oteryn.molehill.cloud"
OUT=Path(os.getenv("CLOUDFLARE_EDGE_AUDIT_OUT","cloudflare-edge-audit"))
PHASES={"http_request_dynamic_redirect","http_request_firewall_custom","http_response_headers_transform","http_config_settings"}

def die(msg): print(f"ERROR: {msg}",file=sys.stderr); raise SystemExit(1)
def call(path):
    req=urllib.request.Request(BASE+path,headers={"Authorization":f"Bearer {TOKEN}","Accept":"application/json"},method="GET")
    try:
        with urllib.request.urlopen(req,timeout=30) as r: status=r.status; raw=r.read(2_000_000)
    except urllib.error.HTTPError as e: status=e.code; raw=e.read(2_000_000)
    except Exception as e: return {"status":0,"state":"error","errors":[{"message":f"{type(e).__name__}: {e}"}]}
    try: data=json.loads(raw)
    except Exception: return {"status":status,"state":"error","errors":[{"message":"non-JSON response"}]}
    ok=200<=status<300 and data.get("success") is True
    state="readable" if ok else "permission_denied" if status in (401,403) else "not_found_or_unavailable" if status==404 else "error"
    errors=[{"code":x.get("code"),"message":str(x.get("message",""))[:300]} for x in data.get("errors",[]) if isinstance(x,dict)]
    return {"status":status,"state":state,"result":data.get("result") if ok else None,"errors":errors}
def certs(r):
    out={"state":r["state"],"http_status":r["status"]}
    if r["state"]!="readable": out["errors"]=r["errors"]; return out
    packs=r["result"] if isinstance(r["result"],list) else []; hits=[]
    for x in packs:
        hosts=[str(h).lower() for h in x.get("hosts",[]) if isinstance(h,str)]
        if LOGIN in hosts: hits.append({"id":x.get("id"),"type":x.get("type"),"status":x.get("status"),"host_count":len(hosts)})
    out.update(pack_count=len(packs),matching_packs=hits,active_exact_login_coverage=any(str(x.get("status","")).lower()=="active" for x in hits)); return out
def setting(name,r):
    out={"state":r["state"],"http_status":r["status"]}
    if r["state"]=="readable" and isinstance(r["result"],dict): out.update(id=r["result"].get("id",name),value=r["result"].get("value"),editable=r["result"].get("editable"))
    else: out["errors"]=r["errors"]
    return out
def rulesets(r):
    out={"state":r["state"],"http_status":r["status"]}; selected=[]
    if r["state"]!="readable": out["errors"]=r["errors"]; return out,selected
    for x in r["result"] if isinstance(r["result"],list) else []:
        if isinstance(x,dict) and x.get("phase") in PHASES: selected.append({k:x.get(k) for k in ("id","phase","kind","name")})
    out["relevant_rulesets"]=selected; return out,selected
def rules_detail(meta,r):
    out={"id":meta.get("id"),"phase":meta.get("phase"),"state":r["state"],"http_status":r["status"]}
    if r["state"]!="readable" or not isinstance(r["result"],dict): out["errors"]=r["errors"]; return out
    rules=r["result"].get("rules",[]); hits=[]
    for x in rules if isinstance(rules,list) else []:
        exp=str(x.get("expression",""))
        if WWW in exp or LOGIN in exp: hits.append({"id":x.get("id"),"ref":x.get("ref"),"action":x.get("action"),"enabled":x.get("enabled",True),"matches_www":WWW in exp,"matches_login":LOGIN in exp})
    out.update(rule_count=len(rules) if isinstance(rules,list) else 0,oteryn_matching_rules=hits); return out
def bot(r):
    out={"state":r["state"],"http_status":r["status"]}; keys=("fight_mode","sbfm_likely_automated","sbfm_definitely_automated","sbfm_verified_bots","sbfm_static_resource_protection","enable_js")
    if r["state"]!="readable" or not isinstance(r["result"],dict): out["errors"]=r["errors"]; return out
    out["settings"]={k:r["result"].get(k) for k in keys if k in r["result"]}; return out
def access(r):
    out={"state":r["state"],"http_status":r["status"]}
    if r["state"]!="readable": out["errors"]=r["errors"]; return out
    apps=r["result"] if isinstance(r["result"],list) else []; hits=[]
    for x in apps:
        d=str(x.get("domain","")).lower()
        if d in (WWW,LOGIN) or d.startswith(WWW+"/") or d.startswith(LOGIN+"/"): hits.append({"id":x.get("id"),"domain":d,"type":x.get("type")})
    out.update(application_count=len(apps),oteryn_applications=hits); return out
def main():
    if not TOKEN: die("CLOUDFLARE_API_TOKEN is missing")
    if not re.fullmatch(r"[0-9a-fA-F]{32}",ACCOUNT): die("invalid CLOUDFLARE_ACCOUNT_ID")
    if not re.fullmatch(r"[0-9a-fA-F]{32}",ZONE): die("invalid CLOUDFLARE_ZONE_ID")
    token=call(f"/accounts/{ACCOUNT}/tokens/verify" if TOKEN.startswith("cfat_") else "/user/tokens/verify")
    if token["state"]!="readable" or not isinstance(token["result"],dict) or token["result"].get("status")!="active": die(f"token verification failed: HTTP {token['status']}")
    settings={n:setting(n,call(f"/zones/{ZONE}/settings/{n}")) for n in ("always_use_https","min_tls_version","security_level","browser_check","security_header")}
    rs,selected=rulesets(call(f"/zones/{ZONE}/rulesets")); details=[rules_detail(x,call(f"/zones/{ZONE}/rulesets/{x['id']}")) for x in selected if x.get("id")]
    evidence={"observed_at_utc":datetime.now(timezone.utc).isoformat(),"classification":"READ_ONLY_CLOUDFLARE_EDGE_AUDIT","canonical_hosts":[WWW,LOGIN],"token":{"active":True,"verification_scope":"account" if TOKEN.startswith("cfat_") else "user"},"certificate_packs":certs(call(f"/zones/{ZONE}/ssl/certificate_packs?status=all&per_page=100")),"zone_settings":settings,"rulesets":rs,"ruleset_details":details,"bot_management":bot(call(f"/zones/{ZONE}/bot_management")),"access_applications":access(call(f"/accounts/{ACCOUNT}/access/apps?per_page=100")),"mutation":"none"}
    OUT.mkdir(parents=True,exist_ok=True); (OUT/"evidence.json").write_text(json.dumps(evidence,indent=2,sort_keys=True)+"\n")
    lines=["# Cloudflare Oteryn edge audit","",f"Observed at: `{evidence['observed_at_utc']}`","",f"- certificate_packs: `{evidence['certificate_packs']['state']}`; active exact login coverage: `{evidence['certificate_packs'].get('active_exact_login_coverage','unknown')}`",f"- rulesets: `{rs['state']}`; relevant count: `{len(rs.get('relevant_rulesets',[]))}`",f"- bot_management: `{evidence['bot_management']['state']}`",f"- access_applications: `{evidence['access_applications']['state']}`"]
    lines += [f"- zone setting `{n}`: `{x['state']}`; value: `{x.get('value','unknown')}`" for n,x in settings.items()]; lines += ["","This audit performs GET requests only and writes a sanitized artifact.",""]
    text="\n".join(lines); (OUT/"summary.md").write_text(text); print(text)
    if os.getenv("GITHUB_STEP_SUMMARY"): open(os.environ["GITHUB_STEP_SUMMARY"],"a").write(text)
if __name__=="__main__": main()
