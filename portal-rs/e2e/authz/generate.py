#!/usr/bin/env python3
"""Extract Axum route registrations and the built-in permission catalog."""
import argparse
import json
import re
from pathlib import Path

PORTAL = Path(__file__).resolve().parents[2]
OUTPUT = Path(__file__).with_name("routes.json")
METHODS = ("get", "head", "post", "put", "patch", "delete", "options", "trace", "connect")
STRING = r'"(?:[^"\\]|\\.)*"'
TOKEN = re.compile(r'r(?P<hash>\#*)".*?"(?P=hash)|' + STRING + r'|//[^\n]*|/\*.*?\*/|[()\[\]{},]', re.S)


def code_only(source):
    """Remove comments without shifting source locations."""
    return TOKEN.sub(lambda m: re.sub(r'[^\n]', ' ', m[0])
                     if m[0].startswith(("//", "/*")) else m[0], source)


def arguments(source, opening):
    """Split a call's arguments at the top level of its delimiters."""
    stack = ["("]
    start = opening + 1
    for match in TOKEN.finditer(source, start):
        token = match[0]
        if token in ("(", "[", "{"):
            stack.append(token)
        elif token in (")", "]", "}"):
            stack.pop()
            if not stack:
                yield source[start:match.start()].strip()
                return
        elif token == "," and len(stack) == 1:
            yield source[start:match.start()].strip()
            start = match.end()
    raise ValueError("unterminated Rust call")


def sources():
    for file in sorted((PORTAL / "crates").glob("*/src/**/*.rs")):
        if "tests" in file.parts or re.search(r'(?:^|_)(?:test|tests|test_db)\.rs$', file.name):
            continue
        source = code_only(file.read_text())
        for test_module in reversed(list(re.finditer(r"#\[cfg\(test\)\]\s*mod \w+\s*\{", source))):
            opening = test_module.end() - 1
            depth = 1
            for token in TOKEN.finditer(source, opening + 1):
                depth += (token[0] == "{") - (token[0] == "}")
                if depth == 0:
                    source = source[:test_module.start()] + re.sub(
                        r"[^\n]", " ", source[test_module.start():token.end()]
                    ) + source[token.end():]
                    break
        yield str(file.relative_to(PORTAL)), source


def literal(expression):
    match = re.fullmatch(STRING, expression)
    return json.loads(match[0]) if match else None


def strings(expression):
    return [json.loads(match[0]) for match in re.finditer(STRING, expression)]


def role_catalog():
    source = (PORTAL / "crates/admin/src/rbac/catalog.rs").read_text()
    permission_source, role_source = source.split("impl BuiltInRole {")
    permissions = dict(re.findall(r'Self::(\w+)\s*=>\s*"([^"]+)"', permission_source))
    roles = dict(re.findall(r'Self::(\w+)\s*=>\s*"([^"]+)"', role_source))
    defaults = {
        name: [permissions[p] for p in re.findall(r'Permission::(\w+)', body)]
        for name, body in re.findall(r'Self::(\w+)\s*=>\s*&\[(.*?)\]', role_source, re.S)
    }
    declared = re.findall(r'Self::(\w+)', role_source.split("];", 1)[0])
    if set(declared) != roles.keys() or roles.keys() != defaults.keys():
        raise ValueError("BuiltInRole::ALL, key and permissions disagree")
    return {roles[name]: defaults[name] for name in declared}, permissions


def route_calls(files):
    for file, source in files.items():
        quoted = [(m.start(), m.end()) for m in TOKEN.finditer(source)
                  if m[0].startswith(('"', 'r'))]
        for match in re.finditer(r'\.(route|nest_service)\s*\(', source):
            if any(start <= match.start() < end for start, end in quoted):
                continue
            path, handler, *_ = arguments(source, match.end() - 1)
            if match[1] == "nest_service":
                if not handler.startswith("ServeDir::new(") or literal(path) is None:
                    raise ValueError(f"{file}: unsupported nested service")
                path = json.dumps(literal(path).rstrip("/") + "/{*path}")
                handler = "get(static_file).head(static_file)"
            yield file, source, match.start(), path, handler


def section(source, pattern):
    matches = list(re.finditer(pattern, source, re.S))
    if len(matches) != 1:
        raise ValueError(f"expected exactly one match for {pattern!r}, got {len(matches)}")
    return matches[0][1]


def named_values(files, file, name):
    crate = file.split("/")[1]
    simple = name.split("::")[-1]
    if "::" in name:
        module = name.split("::")[0]
        candidates = [text for path, text in files.items()
                      if path.startswith(f"crates/{crate}/") and path.endswith(f"/{module}.rs")]
    else:
        candidates = [files[file]]
    pattern = rf'\bconst {simple}\s*:\s*&str\s*=\s*({STRING})\s*;'
    values = {literal(m) for text in candidates for m in re.findall(pattern, text)}
    sibling = str(Path(file).with_name("mod.rs"))
    if not values and sibling in files:
        values = {literal(m) for m in re.findall(pattern, files[sibling])}
    if not values and "::" not in name:
        values = {literal(m) for path, text in files.items()
                  if path.startswith(f"crates/{crate}/") for m in re.findall(pattern, text)}
    if len(values) != 1:
        raise ValueError(f"{file}: ambiguous constant {name}: {values}")
    return values.pop()


def handlers(expression):
    methods = re.findall(r'(?:^|\.|\.merge\()\s*(?:axum::routing::)?(' + "|".join((*METHODS, "any"))
                         + r')(?:\(|::)', expression)
    if methods == ["any"]:
        return dict.fromkeys(METHODS, expression)
    if not methods:
        raise ValueError(f"unresolved method router {expression}")
    return dict.fromkeys(methods, expression)


def path_variants(files, file, source, position, path, handler):
    """Resolve the small set of Rust loops, matches and aliases that register routes."""
    before = source[:position]
    group = file.removeprefix("crates/")
    resolve = lambda name: named_values(files, file, name)
    if handler.removesuffix(".clone()") in {"badge_routes", "profile_routes"}:
        name = handler.removesuffix(".clone()")
        handler = section(source, rf"let {name}\s*=\s*(get\(.*?);")
    direct = literal(path)
    if direct is not None:
        if group == "admin/src/control/http.rs" and not direct.startswith("/internal/"):
            direct = section(source, r'\.nest\(\s*"([^"]+)"') + direct
        return [(direct, handler, None)]
    if re.fullmatch(r'[A-Z][A-Z0-9_]*(?:::[A-Z_]+)?', path) or re.fullmatch(r'\w+::[A-Z_]+', path):
        return [(resolve(path), handler, None)]
    if group == "web/src/redirects.rs":
        pairs = section(source, r'const MAPPINGS[^=]+=\s*&\[(.*?)\];')
        return [(old, handler, None) for old in strings(pairs)[::2]]
    if group == "admin/src/console/access.rs":
        branch = re.search(r'(?:"(\w+)"|_)\s*=>\s*Router::new\(\)$', before)
        rows = re.findall(r'\(\s*"(/[^"]+)",\s*Permission::(\w+),\s*"(\w+)"', before)
        area = branch[1] or "access"
        return [(url, handler, permission) for url, permission, name in rows if name == area]
    if group == "admin/src/support_web.rs":
        suffix = "" if path == "base(area)" else "/{id}"
        return [(url + suffix, handler, "@" + variant) for variant, url in
                re.findall(r'Area::(\w+)\s*=>\s*"(/[^"]+)"', before)]
    if group == "characters/src/lcfa/http.rs":
        return [(resolve(name), handler, None) for name in re.findall(r'mount\(([A-Z_]+),', source)]
    if group == "payments/src/coin_http.rs" and path == "&path":
        path = "&" + section(before, r'let path = (format!\([^;]+);')
    if group == "minigames/src/league/config.rs":
        names = section(source, r'const INGRESS_PATHS[^=]+=\s*\[(.*?)\];').split(",")
        return [(resolve(name.strip()), handler, None) for name in names if name.strip()]
    if group == "minigames/src/league/transport.rs" and path == "path":
        names = section(before, r'for path in \[([^\]]+)\]').split(",")
        return [(resolve(name.strip()), handler, None) for name in names if name.strip()]
    if group == "community/src/ingest.rs":
        values = re.findall(r'\bpath: "([^"]+)"', before)
        values += re.findall(r'let path = family.map_or\("([^"]+)"', before)
        return [(f"/internal/v1/game-public/{value}", handler, None) for value in values]
    if group == "community/src/families.rs":
        family = section(source, r'pub fn path\(.*?match self \{(.*?)\}')
        values = strings(family) + re.findall(r'let path = family.map_or\("([^"]+)"', before)
        return [(f"/internal/v2/game-public/{value}", handler, None) for value in values]
    if group == "community/src/today/http.rs" and "{path}" in path:
        values = strings(section(before, r'let path = if watermark (.*?);'))
        return [(f"/internal/v2/game-public/{value}", handler, None) for value in values]
    if group == "playerdata/src/sessions.rs":
        array = section(source, r'const COMMANDS[^=]+=\s*\[(.*?)\];')
        return [(url, handler, None) for url in [resolve("RECORDS"), *strings(array)]]

    format_match = re.match(r'&format!\(\s*(' + STRING + r')', path)
    template = literal(format_match[1]) if format_match else path.lstrip("&")
    if group == "admin/src/analytics/mod.rs":
        template = literal(section(before, r'let path = format!\((' + STRING + r')\)'))
    tuples = list(re.finditer(r'for (\([^)]*\)|\w+) in\s*\[(.*?)\]\s*\{', before, re.S))

    def loop(variable, first_only=False):
        for match in reversed(tuples):
            if re.search(rf'\b{variable}\b', match[1]):
                array = match[2]
                if first_only and match[1].startswith("("):
                    return re.findall(r'\(\s*(' + STRING + r'|[A-Z][A-Z_0-9]*)\s*,', array)
                return re.findall(STRING, array)
        raise ValueError(f"no source array for {variable}")

    if template == "path" or any("{" + v + "}" in template for v in ("path", "operation", "route")):
        if "{operation}" in template:
            variable = "operation"
        elif "{route}" in template:
            variable = "route"
        else:
            variable = "path"
        values = loop(variable, True)
        rows = []
        for value in values:
            url = literal(value) or resolve(value)
            expression = handler
            if handler in {"handler", "route"}:
                array = next(m[2] for m in reversed(tuples) if re.search(rf'\b{variable}\b', m[1]))
                if group == "wallet/src/wire.rs":
                    method = re.search(re.escape(value) + r',\s*(true|false)', array)[1]
                    expression = "get(handle)" if method == "true" else "post(handle)"
                else:
                    expression = re.search(re.escape(value) + r',\s*(\w+\([^)]*\))', array)[1]
            adapted = url if template == "path" else template.replace("{" + variable + "}", url)
            rows.extend(expand_format(adapted, expression, loop, resolve, source, before))
        return rows
    if format_match:
        return expand_format(template, handler, loop, resolve, source, before)
    raise ValueError(f"unresolved path {path}")


def expand_format(template, handler, loop, resolve, source, before):
    variants = [template]
    for variable in re.findall(r'(?<!\{)\{(\w*)\}(?!\})', template):
        if not variable:
            segment = section(source, r'fn segment\(.*?match family \{(.*?)\}')
            families = re.findall(r'Family::(\w+)', section(before, r'for family in \[(.*?)\]'))
            values = re.findall(r'Family::(\w+) => "([^"]+)"', segment)
            values = [label for family, label in values if family in families]
        elif variable.isupper():
            values = [resolve(variable)]
        elif variable in {"prefix", "lang"}:
            # The houses loop adds an iterator modifier before its body.
            array = re.search(rf'for {variable} in \[([^\]]+)\]', before)
            values = strings(array[1]) if array else [literal(v) for v in loop(variable)]
        else:
            continue
        variants = [url.replace("{" + variable + "}", value) for url in variants for value in values]
    return [(url.replace("{{", "{").replace("}}", "}"), handler, None) for url in variants]


def guard_for(files, file, url, method, permission, permissions):
    """Resolve listener credentials and explicit staff router boundaries."""
    group = file.removeprefix("crates/")
    public = ("public", {"kind": "anonymous"})
    if group == "gameauth/src/admission/http.rs":
        return ("admission", {"kind": "service-token"}) if url != "/health" else (
            "admission", {"kind": "anonymous"})
    if url.startswith("/internal/"):
        if group == "identity/src/termination/game_closure/http.rs":
            listener = "closure"
        elif group == "bazaar/src/game/runtime/transport.rs":
            listener = "bazaar"
        elif group == "minigames/src/league/transport.rs":
            listener = "league"
        else:
            listener = "internal"
        return listener, {"kind": "mtls"}
    if group == "gateway/src/lib.rs":
        return "gateway", {"kind": "client-credentials"} if url.startswith("/v1/") else public[1]
    if url == "/" and method == "post":
        return "embedded", {"kind": "mtls"}
    if group == "internal-api/src/bazaar.rs":
        return "internal", {"kind": "mtls"}
    if group in {"player-api/src/lib.rs", "player-api/src/account_feed.rs"}:
        return "public", {"kind": "bearer"}
    if group == "gameauth/src/devices_http.rs":
        return "native", {"kind": "device-credentials"}
    if group in {"payments/src/provider_webhook.rs", "store/src/web_payments.rs"}:
        return "public", {"kind": "provider-signature"}

    path = re.sub(r'^/(?:pl|en|\{lang(?:uage)?\}|\{locale\})(?=/)', '', url)
    if path.startswith(("/admin", "/console/")):
        if group == "admin/src/analytics/web.rs":
            required = ["Access"]
            alternatives = ["AdminAnalyticsView", "GameAnalyticsRead"]
        else:
            alternatives = []
            if permission and permission.startswith("@"):
                area = permission[1:]
                permission = next((
                    value for source_path, rs in files.items()
                    if source_path.startswith("crates/admin/src/")
                    for value in re.findall(rf'(?:Self|Area)::{area}\s*=>\s*Permission::(\w+)', rs)
                ), None)
            if group == "admin/src/support_web.rs" and path.endswith("/enforcement/create"):
                permission = "ManageSupportEnforcement"
            if group == "admin/src/web.rs":
                predicate = r'path\(\) == "/admin/roles" \{\s*Permission::(\w+)'
                if path != "/admin/roles":
                    predicate += r'\s*\} else \{\s*Permission::(\w+)'
                    permission = re.search(predicate, files[file])[2]
                else:
                    permission = section(files[file], predicate)
            if not permission:
                boundary = re.findall(
                    r'\b(?:configured_guard(?:_from)?|guarded)\(.*?Permission::(\w+)',
                    files[file], re.S,
                )
                if len(set(boundary)) == 1:
                    permission = boundary[0]
            required = list(dict.fromkeys(["Access", permission])) if permission else []
        if not required:
            return "public", {"kind": "unresolved"}
        result = {"kind": "staff", "all": [permissions[p] for p in required]}
        if alternatives:
            result["any"] = [permissions[p] for p in alternatives]
        if group == "wiki/src/web.rs" and path == "/admin/wiki/articles/create":
            required = section(files[file], r'async fn create_form.*?\.authorize\(context, Permission::(\w+)')
            result["all"].append(permissions[required])
        if group == "admin/src/players/http.rs" and path == "/admin/players":
            required = section(files[file], r'async fn search.*?s.check\(context, Permission::(\w+)')
            result["all"].append(permissions[required])
        if group == "world/src/admin.rs":
            required = section(files[file], r'if let Err\(status\) = authorize\(&s, context, Permission::(\w+)')
            result["all"].append(permissions[required])
        if group == "admin/src/console/mod.rs":
            feature = path.rsplit("/", 1)[1]
            module = str(Path(file).with_name(f"{feature}.rs"))
            result["all"].append(section(files[module], r'const PERMISSION: &str = "([^"]+)"'))
        return "public", result

    if path.startswith(("/account/", "/panel/", "/support/tickets", "/support/reports",
                        "/password/change", "/atlas/private-markers", "/atlas/map-reports",
                        "/atlas/places", "/puzzles", "/collection", "/pass", "/shop",
                        "/api/v1/stats/watches")) or path == "/account":
        if "/passkeys/authenticate/" in path:
            return public
        return "public", {"kind": "logged-in"}
    if path in ("/mfa", "/mfa/enroll", "/mfa/confirm", "/mfa/devices", "/logout"):
        return "public", {"kind": "logged-in"}
    if path.startswith("/party") and method != "get":
        return "public", {"kind": "logged-in"}
    if group == "payments/src/coin_http.rs":
        if path.startswith("/api/"):
            return "public", {"kind": "bearer"}
        return "public", {"kind": "anonymous" if method not in ("get", "post") else "logged-in"}
    if group == "gameauth/src/oauth/oauth_http.rs":
        return "public", {"kind": "oauth-grant" if path == "/oauth/token" else "logged-in"}
    if (group == "identity/src/email_change_http.rs" and path.startswith("/email-change/")
            or group == "identity/src/account_lifecycle_http.rs" and path == "/unsubscribe"
            or group == "web/src/routes/verify.rs" and path == "/verify-email"):
        return "public", {"kind": "signed-link"}
    return public


def refusal_for(files, listener, file, url):
    """Protocol refusals that have exact outcomes independently of browser RBAC."""
    if listener == "internal":
        tls = (PORTAL / "bin/oteryn-portal/src/internal_tls.rs").read_text()
        marker = ".with_client_cert_verifier(verifier)"
        refusal = dict(tlsAlert="certificate_required",
                       source=f"bin/oteryn-portal/src/internal_tls.rs:{tls[:tls.index(marker)].count(chr(10)) + 1}")
        if url in ("/internal/v1/game-auth/native-scope-assignments",
                   "/internal/v1/game-auth/native-scope-revocations"):
            peer_source = "crates/gameauth/src/scopes.rs"
            source = files[peer_source]
            marker = "return guard::failure(StatusCode::UNAUTHORIZED)"
            line = source.count(chr(10), 0, source.index(marker)) + 1
            refusal["wrongIdentity"] = dict(status=401, source=f"{peer_source}:{line}")
        return refusal
    if listener == "admission" and url != "/health":
        marker = "credential.is_none() || !tokens.authenticates"
        status = 401
    elif url == "/__test/payments/webhook":
        marker = "let Some(signature) = signature else"
        status = 400
    else:
        return None
    source = files[file]
    position = source.index(marker)
    return dict(status=status, source=f"{file}:{source.count(chr(10), 0, position) + 1}")


def inventory(files, permissions):
    rows = {}
    for file, source, offset, path, handler in route_calls(files):
        line = source.count("\n", 0, offset) + 1
        try:
            for url, expression, permission in path_variants(files, file, source, offset, path, handler):
                for method in handlers(expression):
                    listener, guard = guard_for(files, file, url, method, permission, permissions)
                    row = rows.setdefault((listener, url, method), {
                        "listener": listener, "path": url, "method": method.upper(),
                        "registrations": []
                    })
                    item = {"source": f"{file}:{line}", "guard": guard}
                    refusal = refusal_for(files, listener, file, url)
                    if refusal:
                        item["refusal"] = refusal
                    row["registrations"].append(item)
        except (AttributeError, StopIteration, ValueError) as error:
            raise ValueError(f"{file}:{line}: {path}: {error}") from error
    return [rows[key] for key in sorted(rows)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = dict(sources())
    roles, permissions = role_catalog()
    result = {"roles": roles, "routes": inventory(files, permissions)}
    generated = json.dumps(result, indent=2) + "\n"
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text() != generated:
            parser.exit(1, f"{OUTPUT.name} is stale; rerun {Path(__file__).name}\n")
    else:
        OUTPUT.write_text(generated)


if __name__ == "__main__":
    main()
