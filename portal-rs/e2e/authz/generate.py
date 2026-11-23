#!/usr/bin/env python3
"""Extract Axum route registrations and the built-in permission catalog."""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

PORTAL = Path(__file__).resolve().parents[2]
OUTPUT = Path(__file__).with_name("routes.json")
METHODS = ("get", "head", "post", "put", "patch", "delete", "options", "trace", "connect")
STRING = r'"(?:[^"\\]|\\.)*"'
TOKEN_START = re.compile(r'r(?P<hashes>#{0,255})"|"|//|/\*|[()\[\]{},]')
QUOTED_STRING = re.compile(STRING, re.S)


def tokens(source, start=0):
    missing_end = set()
    while opening := TOKEN_START.search(source, start):
        first, end = opening.span()
        start = end
        text = opening[0]
        if text == '//':
            line_end = source.find('\n', end)
            end = len(source) if line_end == -1 else line_end
        elif text == '/*' or opening['hashes'] is not None:
            terminal = '*/' if text == '/*' else '"' + opening['hashes']
            closing = -1 if terminal in missing_end else source.find(terminal, end)
            if closing == -1:
                missing_end.add(terminal)
                if text != '/*':
                    start = end - 1
                continue
            end = closing + len(terminal)
        elif text == '"':
            if 'quoted string' in missing_end:
                continue
            quoted = QUOTED_STRING.match(source, first)
            if quoted is None:
                missing_end.add('quoted string')
                continue
            end = quoted.end()
        start = end
        yield source[first:end], first, end


def code_only(source):
    """Remove comments without shifting source locations."""
    parts = []
    start = 0
    for token, first, end in tokens(source):
        if token.startswith(("//", "/*")):
            parts.extend((source[start:first], re.sub(r'[^\n]', ' ', token)))
            start = end
    parts.append(source[start:])
    return ''.join(parts)


def arguments(source, opening):
    """Split a call's arguments at the top level of its delimiters."""
    stack = ["("]
    start = opening + 1
    for token, first, end in tokens(source, start):
        if token in ("(", "[", "{"):
            stack.append(token)
        elif token in (")", "]", "}"):
            stack.pop()
            if not stack:
                yield source[start:first].strip()
                return
        elif token == "," and len(stack) == 1:
            yield source[start:first].strip()
            start = end
    raise ValueError("unterminated Rust call")


def sources():
    for file in sorted(path for root in ("crates", "bin")
                       for path in (PORTAL / root).glob("*/src/**/*.rs")):
        if "tests" in file.parts or re.search(r'(?:^|_)(?:test|tests|test_db)\.rs$', file.name):
            continue
        source = code_only(file.read_text())
        for test_module in reversed(list(re.finditer(r"#\[cfg\(test\)\]\s*mod \w+\s*\{", source))):
            opening = test_module.end() - 1
            depth = 1
            for token, _, end in tokens(source, opening + 1):
                depth += (token == "{") - (token == "}")
                if depth == 0:
                    source = source[:test_module.start()] + re.sub(
                        r"[^\n]", " ", source[test_module.start():end]
                    ) + source[end:]
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
        quoted = [(first, end) for token, first, end in tokens(source)
                  if token.startswith(('"', 'r'))]
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
    root, crate, *_ = file.split("/")
    simple = name.split("::")[-1]
    if "::" in name:
        module = name.split("::")[0]
        candidates = [text for path, text in files.items()
                      if path.startswith(f"{root}/{crate}/") and path.endswith(f"/{module}.rs")]
    else:
        candidates = [files[file]]
    pattern = rf'\bconst {simple}\s*:\s*&str\s*=\s*({STRING})\s*;'
    values = {literal(m) for text in candidates for m in re.findall(pattern, text)}
    sibling = str(Path(file).with_name("mod.rs"))
    if not values and sibling in files:
        values = {literal(m) for m in re.findall(pattern, files[sibling])}
    if not values and "::" not in name:
        values = {literal(m) for path, text in files.items()
                  if path.startswith(f"{root}/{crate}/") for m in re.findall(pattern, text)}
    if len(values) != 1:
        raise ValueError(f"{file}: ambiguous constant {name}: {values}")
    return values.pop()


def handlers(expression):
    methods = re.findall(r'(?:^|\.|\.merge\()\s*(?:axum::routing::)?(' + "|".join((*METHODS, "any"))
                         + r')(?:_service)?(?:\(|::)', expression)
    if methods == ["any"]:
        return {method: method for method in METHODS}
    if not methods:
        raise ValueError(f"unresolved method router {expression}")
    result = {method: method for method in methods}
    if "get" in result and "head" not in result:
        result["head"] = "get"
    return result


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
    if file == "bin/oteryn-portal/src/game_routes.rs":
        if path == "path" and handler.startswith("post_service(sessions"):
            session_file = "crates/playerdata/src/sessions.rs"
            values = section(files[session_file], r'const COMMANDS[^=]+=\s*&?\[(.*?)\];')
            names = [name.strip() for name in values.split(",") if name.strip()]
            return [(literal(name) or named_values(files, session_file, name), handler, None) for name in names]
        if "family.path()" in path:
            family = files["crates/community/src/families.rs"]
            values = section(family, r'pub fn path\(.*?match self \{(.*?)\}')
            prefix = strings(path)[0].split("{}")[0]
            return [(prefix + suffix, handler, None) for suffix in strings(values)]
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
                if first_only:
                    values = [item.strip() for item in array.split(",") if item.strip()]
                    if all(literal(item) is not None or re.fullmatch(r"[A-Z][A-Z_0-9]*", item) for item in values):
                        return values
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
        return "native", {"kind": "anonymous" if "{*rest}" in url else "device-credentials"}
    if group in {"payments/src/provider_webhook.rs", "store/src/web_payments.rs"}:
        return "public", {"kind": "provider-signature"}

    path = re.sub(r'^/(?:pl|en|\{lang(?:uage)?\}|\{locale\})(?=/)', '', url)
    if file == "bin/oteryn-portal/src/code_routes.rs" and path == "/shop/codes/redeem":
        return "public", {"kind": "logged-in"}
    if path.startswith(("/admin", "/console/")) or path == "/console":
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
            if group == "admin/src/operations/mod.rs":
                code = files[file]
                routers = re.findall(r'let (\w+) = Router::new\(\)(.*?);', code, re.S)
                router = next(name for name, body in routers if json.dumps(path) in body)
                permission = section(code, rf'\(\s*{router},\s*Permission::(\w+)')
            if group.startswith("admin/src/commerce/panels/"):
                name = {
                    "/admin/payment-review": "payments::routes",
                    "/admin/store-catalog": "catalog::staging",
                    "/admin/store-activation": "catalog::activation",
                }[path]
                composition = files["crates/admin/src/commerce/panels/mod.rs"]
                permission = section(composition, rf'Permission::(\w+),\s*{name}\(')
            if group == "admin/src/commerce/http.rs" and path == "/admin/commerce/reconcile":
                permission = "ReconcilePayments"
            if group == "identity/src/staff_confirmation_http.rs":
                composition = files["bin/oteryn-portal/src/main.rs"]
                permission = section(composition, r'configured_guard\(\s*staff_confirmation,.*?Permission::(\w+)')
            if group == "admin/src/web.rs":
                predicate = r'path\(\) == "/admin/roles" \{\s*Permission::(\w+)'
                if path != "/admin/roles":
                    predicate += r'\s*\} else \{\s*Permission::(\w+)'
                    permission = re.search(predicate, files[file])[2]
                else:
                    permission = section(files[file], predicate)
            if not permission:
                boundary = []
                pending_guard = False
                for token in re.finditer(
                    r'\b(?:configured_guard(?:_from)?|guarded)\(|Permission::(\w+)', files[file],
                ):
                    if token[1] is None:
                        pending_guard = True
                    elif pending_guard:
                        boundary.append(token[1])
                        pending_guard = False
                if len(set(boundary)) == 1:
                    permission = boundary[0]
            required = list(dict.fromkeys(["Access", permission])) if permission else []
        if not required:
            return "public", {"kind": "unresolved"}
        result = {"kind": "staff", "all": [permissions[p] for p in required]}
        if alternatives:
            result["any"] = [permissions[p] for p in alternatives]
        if group == "admin/src/commerce/http.rs" and path != "/admin/commerce/reconcile":
            policy = section(files[file], r"async fn authorize_panel.*?for permission in \[(.*?)\]")
            result["any"] = [permissions[p] for p in re.findall(r"Permission::(\w+)", policy)]
            if method == "post":
                commands = files["crates/admin/src/commerce/mod.rs"]
                clauses = section(commands, r'pub fn permission\(&self\).*?match self \{(.*?)\n        \}')
                arms = re.split(r"=> rbac::Permission::(\w+),", clauses)
                result["actions"] = {
                    variant.lower(): permissions[required]
                    for matches, required in zip(arms[::2], arms[1::2])
                    for variant in re.findall(r"Self::(\w+)", matches)
                }
        if group == "admin/src/console/access.rs" and method == "get":
            area = path.rsplit("/", 1)[1]
            if area in ("audit", "players"):
                handler = files[f"crates/admin/src/console/{area}.rs"]
                if area == "audit":
                    expression = section(handler, r"pub\(crate\) async fn page(.*?)if !platform && !game")
                else:
                    expression = section(handler, r"let can_search = (.*?)let lang =")
                result["any"] = re.findall(r'\.check\(actor, "([^"]+)"\)', expression)
        if group == "admin/src/console/access.rs" and path.endswith("/players") and method == "post":
            handler = files["crates/admin/src/console/players.rs"]
            result["all"].append(section(handler, r'async fn search.*?\.check\(actor, "([^"]+)"\)'))
        if group == "wiki/src/web.rs":
            repo = files["crates/wiki/src/repository.rs"]
            article = "/admin/wiki/articles"
            editor_paths = {
                article + "/create", article + "/{id}", article + "/{id}/edit",
                article + "/{id}/preview/{locale}", article + "/{id}/revisions",
            }
            manage = section(repo, r'pub async fn create\(.*?\.authorize\(context, Permission::(\w+)')
            publish = section(repo, r'pub async fn restore\(.*?\.authorize\(context, Permission::(\w+)')
            if path in editor_paths or (path == article and method == "post"):
                result["all"].append(permissions[manage])
            if path == article + "/{id}/revisions/{source}/restore":
                result["all"].append(permissions[publish])
            if path == article + "/{id}/{action}":
                transition = section(repo, r'pub async fn transition(.*?)let mut tx')
                choices = re.findall(r'Permission::(\w+)', transition)
                result["actions"] = {
                    "publisher": permissions[choices[0]], "editor": permissions[choices[1]],
                }
        if group == "content/src/events/web.rs" and path == "/admin/events/{id}/status":
            repository = files["crates/content/src/events/repository.rs"]
            transition = section(repository, r"pub async fn transition.*?&\[(.*?)\]")
            result["all"] = list(dict.fromkeys([
                *result["all"], *(permissions[p] for p in re.findall(r"Permission::(\w+)", transition)),
            ]))
        if group == "admin/src/players/http.rs" and path == "/admin/players":
            required = section(files[file], r'async fn search.*?s.check\(context, Permission::(\w+)')
            result["all"].append(permissions[required])
        if group == "admin/src/players/http.rs" and path == "/admin/players/{id}":
            players = files["crates/admin/src/players/mod.rs"]
            for_read = ("account", "kick_view")
            def player_permission(handler):
                visibility = "" if handler.endswith("_impl") else "pub "
                kind = section(players, rf"{visibility}async fn {handler}\(.*?self.check\(context, Permission::(\w+)")
                return permissions[kind]
            if method == "get":
                result["any"] = [player_permission(handler) for handler in for_read]
            else:
                commands = section(files[file], r"match form\.action\.as_str\(\) \{(.*?)\n\s*_ =>")
                arms = re.split(r'"([a-z-]+)"\s*=>', commands)
                actions = {}
                for action, arm in zip(arms[1::2], arms[2::2]):
                    handlers_in_arm = re.findall(r'\bs\s*\.\s*(\w+)\s*\(', arm)
                    if len(handlers_in_arm) != 1:
                        raise ValueError(f"unresolved player command {action}")
                    handler, = handlers_in_arm
                    if handler == "attest_reset_evidence_for":
                        handler = section(players, r'pub async fn attest_reset_evidence_for\(.*?self\.(\w+)\(')
                    actions[action] = player_permission(handler)
                result["actions"] = actions
                approval = section(players, r'pub async fn approve_reset\(.*?&self.auth,\s*context,\s*&\[(.*?)\]')
                result["actionAll"] = {"approve-reset": [
                    permissions[p] for p in re.findall(r"Permission::(\w+)", approval)
                ]}
        if group == "world/src/admin.rs":
            required = section(files[file], r'if let Err\(status\) = authorize\(&s, context, Permission::(\w+)')
            result["all"].append(permissions[required])
            if method == "post":
                actions = section(files[file], r'let permission = match form.action.as_str\(\) \{(.*?)_ =>')
                result["actions"] = {
                    action: permissions[name]
                    for action, name in re.findall(r'"([^"]+)" => Permission::(\w+)', actions)
                }
        if group == "admin/src/console/mod.rs" and path != "/console":
            feature = path.rsplit("/", 1)[1]
            module = str(Path(file).with_name(f"{feature}.rs"))
            result["all"].append(section(files[module], r'const PERMISSION: &str = "([^"]+)"'))
        return "public", result

    if method == "get" and group == "identity/src/passkeys_http.rs" and not path.endswith("/manage"):
        return public
    if group == "stats/src/watch/http.rs":
        return "public", {"kind": "bearer"}
    if group == "atlas/src/bestiary/web.rs":
        return "public", {"kind": "logged-in"}
    if group == "bazaar/src/game/runtime/browser.rs" and path == "/bazaar/items/new":
        return "public", {"kind": "logged-in"}
    if group == "playerdata/src/hunts_web.rs" and path.endswith("/share"):
        return public
    if group == "store/src/web.rs" and path == "/shop" and method == "post":
        # The configured browser shop keeps purchases off until it has owner authority.
        return public
    if path.startswith(("/account/", "/panel/", "/support/tickets", "/support/reports", "/support/appeals",
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
    if listener == "gateway" and url.startswith("/v1/"):
        source = "crates/gateway/src/wire.rs" if url.endswith("/login") else file
        marker = "if !visible(&r.game_login_ticket" if url.endswith("/login") else "if !s.protocol3 {"
        lines = files[source]
        if source == file:
            start = lines.index("async fn recovery_front")
        else:
            start = 0
        position = lines.index(marker, start)
        return dict(status=400, source=f"{source}:{lines.count(chr(10), 0, position) + 1}")
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
    elif url == "/api/v1/payments/provider-webhook":
        marker = "Delivery::Unauthenticated => StatusCode::UNAUTHORIZED"
        status = 401
    elif file == "crates/atlas/src/bestiary/web.rs":
        marker = "return StatusCode::UNAUTHORIZED.into_response()"
        status = 401
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
                for method, dispatch in handlers(expression).items():
                    listener, guard = guard_for(files, file, url, dispatch, permission, permissions)
                    profile = None
                    if file == "crates/payments/src/coin_http.rs" and url.startswith("/native-account/coin-checkout/"):
                        profile = "unavailable" if expression.startswith("any(") else "configured"
                        guard = {"kind": "anonymous" if profile == "unavailable" else "logged-in"}
                    if file == "crates/bazaar/src/items/web.rs":
                        unavailable = re.search(rf'\b{dispatch}\(unavailable\)', expression)
                        profile = "bazaar-unavailable" if unavailable else "bazaar-qualified-embedding"
                        logged_in = not unavailable and (dispatch == "post" or url.endswith("/new"))
                        guard = {"kind": "logged-in" if logged_in else "anonymous"}
                    if file == "crates/bazaar/src/game/runtime/browser.rs" and url.startswith("/{locale}/bazaar/items"):
                        profile = "bazaar-runtime"
                    row = rows.setdefault((listener, url, method), {
                        "listener": listener, "path": url, "method": method.upper(),
                        "registrations": []
                    })
                    item = {"source": f"{file}:{line}", "guard": guard}
                    if method != dispatch:
                        item["implicitFrom"] = dispatch.upper()
                    if profile:
                        item["profile"] = profile
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
        subprocess.run([sys.executable, str(OUTPUT.with_name("check_coverage.py"))], check=True)
    else:
        OUTPUT.write_text(generated)


if __name__ == "__main__":
    main()
