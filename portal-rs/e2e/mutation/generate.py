"""Regenerate the browser mutation inventory from Portal sources."""
import argparse
import json
from pathlib import Path
import re

PORTAL = Path(__file__).resolve().parents[2]
OUTPUT = Path(__file__).with_name("forms.json")
WRITE = re.compile(r"(?:(?<![\w:])|(?<=routing::))(post|put|patch|delete)\s*\(\s*([\w:]+|\|)")
ROUTE = re.compile(r"\.route\(\s*")
TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|[(),]')
TAG = re.compile(r"<(form|input|button|select|option|textarea)\b((?:\{\{.*?\}\}|\{%.*?%\}|\"[^\"]*\"|'[^']*'|[^>])*?)>", re.I | re.S)
ATTRIBUTE = re.compile(
    r"""([\w:-]+)\s*=\s*(?:"((?:\{%.*?%\}|\{\{.*?\}\}|[^"])*)"|'([^']*)'|([^\s>]+))""",
    re.S,
)
FORM = re.compile(r"<form\b.*?</form\s*>", re.I | re.S)
ACTION_FIELDS = {"action", "decision", "kind", "op", "operation", "status"}
INLINE_METHOD = re.compile(r"MethodFilter::(POST|PUT|PATCH|DELETE)\b")
CONSTANT = re.compile(r'\bconst (\w+)\s*:\s*&str\s*=\s*"([^"]+)"')

# These are protocol receivers even when their paths look like regular HTTP routes.
NON_BROWSER = {
    "/oauth/token", "/__test/payments/webhook", "/api/v1/payments/provider-webhook",
    "/api/v1/native-account/coin-purchases/orders",
}
# These serve only as nested peer authentication probes.
PRIVATE_PROBES = {
    "crates/gameauth/src/runtime.rs", "crates/store/src/delivery_http.rs",
    "crates/store/src/purchase_http.rs", "crates/support/src/sanctions/enforcement.rs",
}
# Non-literal registrations require their enclosing loop/match to interpret.
ROUTE_BINDINGS = {
    ("crates/admin/src/console/access.rs", "path"): ["/admin/console/players"],
    ("crates/admin/src/support_web.rs", '&format!("{}/{{id}}", base(area))'): [
        "/admin/support/tickets/{id}", "/admin/moderation/reports/{id}",
        "/admin/moderation/enforcement/{id}",
    ],
    ("crates/payments/src/coin_http.rs", "&path"): ["/native-account/coin-checkout/{order_id}"],
}


def location(path, source, offset):
    return f"{path}:{source.count(chr(10), 0, offset) + 1}"


def attributes(tag):
    return {
        match[1].lower(): next(value for value in match.groups()[1:] if value is not None)
        for match in ATTRIBUTE.finditer(tag)
    }


def bindings(path, source, all_sources):
    """Read the finite discriminants supplied to shared form templates."""
    if path == "templates/account/lifecycle.html":
        router = all_sources["crates/identity/src/account_lifecycle_http.rs"]
        targets = re.findall(r'\.route\(\s*"(/\{lang\}/[^"]+)"', router)
        return {"{{ action }}": list(dict.fromkeys(targets))}
    if path in {"templates/content/announcements.html", "templates/admin/events/form.html"}:
        owner = {
            "templates/content/announcements.html": "crates/content/src/announcements_staff.rs",
            "templates/admin/events/form.html": "crates/content/src/events/web.rs",
        }[path]
        renderer = all_sources[owner]
        targets = re.findall(r'\baction: (?:format!\()?"(/admin/[^"]+)"', renderer)
        if path == "templates/admin/events/form.html":
            action = renderer.split("action: id.map_or_else(", 1)[1].split("),\n", 2)[:2]
            targets = re.findall(r'"(/admin/[^"]+)"', "\n".join(action))
        return {"{{ action }}": targets}
    if path == "templates/admin/wiki/action.html":
        edit = all_sources["templates/admin/wiki/edit.html"]
        return {"{{ action }}": re.findall(r'{% set action = "([^"]+)" %}', edit)}
    if path == "crates/admin/src/commerce/page.html":
        router = all_sources["crates/admin/src/commerce/http.rs"]
        commands = router.split("for (key, command) in [", 1)[1].split("] {", 1)[0]
        return {"{{action.key}}": re.findall(r'\("([^"]+)", Command::', commands)}
    if path == "crates/admin/src/bazaar.rs":
        panel = source.split("let forms = vec![", 1)[1].split("\n    ];", 1)[0]
        return {"{{ form.action }}": re.findall(r'\bform\(\s*"([^"]+)"', panel)}
    if path == "crates/support/src/tickets/page.html":
        domain = all_sources["crates/support/src/tickets/mod.rs"]
        statuses = domain.split("pub const STATUSES:", 1)[1].split("];", 1)[0]
        return {"{{ option.value }}": re.findall(r'"([^"]+)"', statuses)}
    if path == "crates/minigames/src/collection_pvp/http.rs":
        operations = source.split("let options = [", 1)[1].split("]", 1)[0]
        return {"{{ option.0 }}": re.findall(r'"([^"]+)"', operations)}
    if path == "crates/admin/src/support_web.rs":
        panel = source.split("let actions:", 1)[1].split("for (action, fields)", 1)[0]
        return {"{action}": re.findall(r'\(\s*"([^"]+)",\s*&\[', panel)}
    return {}


def forms(path, source, all_sources):
    # Normalize escaped HTML attribute delimiters in Rust string literals.
    source = source.replace(r'\"', '"')
    dynamic = bindings(path, source, all_sources)
    same_line = {}
    for form in FORM.finditer(source):
        tags = list(TAG.finditer(form[0]))
        if not tags:
            continue
        attrs = attributes(tags[0][2])
        if attrs.get("method", "").lower() != "post":
            continue
        target = attrs.get("action", "")
        fields = []
        submits = []
        for tag in tags[1:]:
            field = attributes(tag[2])
            name = field.get("name")
            if name:
                fields.append(name)
            if tag[1].lower() == "button" or field.get("type") in {"submit", "image"}:
                if field.get("type") in {"button", "reset"}:
                    continue
                submits.append({key: field[key] for key in ("name", "value", "formaction") if key in field})
            elif name in ACTION_FIELDS and field.get("type") in {"hidden", "radio"}:
                value = field.get("value", "")
                submits.extend({"name": name, "value": choice} for choice in dynamic.get(value, [value]))
            elif name in ACTION_FIELDS and tag[1].lower() == "select":
                options = form[0][tag.end():].split("</select", 1)[0]
                for option in TAG.finditer(options):
                    if option[1].lower() != "option":
                        continue
                    value = attributes(option[2]).get("value")
                    if value is not None:
                        submits.extend({"name": name, "value": choice} for choice in dynamic.get(value, [value]))
        action_variants = []
        if "{% if" in target:
            conditional = re.search(r"{% if .*? %}(.*?)(?:{% else %}(.*?))?{% endif %}", target)
            if conditional:
                action_variants = [
                    target[:conditional.start()] + (branch or "") + target[conditional.end():]
                    for branch in conditional.groups()
                ]
        if "{{ action }}" in target:
            # Literal action loops in Askama templates emit one form per action.
            loop = re.search(r'{% for action in (\[[^\]]+\]) %}(?:(?!{% endfor %}).)*$', source[:form.start()], re.S)
            if loop:
                action_variants = [target.replace("{{ action }}", action) for action in json.loads(loop[1])]
        for placeholder, choices in dynamic.items():
            if placeholder in target:
                action_variants = [target.replace(placeholder, choice) for choice in choices]
        location_key = location(path, source, form.start())
        same_line[location_key] = same_line.get(location_key, 0) + 1
        yield dict(source=location_key, ordinal=same_line[location_key], action=target,
                   actionVariants=action_variants, fields=list(dict.fromkeys(fields)), submits=submits)


def source_files():
    yield from sorted((PORTAL / "templates").rglob("*.html"))
    for root in ("crates", "bin"):
        for file in sorted((PORTAL / root).rglob("*")):
            if file.suffix == ".html":
                yield file
            if (file.suffix == ".rs" and "tests" not in file.parts
                    and not re.search(r"(?:^|_)tests?\.rs$|_test_db\.rs$", file.name)):
                yield file


def canonical(path):
    return re.sub(r"^/(?:pl|en|\{locale\})/", "/{lang}/", path)


def route_paths(path, expression, constants, source):
    if (path, expression) in ROUTE_BINDINGS:
        return ROUTE_BINDINGS[path, expression]
    literal = re.fullmatch(r'"([^"]+)"', expression)
    formatted = re.fullmatch(r'&?format!\("([^"]+)"(?:,\s*[^)]+)?\)', expression)
    if literal:
        return [literal[1]]
    if formatted:
        value = formatted[1].replace("{{", "\x00").replace("}}", "\x01")
        value = re.sub(r"\{(\w+)\}", lambda m: constants.get(m[1], m[0]), value)
        value = value.replace("\x00", "{").replace("\x01", "}")
        if "{prefix}" in value:
            paths = [value.replace("{prefix}", "/{lang}")]
            if 'for prefix in ["",' in source:
                paths.append(value.replace("{prefix}", ""))
            return paths
        return [value]
    if expression in constants:
        return [constants[expression]]
    return []


def registrations(source):
    for route in ROUTE.finditer(source):
        depth = 0
        separator = None
        for token in TOKEN.finditer(source, route.end()):
            if token[0] == "(":
                depth += 1
            elif token[0] == ")" and depth:
                depth -= 1
            elif token[0] == ")":
                if separator is not None:
                    yield route.start(), source[route.end():separator].strip(), source[separator + 1:token.start()]
                break
            elif token[0] == "," and depth == 0 and separator is None:
                separator = token.start()


def inventory():
    sources = {file.relative_to(PORTAL).as_posix(): file.read_text() for file in source_files()}
    routes = {}
    form_rows = []
    for path, source in sources.items():
        form_rows.extend(forms(path, source, sources))
        if not path.endswith(".rs"):
            continue
        source = source.split("#[cfg(test)]\nmod tests {", 1)[0]
        if path.startswith(("crates/gateway/", "crates/internal-api/")):
            continue
        constants = dict(CONSTANT.findall(source))
        for offset, expression, tail in registrations(source):
            if path == "crates/web/src/lib.rs":
                match = re.match(r"(badge_routes|profile_routes)\b", tail.strip())
                if match:
                    tail = source.split(f"let {match[1]} = ", 1)[1].split("\n    );", 1)[0]
                    # profile_routes has a separate .on() after the initial get().
                    if match[1] == "profile_routes":
                        tail = source.split("let profile_routes = ", 1)[1].split("let badge_routes = ", 1)[0]
            methods = WRITE.findall(tail)
            methods.extend((method, "|") for method in INLINE_METHOD.findall(tail))
            if not methods:
                continue
            for endpoint in route_paths(path, expression.strip(), constants, source):
                if (not endpoint.startswith("/") or endpoint.startswith("/internal/")
                        or endpoint in NON_BROWSER or endpoint.startswith("/api/v1/game-auth/")
                        or path == "crates/admin/src/control/http.rs"
                        or (endpoint == "/" and path in PRIVATE_PROBES)):
                    continue
                for method, handler in methods:
                    key = method.upper(), canonical(endpoint)
                    route_row = routes.setdefault(key, dict(method=key[0], route=key[1], sources=[]))
                    registration = dict(source=location(path, source, offset), handler=handler if handler != "|" else "inline closure")
                    if registration not in route_row["sources"]:
                        route_row["sources"].append(registration)
    return dict(forms=form_rows, routes=[routes[key] for key in sorted(routes)])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if forms.json differs from source")
    args = parser.parse_args()
    generated = json.dumps(inventory(), indent=2, ensure_ascii=False) + "\n"
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text() != generated:
            parser.exit(1, "forms.json is stale; rerun mutation/generate.py\n")
    else:
        OUTPUT.write_text(generated)


if __name__ == "__main__":
    main()
