#!/usr/bin/env python3
"""Generate the source-level portal feature audit using the checked-in route inventory."""
from collections import Counter, defaultdict
from html import unescape
from html.parser import HTMLParser
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
E2E = ROOT / "e2e"
ACTORS = ("anonymous", "player")
CATALOG = "crates/admin/src/rbac/catalog.rs"
ASKAMA = re.compile(r"\{\{.*?\}\}|\{%.*?%\}|\{#.*?#\}", re.S)
RUST_STRING = r'"(?:[^"\\\n]|\\.)*"'
FUNCTION = re.compile(r"(?m)^\s*(?:pub(?:\([^\n]*?\))?\s+)?(?:async\s+)?fn (\w+)\s*\(")
TEST = re.compile(r"(?m)^\s*test\(\s*(['\"])(.*?)\1\s*,")
ROUTES = json.loads((E2E / "inventory.json").read_text())["routes"]
EXPECTATIONS = {
    (row["crate"], row["route"], row["method"]): row
    for row in json.loads((E2E / "route-expectations.json").read_text())
}
FEATURES = []
SPECS = {}
for spec in sorted((E2E / "specs").glob("*.spec.mjs")):
    body = spec.read_text()
    declarations = list(TEST.finditer(body))
    for index, test in enumerate(declarations):
        end = declarations[index + 1].start() if index + 1 < len(declarations) else len(body)
        SPECS[spec.name, test[2]] = body[test.end():end]


def add(kind, feature, source, crate, actors, evidence=(), **details):
    for actor in actors:
        hits = sorted({item["test"] for item in evidence if item["actor"] == actor})
        FEATURES.append(dict(kind=kind, feature=feature, source=source, crate=crate,
                             actor=actor, tests=hits,
                             status="COVERED" if hits else "UNCOVERED", **details))


def hit(actor, spec, test, proof):
    return dict(actor=actor, test=f"{spec}:{test} ({proof})")


def reviewed(actor, spec, title_fragment, proof):
    for (file, title), body in SPECS.items():
        if file == spec and title_fragment in title and proof in body:
            return [hit(actor, file, title, proof)]
    return []


def source_at(file, text, offset):
    return f"{file}:{text.count(chr(10), 0, offset) + 1}"


def route_pattern(path):
    chunks = re.split(r"(\{[^}]+\})", path)
    return re.compile("".join(
        r"(?:\{lang\}|pl|en)" if chunk == "{lang}" else
        r".+" if chunk.startswith("{*") else r"[^/]+" if chunk.startswith("{") else
        re.escape(chunk) for chunk in chunks) + "$")


def endpoint(value):
    value = re.sub(r"\{\{\s*(?:chrome\.)?(?:lang|locale|language)\s*\}\}", "{lang}", value)
    value = re.sub(r"\{\{.*?\}\}", "{parameter}", value)
    return value.split("?", 1)[0].split("#", 1)[0].rstrip("/") or "/"


def actors_for(path, crate):
    if re.search(r"(?:^|/)(?:admin|console)(?:/|$)", path):
        return staff_actors(path)
    if (re.search(r"/(?:account|panel)(?:/|$)|/support/(?:tickets|reports)(?:/|$)", path)
            or crate in {"playerdata", "player-api"}):
        return ("player",)
    if re.search(r"/(?:logout|mfa/(?:enroll|confirm|devices))(?:/|$)", path):
        return ("player",)
    if path.startswith(("/api/", "/__test/")):
        return ("API client",)
    return ACTORS


def rust_sources():
    for directory in ("crates", "bin"):
        for path in sorted((ROOT / directory).rglob("*.rs")):
            file = path.relative_to(ROOT).as_posix()
            if "tests" in path.parts or re.search(r"(?:^|/|_)tests?\.rs$|_test_db\.rs$", file):
                continue
            text = path.read_text()
            test_module = re.search(r"(?m)^#\[cfg\(test\)\]\s*mod tests\s*\{", text)
            yield file, text[:test_module.start()] if test_module else text


RUST = dict(rust_sources())
permission_src = RUST[CATALOG]
perm_section, role_section = permission_src.split("pub enum BuiltInRole", 1)
PERMISSIONS = {
    match[1]: (match[2], source_at(CATALOG, permission_src, match.start()))
    for match in re.finditer(r'Self::(\w+) => "([^"]+)"', perm_section)
}
ROLE_KEYS = dict(re.findall(r'Self::(\w+) => "([^"]+)"', role_section))
ROLES = defaultdict(list)
for role, block in re.findall(r"Self::(\w+) => &\[(.*?)\]", role_section, re.S):
    for permission in re.findall(r"Permission::(\w+)", block):
        ROLES[permission].append(ROLE_KEYS[role])

# Match actual helper calls for role actions, whose selectors live outside specs.
ROLE_PROOFS = {
    value: reviewed("platform_admin", "staff-rbac-roles.spec.mjs", "role grant enables",
                    f"membership(s.page,account.id,id,'{value}')")
    for value in ("grant", "revoke")
}

STAFF_SURFACES = [
    ("/admin/support/tickets", "ManageSupportTickets"),
    ("/admin/moderation/reports", "ManageSupportReports"),
    ("/admin/moderation/enforcement", "ManageSupportEnforcement"),
    ("/admin/news", "ManageNews"),
    ("/admin/pages", "ManagePages"),
    ("/admin/media", "ManageMedia"),
    ("/admin/events", "ManageEvents"),
    ("/admin/wiki", "WikiAccess"),
    ("/admin/roles", "ManageRoles"),
    ("/admin/console/audit", "ViewAudit"),
    ("/admin/analytics", "AdminAnalyticsView"),
]


def staff_actors(path):
    for prefix, permission in STAFF_SURFACES:
        if path == prefix or path.startswith(prefix + "/"):
            return tuple(ROLES[permission])
    return ("platform_admin",)


def route_source(row):
    for source in row["sources"]:
        if source.rsplit(":", 1)[-1].isdigit():
            return source
    crate, path = row["crate"], row["route"]
    source = row["sources"][0].split(":", 1)[0]
    if source.endswith(".rs") and source in RUST:
        candidates = [(source, RUST[source])]
    elif crate == "content":
        candidates = [("crates/content/src/pages.rs", RUST["crates/content/src/pages.rs"])]
    elif crate == "admin":
        candidates = [("crates/admin/src/support_web.rs", RUST["crates/admin/src/support_web.rs"])]
    elif crate == "gameauth":
        candidates = [("crates/gameauth/src/devices_http.rs", RUST["crates/gameauth/src/devices_http.rs"])]
    else:
        return row["sources"][0]
    for file, text in candidates:
        if crate == "gameauth":
            needle = 'const BASE:'
        elif source.endswith("/analytics/web.rs"):
            needle = '.route(&format!("{path}/data")'
        else:
            suffix = path.removeprefix("/{lang}/").removesuffix("/{id}")
            escaped = suffix.replace("{", "{{").replace("}", "}}")
            found = re.search(r'"[^"\n]*(?:' + re.escape(escaped) + "|" + re.escape(suffix) + r')"', text)
            if found:
                return source_at(file, text, found.start())
            needle = "fn router"
        offset = text.find(needle)
        if offset >= 0:
            return source_at(file, text, offset)
    return row["sources"][0]


for row in ROUTES:
    path, crate = row["route"], row["crate"]
    for method in row["methods"]:
        actors = actors_for(path, crate)
        evidence = []
        item = EXPECTATIONS.get((crate, path, method))
        # The generated route test asserts an HTTP response, including refusals.
        # For writes, only credit a non-refused HTTP operation.
        if item and (method == "GET" or item["status"] < 300):
            actor = "platform_admin" if item["actor"] == "staff" else item["actor"]
            if actor in actors:
                evidence.append(hit(actor, "routes.spec.mjs", f"route {crate} {path}",
                                    f"asserts HTTP {item['status']}"))
        add("route", f"{method} {path}", route_source(row), crate, actors, evidence,
            route=path, method=method, sources=row["sources"])


TEMPLATE_CRATES = defaultdict(set)
for file, text in RUST.items():
    crate = file.split("/")[1]
    for match in re.finditer(r'#\[template\(\s*path\s*=\s*"([^"]+)"([^\]]*)\]', text):
        path = match[1]
        TEMPLATE_CRATES[path].add(crate)
        config = re.search(r'config\s*=\s*"../../templates/([^"]+)/askama.toml"', match[2])
        if config:
            TEMPLATE_CRATES[config[1] + "/" + path].add(crate)


def template_crate(file, action=""):
    parts = file.split("/")
    if parts[0] == "crates":
        return parts[1]
    if parts[0] == "static":
        return "ui"
    template = "/".join(parts[1:])
    owners = TEMPLATE_CRATES[template]
    if not owners and Path(template).suffix == ".ftl":
        prefix = str(Path(template).parent) + "/"
        owners = set().union(*(owners for name, owners in TEMPLATE_CRATES.items()
                               if name.startswith(prefix) and "/" not in name[len(prefix):]))
    if len(owners) == 1:
        return next(iter(owners))
    if action:
        matching = {row["crate"] for row in ROUTES if route_pattern(row["route"]).fullmatch(endpoint(action))}
        if len(matching) == 1:
            return next(iter(matching))
    return {"components": "ui", "layout.html": "ui", "pages": "web",
            "profile": "characters", "profiles": "community", "services": "characters",
            "bestiary": "atlas", "pass": "minigames", "collection": "minigames",
            "badges": "minigames", "places": "wiki", "deaths": "community",
            "guilds": "community", "houses": "community", "today": "community",
            "events": "content", "oauth": "gameauth", "mockups": "ui",
            "search": "web", "account": "web"}.get(parts[1], parts[1])


def template_actors(file, action=""):
    if "/admin/" in file or "/console/" in file:
        target = endpoint(action)
        if not target.startswith("/admin/") and file.startswith("templates/admin/"):
            target = "/admin/" + file.split("/")[2].split(".")[0]
        return staff_actors(target)
    if endpoint(action) == "/{lang}/logout":
        return ("player",)
    if "/account/mfa/" in file and endpoint(action).endswith(("/challenge", "/login", "/email")):
        return ("anonymous",)
    if any(f"/account/{name}/" in file for name in ("login", "reset")):
        return ACTORS
    if "/account/" in file and file.split("/")[-1].split(".")[0] not in {
        "register", "login", "verify", "reset", "forgot", "verification",
    }:
        return ("player",)
    return actors_for(endpoint(action), template_crate(file, action))


ACTION_PROOFS = {
    ("templates/account/register.html", "/{lang}/register", ""): (
        "anonymous", "identity.spec.mjs", "registration, mail verification", "await register(page,lang)"),
    ("templates/account/login/page.html", "/{lang}/login", ""): (
        "anonymous", "identity.spec.mjs", "registration, mail verification", "await login(page,lang,account)"),
    ("templates/account/mfa/page.html", "/{lang}/mfa/enroll", ""): (
        "player", "identity.spec.mjs", "TOTP enrollment", 'main form[action$="/mfa/enroll"]'),
    ("templates/account/mfa/page.html", "/{lang}/mfa/confirm", ""): (
        "player", "identity.spec.mjs", "TOTP enrollment", 'main form[action$="/mfa/confirm"]'),
    ("templates/account/mfa/page.html", "/{lang}/mfa/challenge", ""): (
        "anonymous", "identity.spec.mjs", "TOTP enrollment", 'main form[action$="/mfa/challenge"]'),
    ("templates/components/thais-header.html", "/{lang}/logout", ""): (
        "player", "identity.spec.mjs", "registration, mail verification", "await logout(page,lang)"),
    ("templates/admin/roles.html", "/admin/roles", "create"): (
        "platform_admin", "staff-rbac-roles.spec.mjs", "role grant enables", "await role(s.page,"),
}


def actions_in_specs(file, action, name, value, control):
    target = endpoint(action)
    proof = ACTION_PROOFS.get((file, target, value))
    if proof:
        yield from reviewed(*proof)
    if file == "templates/admin/roles.html" and target == "/admin/roles" and name == "action":
        yield from ROLE_PROOFS.get(value, [])
    if not action or not action.startswith("/"):
        return
    for (file, title), body in SPECS.items():
        actor = ("platform_admin" if file.startswith(("staff-", "admin", "content.")) else
                 "anonymous" if file.startswith("identity.") and "/logout" not in target else "player")
        # Require a submit helper with the literal full action selector. Named
        # buttons require independent evidence for the selected submit control.
        for match in re.finditer(r"\bsubmit\(\s*\w+(?:\.page)?\s*,\s*(['`])(.*?)\1", body, re.S):
            path = re.search(r'\[action="([^"]+)"\]', match[2])
            if (path and endpoint(path[1]) == target and not name
                    and (control is None or control.get("type", "submit") == "submit")
                    and re.search(r"\bexpect\(", body[match.end():])):
                yield hit(actor, file, title, match[2])


class Template(HTMLParser):
    """Tokenize HTML with Askama spans masked, retaining original attribute values."""

    def __init__(self, file, source):
        super().__init__(convert_charrefs=False)
        self.file, self.source = file, source
        self.forms, self.controls, self.hrefs, self.form_stack = [], [], [], []
        self.hidden_fields = defaultdict(dict)
        self.offsets = [0] + [m.end() for m in re.finditer("\n", source)]
        self.feed(ASKAMA.sub(lambda m: re.sub(r"[^\n]", "x", m[0]), source))
        self.close()

    def handle_starttag(self, tag, attrs):
        line, column = self.getpos()
        start = self.offsets[line - 1] + column
        token = self.get_starttag_text()
        values = {}
        for m in re.finditer(r"""([^\s=<>]+)\s*(?:=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+)))?""", token[1 + len(tag):-1]):
            name = m[1].lower()
            if name.startswith("xxx") and name not in {"x", "xx"}:
                continue
            value = next((i for i in (2, 3, 4) if m[i] is not None), None)
            offset = start + 1 + len(tag)
            values[name] = unescape(self.source[offset + m.start(value):offset + m.end(value)]) if value else ""
        location = f"{self.file}:{line}"
        if "href" in values:
            self.hrefs.append((location, tag, values["href"]))
        if tag == "form":
            form = (location, values, [])
            self.forms.append(form)
            self.form_stack.append(form)
        if tag == "input" and self.form_stack and values.get("type", "").lower() == "hidden":
            if values.get("name") in {"action", "op", "operation", "command", "kind", "status"}:
                self.hidden_fields[self.form_stack[-1][0]][values["name"]] = values.get("value", "")
        if tag in {"button", "input"} and values.get("type", "submit" if tag == "button" else "text").lower() in {"submit", "image"}:
            self.controls.append((location, values, self.form_stack[-1] if self.form_stack else None))

    handle_startendtag = handle_starttag

    def handle_endtag(self, tag):
        if tag == "form" and self.form_stack:
            self.form_stack.pop()


def template_features(file, text):
    template = Template(file, text)
    forms_by_id = {attrs["id"]: form for form in template.forms if "id" in (attrs := form[1])}
    for location, control, parent in template.controls:
        form = forms_by_id.get(control["form"]) if "form" in control else parent
        if form:
            form[2].append((location, control))

    for location, attrs, controls in template.forms:
        action = attrs.get("action") or "(current page)"
        method = attrs.get("method", "get").upper()
        crate = template_crate(file, action)
        actors = template_actors(file, action)
        hidden = template.hidden_fields[location]
        all_hits = {}
        for control_location, control in controls:
            destination = control.get("formaction") or action
            verb = control.get("formmethod", method).upper()
            name, value = control.get("name", ""), control.get("value", "")
            evidence = list(actions_in_specs(file, destination, name, value, control))
            for entry in evidence:
                all_hits[entry["actor"], entry["test"]] = entry
            if not name and len(controls) > 1:
                evidence = []
            add("submit", f"submit {verb} {destination} {name + '=' + value if name else '(unnamed)'}",
                control_location, crate, actors, evidence, action=destination, method=verb,
                name=name, value=value, form_source=location, hidden_dispatch=hidden)
        if not controls:
            for entry in actions_in_specs(file, action, "", "", None):
                all_hits[entry["actor"], entry["test"]] = entry
        add("form", f"form {method} {action}", location, crate, actors, all_hits.values(),
            action=action, method=method, submit_controls=len(controls), hidden_dispatch=hidden)

    loops = []
    forms_by_line = defaultdict(list)
    for location, attrs, _ in template.forms:
        forms_by_line[int(location.rsplit(":", 1)[1])].append((location, attrs))
    for directive in ASKAMA.finditer(text):
        loop = re.fullmatch(r'{%\s*for\s+(\w+)\s+in\s+\[(.*?)\]\s*%}', directive[0], re.S)
        if loop:
            loops.append((loop[1], re.findall(r'"([^"]+)"', loop[2]), directive.end()))
        elif re.match(r"{%\s*for\b", directive[0]):
            loops.append(None)
        elif re.match(r"{%\s*endfor\b", directive[0]) and loops:
            active = loops.pop()
            if not active:
                continue
            variable, values, start = active
            placeholder = re.compile(r"\{\{\s*" + re.escape(variable) + r"\s*\}\}")
            first_line = text.count("\n", 0, start) + 1
            last_line = text.count("\n", 0, directive.start()) + 1
            for line in range(first_line, last_line + 1):
                for location, attrs in forms_by_line[line]:
                    action = attrs.get("action", "")
                    if not placeholder.search(action):
                        continue
                    method = attrs.get("method", "get").upper()
                    for value in values:
                        destination = placeholder.sub(value, action)
                        add("form variant", f"{method} {destination}", location,
                            template_crate(file, action), template_actors(file, action),
                            action=destination, method=method, loop_variable=variable)

    for lookup in ASKAMA.finditer(text):
        for match in re.finditer(r'\b(?:text|t|tr)\(\s*"([\w.-]+)"', lookup[0]):
            add("translation reference", f"translation lookup {match[1]}",
                source_at(file, text, lookup.start() + match.start()), template_crate(file),
                template_actors(file), key=match[1], locales=["pl", "en"])

    for branch in re.finditer(
        r'{%\s*if\s+[^%\n]*?\b(?:lang|locale)\s*==\s*"(en|pl)"\s*%}'
        r'([^{}]+){%\s*else\s*%}([^{}]+){%\s*endif\s*%}', text
    ):
        for group, locale in ((2, branch[1]), (3, "en" if branch[1] == "pl" else "pl")):
            value = unescape(branch[group].strip())
            if value:
                add("localized literal", f"{locale} {value}", source_at(file, text, branch.start(group)),
                    template_crate(file), template_actors(file), locale=locale, value=value)

    for location, tag, href in template.hrefs:
        if re.match(r"(?i)^(?:[a-z][a-z0-9+.-]*:|//)", href.strip()):
            continue
        evidence = []
        if file == "templates/components/thais-header.html" and href.endswith("#profesje"):
            evidence = reviewed("anonymous", "public.spec.mjs", "Thais homepage",
                                "a[href=\"#profesje\"]').click()")
        if file == "templates/pages/home.html" and 'class="hero-news-all"' in text.splitlines()[int(location.rsplit(":", 1)[1]) - 1]:
            evidence = reviewed("anonymous", "public.spec.mjs", "Thais homepage",
                                "expect(news.locator('.hero-news-all')).toHaveAttribute")
        add("link" if tag in {"a", "area"} else "href", f"<{tag}> href={href}",
            location, template_crate(file), template_actors(file), evidence, href=href, tag=tag)


catalogues = defaultdict(dict)
for path in sorted(path for folder in ("templates", "crates", "static", "bin")
                   for path in (ROOT / folder).rglob("*")):
    parts = path.relative_to(ROOT).parts
    if parts[0] not in {"templates", "crates", "static", "bin"} or not path.is_file():
        continue
    file = "/".join(parts)
    if "templates" in parts and path.suffix in {".html", ".htm", ".jinja", ".j2"}:
        template_features(file, path.read_text())
    locale_file = re.search(r"(?:^|[_.-])(pl|en)$", path.stem)
    if locale_file and path.suffix in {".ftl", ".tsv"}:
        locale = locale_file[1]
        text = path.read_text()
        declaration = r"^(-?[\w-]+)\s*=" if path.suffix == ".ftl" else r"^([^#\s][^\t\n]*)\t"
        for match in re.finditer(declaration, text, re.M):
            following = text[match.end():].split("\n")
            lines = [following[0].strip()]
            for line in following[1:]:
                if not line.startswith((" ", "\t")):
                    break
                lines.append(line.strip())
            value = "\n".join(lines).strip()
            catalogue_name = str(path.parent.relative_to(ROOT) / (path.stem[:locale_file.start()] + path.suffix))
            catalogues[catalogue_name].setdefault(locale, {})[match[1]] = source_at(file, text, match.start())
            evidence = []
            if file.startswith("crates/ui/i18n/") and match[1] == "shell-news":
                evidence = reviewed("anonymous", "public.spec.mjs", "Thais homepage",
                                    "expect(news.getByRole('heading', {level: 1})).toHaveText")
            if file in {"crates/characters/src/create_web/en.ftl", "crates/characters/src/create_web/pl.ftl"} and match[1] == "pending":
                evidence = reviewed("player", "account.spec.mjs", "character request reaches pending", value)
            add("translation", f"{locale} {match[1]}", source_at(file, text, match.start()),
                template_crate(file), ("player",) if "/create_web/" in file else template_actors(file),
                evidence, locale=locale, key=match[1], value=value)

for catalogue in catalogues.values():
    if catalogue.keys() != {"pl", "en"}:
        continue
    for locale, other in (("pl", "en"), ("en", "pl")):
        for key in sorted(catalogue[other].keys() - catalogue[locale].keys()):
            source = catalogue[other][key]
            file = source.rsplit(":", 1)[0]
            add("translation gap", f"{locale} catalogue missing {key}", source,
                template_crate(file), template_actors(file), locale=locale,
                key=key, defined_in=other)

form_tests = defaultdict(set)
for row in FEATURES:
    if row["kind"] == "form" and row["tests"] and row["action"].startswith("/"):
        form_tests[row["actor"], row["method"], endpoint(row["action"])].update(row["tests"])
for row in FEATURES:
    if row["kind"] == "route":
        tests = form_tests.get((row["actor"], row["method"], row["route"]))
        if tests:
            row["tests"] = sorted(set(row["tests"]) | tests)
            row["status"] = "COVERED"


# Rust's strings and comments are masked for brace tracking. Offsets stay
# aligned to the original file, including multi-line raw HTML and SQL literals.
RUST_TOKENS = re.compile(r'//[^\n]*|/\*.*?\*/|r(?P<hash>\#*)"(?s:.*?)"(?P=hash)|' + RUST_STRING, re.S)
DISPATCH = re.compile(
    r"\bmatch\s+(?P<subject>(?:\([^{};]{1,140}\)|[^{};\n]{1,140}))\s*\{"
)
DISPATCH_FIELD = re.compile(
    r"\b(?:action|command|operation|decision|intent|op|form\.(?:kind|status|state|mode))\b"
)
CHECK = re.compile(r"\bPermission::(\w+)")
TRANSLATE = re.compile(
    r'\b(?:text|tr|translate|t)\(\s*(?:(?:[\w.:&]+\s*,\s*)?)"([\w.-]+)"'
)
KEY_ARM = re.compile(r'^\s*("(?:[^"\\]|\\.)*"|(?:\w+::)+\w+|_)(?:\s*\([^;]*?\))?')
TRANSLATION_CONTEXT = re.compile(r"\b(?:lang|locale|language|english|\ben\b)\b|Language::(?:En|Pl)", re.I)


def mask_rust(text):
    return RUST_TOKENS.sub(lambda match: re.sub(r"[^\n]", " ", match[0]), text)


def top_level_arrows(text, start, end=None):
    masked = mask_rust(text[start:end])
    nesting = []
    cursor = 0
    for token in re.finditer(r"=>|[{}()\[\],]", masked):
        symbol = token[0]
        if symbol in "{([":
            nesting.append(symbol)
        elif symbol in "})]":
            if not nesting:
                break
            nesting.pop()
            if not nesting and symbol == "}" and not masked[token.end():].lstrip().startswith(","):
                cursor = token.end()
        elif not nesting and symbol == ",":
            cursor = token.end()
        elif not nesting:
            yield start + cursor, text[start + cursor:start + token.start()].strip()


def source_actors(file, function_source):
    roles = sorted({role for variant in CHECK.findall(function_source) for role in ROLES[variant]})
    if roles:
        return tuple(roles)
    if "/admin/" in file or re.search(r"/(?:admin|staff)(?:\.rs|/)", file):
        return ("platform_admin",)
    if "Form<" in function_source or "BrowserSession" in function_source:
        return ("player",)
    return ACTORS


permission_checks = defaultdict(list)
for file, text in RUST.items():
    if file == CATALOG:
        continue
    crate = file.split("/")[1]
    masked = mask_rust(text)
    functions = list(FUNCTION.finditer(masked))
    for index, function in enumerate(functions):
        end = functions[index + 1].start() if index + 1 < len(functions) else len(text)
        start = function.start()
        body = text[start:end]
        name = function[1]
        actors = source_actors(file, body)
        for check in CHECK.finditer(masked, start, end):
            variant = check[1]
            if variant not in PERMISSIONS:
                continue
            # Navigation metadata and catalogues can supply a permission to a
            # generic checker. Retain the reference as well as direct checks.
            source = source_at(file, text, check.start())
            kind = "gate"
            evidence = []
            if file == "crates/admin/src/console/access.rs" and variant == "ViewAudit" and name == "router":
                evidence = reviewed("platform_admin", "staff-rbac-audit.spec.mjs",
                                    "audit console filters", "await audit(s.page,")
            if re.search(r"\b(?:ALL|navigation|sidebar|register|permission)\b", name):
                kind = "permission reference"
            permission_checks[variant].append(dict(function=name, source=source, kind=kind))
            add(kind, f"{PERMISSIONS[variant][0]} · {name}", source, crate,
                ROLES[variant] or ("staff (custom role)",), evidence,
                permission=PERMISSIONS[variant][0], function=name,
                catalog_source=PERMISSIONS[variant][1])

        for dispatch in DISPATCH.finditer(masked, start, end):
            subject = dispatch["subject"]
            if not DISPATCH_FIELD.search(subject):
                continue
            # Method-call result matches are service error handling. The
            # dispatch inventory describes action inputs and enum variants.
            if ".await" in subject or ("(" in subject and not re.search(r"\.as_(?:str|ref|deref)\(\)", subject)
                                       and not subject.lstrip().startswith("(")):
                continue
            for offset, arm in top_level_arrows(text, dispatch.end(), end):
                if not KEY_ARM.match(arm) and not re.match(r"^(?:[A-Z]\w*|\()", arm):
                    continue
                source = source_at(file, text, offset + len(text[offset:]) - len(text[offset:].lstrip()))
                value = re.fullmatch(r'"([^"]+)"(?:\s+if.*)?', arm, re.S)
                evidence = ROLE_PROOFS.get(value[1], []) if file == "crates/admin/src/web.rs" and value else []
                add("mutation", f"{name}: {subject.strip()} → {' '.join(arm.split())}",
                    source, crate, actors, evidence, handler=name, selector=subject.strip(), arm=arm)

        for condition in re.finditer(
            r'\bif\s+([^{};\n]{0,100}?(?:\.|\b)(?:action|command|op)\b[^{};\n]{0,55}?)(?:==|!=)\s*("(?:[^"\\]|\\.)*")',
            body,
        ):
            subject, value = condition.groups()
            add("mutation", f"{name}: {subject.strip()} → {value}",
                source_at(file, text, start + condition.start()), crate, actors,
                handler=name, selector=subject.strip(), arm=value)

        for ref in TRANSLATE.finditer(body):
            add("translation reference", f"translation lookup {ref[1]}",
                source_at(file, text, start + ref.start()), crate, actors, key=ref[1], locales=["pl", "en"])

    for branch in re.finditer(
        r'(?:\bif\b[^{};\n]{0,100})\{\s*(' + RUST_STRING + r')\s*\}\s*else\s*\{\s*(' + RUST_STRING + r')\s*\}',
        text,
    ):
        condition = branch[0].split("{", 1)[0]
        if not TRANSLATION_CONTEXT.search(condition):
            continue
        reverse = bool(re.search(r'"pl"|Language::Pl|!en\b', condition))
        for group, locale in enumerate(("pl", "en") if reverse else ("en", "pl"), 1):
            value = branch[group][1:-1]
            add("localized literal", f"{locale} {value}", source_at(file, text, branch.start(group)),
                crate, source_actors(file, ""), locale=locale, value=value)

for variant, (permission, source) in PERMISSIONS.items():
    add("permission", permission, source, "admin", ROLES[variant] or ("staff (custom role)",),
        permission=permission, references=permission_checks[variant])


def markdown(value):
    return str(value).replace("|", "&#124;").replace("\n", " ")


FEATURES.sort(key=lambda row: (row["crate"], row["actor"], row["kind"],
                               row["source"], row["feature"]))
by_crate = defaultdict(Counter)
for row in FEATURES:
    by_crate[row["crate"]][row["status"]] += 1

summary = {
    crate: dict(features=sum(counts.values()), covered=counts["COVERED"], uncovered=counts["UNCOVERED"])
    for crate, counts in sorted(by_crate.items())
}
(E2E / "features.json").write_text(json.dumps(dict(summary=summary, features=FEATURES),
                                             indent=2, ensure_ascii=False) + "\n")
output = [
    "# Portal E2E feature audit", "",
    "Regenerate with `python3 portal-rs/e2e/features.py`. Counts are feature–actor pairs.",
    "A row marked COVERED has identifiable interaction or assertion evidence in the existing specs.",
    "A route HTTP assertion covers only the route contract and can include an expected read refusal.",
    "An UNCOVERED row has no verified evidence in this static scan. Dynamic selectors and indirect helper calls may require manual review.",
    "Source scope includes the checked-in route inventory, Askama template forms and hrefs, Rust dispatch and permission references, and locale catalogues and explicit lookups. Template patterns preserve runtime expressions.",
    "Computed actions, indirect RBAC checks, and database-backed editorial translations need review with real fixtures. Actor attribution for public surfaces and dynamic references is inferred from their owning surface.",
    "Browser crawl results are independent of the feature coverage classifications.",
    "", "| Crate | Features | Covered | Uncovered |", "| --- | ---: | ---: | ---: |",
]
for crate, counts in summary.items():
    output.append(f"| {crate} | {counts['features']} | {counts['covered']} | {counts['uncovered']} |")
groups = defaultdict(list)
covered = []
for row in FEATURES:
    if row["status"] == "COVERED":
        covered.append(row)
    else:
        groups[row["crate"], row["actor"]].append(row)


def table(rows):
    output.extend(["", "| Feature | Source file:line | Actor | Covering spec:test | Status |",
                   "| --- | --- | --- | --- | --- |"])
    for row in rows:
        fields = [row["feature"], row["source"], row["actor"], "<br>".join(row["tests"]), row["status"]]
        output.append("| " + " | ".join(map(markdown, fields)) + " |")


output.extend(["", "## Uncovered features"])
for (crate, actor), rows in groups.items():
    output.extend(["", f"### {crate} / {actor}"])
    table(rows)
output.extend(["", "## Covered features"])
table(covered)
(E2E / "FEATURES.md").write_text("\n".join(output) + "\n")
