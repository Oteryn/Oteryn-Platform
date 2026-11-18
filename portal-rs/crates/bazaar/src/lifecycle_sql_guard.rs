use std::path::{Path, PathBuf};
use syn::{
    Attribute, Expr, ImplItem, Item, Lit, Meta, TraitItem,
    buffer::{Cursor, TokenBuffer},
    visit::{self, Visit},
};

// Inline PostgreSQL fixtures seed catalogue rows, model a completed CC1
// handoff, or build a worker backlog. Exempt only these statements in the
// cfg(test) portion of their source files.
const FIXTURE_WRITES: &[(&str, &str)] = &[
    (
        "game/runtime/browser.rs",
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at,created_at) VALUES($1,$2,'item','auction','coins',100,false,CASE WHEN $3=0 THEN 'active'::bazaar_state ELSE 'completed'::bazaar_state END,now()+interval '1 day',now()+make_interval(secs=>$3))",
    ),
    (
        "game/runtime/browser.rs",
        "UPDATE bazaar_listings SET state='active'",
    ),
    (
        "game/runtime/browser.rs",
        "UPDATE bazaar_listings SET state='awaiting_delivery'",
    ),
    (
        "game/runtime/browser.rs",
        "UPDATE bazaar_listings SET state='completed',kind='character'",
    ),
    (
        "game/runtime/worker.rs",
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'item','auction','coins',100,false,'awaiting_delivery',now())",
    ),
    (
        "game/runtime/worker.rs",
        "INSERT INTO bazaar_authority_gold_bids(request_id,listing_id,buyer,character_id,maximum,hold_operation_id) VALUES($1,$1,$1,$1,100,$1)",
    ),
    (
        "game/runtime/mod.rs",
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'item','auction','coins',100,false,now()+interval '1 day')",
    ),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Token<'a> {
    Word(&'a str),
    Symbol(char),
    Literal,
}

impl Token<'_> {
    fn word(self, value: &str) -> bool {
        matches!(self, Self::Word(text) if text.eq_ignore_ascii_case(value))
    }
}

fn quoted(source: &str, quote: u8, escapes: bool) -> (&str, &str) {
    let bytes = source.as_bytes();
    let mut end = 1;
    while end < bytes.len() {
        if escapes && bytes[end] == b'\\' {
            end += 2;
        } else if bytes[end] == quote {
            if bytes.get(end + 1) == Some(&quote) {
                end += 2;
            } else {
                return (&source[1..end], &source[end + 1..]);
            }
        } else {
            end += 1;
        }
    }
    (&source[1..], "")
}

fn block_comment(mut source: &str) -> &str {
    let mut depth = 1;
    while let Some(end) = source.find("*/") {
        let nested = source.find("/*").filter(|start| *start < end);
        let index = nested.unwrap_or(end);
        if nested.is_some() {
            depth += 1;
        } else {
            depth -= 1;
        }
        source = &source[index + 2..];
        if depth == 0 {
            return source;
        }
    }
    ""
}

fn dollar_string(source: &str) -> Option<(&str, &str)> {
    let (tag, body) = source.strip_prefix('$')?.split_once('$')?;
    if !tag.is_empty()
        && (!tag.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            || !tag.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
    {
        return None;
    }
    body.split_once(&source[..tag.len() + 2])
}

// PostgreSQL token boundaries keep write targets outside string literals and comments.
fn sql_tokens(mut source: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    while let Some(first) = source.chars().next() {
        if first.is_whitespace() {
            source = source.trim_start();
        } else if source.starts_with("--") {
            source = source.split_once('\n').map_or("", |(_, rest)| rest);
        } else if let Some(comment) = source.strip_prefix("/*") {
            source = block_comment(comment);
        } else if let Some((_, rest)) = dollar_string(source) {
            tokens.push(Token::Literal);
            source = rest;
        } else if source.starts_with("E'") || source.starts_with("e'") {
            let (_, rest) = quoted(&source[1..], b'\'', true);
            tokens.push(Token::Literal);
            source = rest;
        } else if first == '"' || first == '\'' {
            let (body, rest) = quoted(source, first as u8, false);
            tokens.push(if first == '"' {
                Token::Word(body)
            } else {
                Token::Literal
            });
            source = rest;
        } else if first.is_alphanumeric() || first == '_' {
            let end = source
                .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
                .unwrap_or(source.len());
            tokens.push(Token::Word(&source[..end]));
            source = &source[end..];
        } else {
            tokens.push(Token::Symbol(first));
            source = &source[first.len_utf8()..];
        }
    }
    tokens
}

fn target<'a, 'sql>(mut tokens: &'a [Token<'sql>]) -> Option<(&'sql str, &'a [Token<'sql>])> {
    if tokens.first()?.word("only") {
        tokens = &tokens[1..];
    }
    let parenthesized = tokens.first() == Some(&Token::Symbol('('));
    if parenthesized {
        tokens = &tokens[1..];
    }
    let (Token::Word(table), mut rest) = tokens.split_first()? else {
        return None;
    };
    let mut table = *table;
    while let [Token::Symbol('.'), Token::Word(name), remaining @ ..] = rest {
        table = name;
        rest = remaining;
    }
    if parenthesized {
        rest = rest.strip_prefix(&[Token::Symbol(')')])?;
    }
    Some((table, rest))
}

fn assignment_to(tokens: &[Token<'_>], column: &str) -> bool {
    let rest = tokens.strip_prefix(&[Token::Symbol('*')]).unwrap_or(tokens);
    let rest = match rest {
        [set, ..] if set.word("set") => rest,
        [as_token, Token::Word(_), rest @ ..] if as_token.word("as") => rest,
        [Token::Word(_), rest @ ..] => rest,
        _ => return false,
    };
    let [set, assignments @ ..] = rest else {
        return false;
    };
    if !set.word("set") {
        return false;
    }
    let mut depth = 0;
    let mut lhs = true;
    for token in assignments {
        if depth == 0
            && (matches!(token, Token::Symbol(')' | ';'))
                || ["where", "from", "returning"]
                    .iter()
                    .any(|word| token.word(word)))
        {
            break;
        }
        match token {
            Token::Symbol('(' | '[') => depth += 1,
            Token::Symbol(')' | ']') => depth -= 1,
            Token::Symbol('=') if depth == 0 => lhs = false,
            Token::Symbol(',') if depth == 0 => lhs = true,
            _ if lhs && token.word(column) => return true,
            _ => {}
        }
    }
    false
}

fn forbidden_state_write(sql: &str) -> bool {
    const INSERT_TARGETS: &[&str] = &[
        "bazaar_listings",
        "bazaar_game_outbox",
        "bazaar_game_projections",
        "bazaar_authority_dispatch",
        "bazaar_authority_receipts",
        "bazaar_game_quarantine",
        "bazaar_authority_gold_bids",
        "bazaar_authority_transfers",
        "bazaar_authority_transfer_retries",
        "bazaar_authority_transfer_retirements",
        "character_commands",
    ];
    let tokens = sql_tokens(sql);
    tokens.iter().enumerate().any(|(index, token)| {
        let rest = &tokens[index + 1..];
        if (token.word("insert") || token.word("merge"))
            && let [into, rest @ ..] = rest
            && into.word("into")
        {
            return target(rest).is_some_and(|(table, _)| {
                INSERT_TARGETS
                    .iter()
                    .any(|protected| table.eq_ignore_ascii_case(protected))
            });
        }
        if token.word("update")
            && let Some((table, rest)) = target(rest)
        {
            return ["bazaar_game_outbox", "bazaar_game_projections"]
                .iter()
                .any(|protected| table.eq_ignore_ascii_case(protected))
                || (table.eq_ignore_ascii_case("bazaar_listings") && assignment_to(rest, "state"))
                || (table.eq_ignore_ascii_case("bazaar_authority_gold_bids")
                    && assignment_to(rest, "phase"));
        }
        false
    })
}

fn normalize_sql(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Ident>()
                .is_ok_and(|cfg| cfg == "test")
    })
}

macro_rules! attributes {
    ($node:expr, $syntax:ident, $($variant:ident),+ $(,)?) => {
        match $node {
            $($syntax::$variant(node) => node.attrs.as_slice(),)+
            _ => &[],
        }
    };
}

struct SourceGuard<'a> {
    path: &'a Path,
    test_scope: bool,
    modules: Vec<PathBuf>,
    module_dir: PathBuf,
    physical_dir: PathBuf,
    offenders: &'a mut Vec<String>,
}

impl SourceGuard<'_> {
    fn scoped(&mut self, attrs: &[Attribute], visit: impl FnOnce(&mut Self)) {
        let enclosing = self.test_scope;
        self.test_scope |= test_only(attrs);
        visit(self);
        self.test_scope = enclosing;
    }

    fn sql(&mut self, literal: &syn::LitStr) {
        let sql = literal.value();
        if !forbidden_state_write(&sql) {
            return;
        }
        let normalized = normalize_sql(&sql);
        if self.test_scope
            && FIXTURE_WRITES
                .iter()
                .any(|(file, known)| self.path == Path::new(file) && normalized == *known)
        {
            return;
        }
        self.offenders
            .push(format!("{}: {normalized}", self.path.display()));
    }

    fn macro_tokens(&mut self, mut tokens: Cursor<'_>) {
        // Macro arguments are token trees. Exempt only when the Rust AST places
        // the entire macro in test scope.
        while let Some((_, next)) = tokens.token_tree() {
            if let Some((inside, _, _, _)) = tokens.any_group() {
                self.macro_tokens(inside);
            } else if let Some((literal, _)) = tokens.literal()
                && let Ok(sql) = syn::parse_str::<syn::LitStr>(&literal.to_string())
            {
                self.sql(&sql);
            }
            tokens = next;
        }
    }
}

impl<'ast> Visit<'ast> for SourceGuard<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        let attrs = attributes!(
            item,
            Item,
            Const,
            Enum,
            ExternCrate,
            Fn,
            ForeignMod,
            Impl,
            Macro,
            Mod,
            Static,
            Struct,
            Trait,
            TraitAlias,
            Type,
            Union,
            Use
        );
        self.scoped(attrs, |guard| visit::visit_item(guard, item));
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        let attrs = attributes!(item, ImplItem, Const, Fn, Type, Macro);
        self.scoped(attrs, |guard| visit::visit_impl_item(guard, item));
    }

    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        let attrs = attributes!(item, TraitItem, Const, Fn, Type, Macro);
        self.scoped(attrs, |guard| visit::visit_trait_item(guard, item));
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        let attrs = attributes!(
            expr, Expr, Array, Assign, Async, Await, Binary, Block, Break, Call, Cast, Closure,
            Const, Continue, Field, ForLoop, Group, If, Index, Infer, Let, Lit, Loop, Macro, Match,
            MethodCall, Paren, Path, Range, RawAddr, Reference, Repeat, Return, Struct, Try,
            TryBlock, Tuple, Unary, Unsafe, While, Yield
        );
        self.scoped(attrs, |guard| visit::visit_expr(guard, expr));
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        self.scoped(&local.attrs, |guard| visit::visit_local(guard, local));
    }

    fn visit_stmt_macro(&mut self, mac: &'ast syn::StmtMacro) {
        self.scoped(&mac.attrs, |guard| visit::visit_stmt_macro(guard, mac));
    }

    fn visit_lit_str(&mut self, literal: &'ast syn::LitStr) {
        self.sql(literal);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let tokens = TokenBuffer::new2(mac.tokens.clone());
        self.macro_tokens(tokens.begin());
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let explicit = module.attrs.iter().find_map(|attr| {
            let Meta::NameValue(value) = &attr.meta else {
                return None;
            };
            let Expr::Lit(expr) = &value.value else {
                return None;
            };
            let Lit::Str(path) = &expr.lit else {
                return None;
            };
            value.path.is_ident("path").then(|| path.value())
        });
        if module.content.is_some() {
            let directory = explicit.map_or_else(
                || self.module_dir.join(module.ident.to_string()),
                |path| self.physical_dir.join(path),
            );
            let old_module = std::mem::replace(&mut self.module_dir, directory.clone());
            let old_physical = std::mem::replace(&mut self.physical_dir, directory);
            visit::visit_item_mod(self, module);
            self.module_dir = old_module;
            self.physical_dir = old_physical;
        } else if !self.test_scope {
            self.modules.push(match explicit {
                Some(file) => self.physical_dir.join(file),
                None => self.module_dir.join(module.ident.to_string()),
            });
        }
    }
}

fn inspect(
    root: &Path,
    path: &Path,
    file: &syn::File,
    offenders: &mut Vec<String>,
) -> Vec<PathBuf> {
    if path == Path::new("lifecycle.rs") {
        return Vec::new();
    }
    let parent = path.parent().unwrap_or(Path::new(""));
    let directory = if matches!(
        path.file_name().and_then(std::ffi::OsStr::to_str),
        Some("lib.rs" | "mod.rs")
    ) {
        parent.to_path_buf()
    } else {
        path.with_extension("")
    };
    let mut guard = SourceGuard {
        path,
        test_scope: test_only(&file.attrs),
        modules: Vec::new(),
        module_dir: directory,
        physical_dir: parent.to_path_buf(),
        offenders,
    };
    guard.visit_file(file);
    guard
        .modules
        .into_iter()
        .map(|path| {
            if path.extension().is_some() {
                path
            } else {
                let flat = path.with_extension("rs");
                if root.join(&flat).is_file() {
                    flat
                } else {
                    path.join("mod.rs")
                }
            }
        })
        .collect()
}

pub(super) fn scan_source(path: &Path, source: &str, offenders: &mut Vec<String>) {
    let parsed =
        syn::parse_file(source).or_else(|_| syn::parse_file(&format!("fn check() {{ {source} }}")));
    assert!(
        parsed.is_ok(),
        "invalid source fixture: {:?}",
        parsed.as_ref().err()
    );
    if let Ok(file) = parsed {
        inspect(Path::new(""), path, &file, offenders);
    }
}

pub(super) fn scan(root: &Path) -> std::io::Result<Vec<String>> {
    let mut pending = vec![PathBuf::from("lib.rs")];
    let mut offenders = Vec::new();
    while let Some(path) = pending.pop() {
        let source = std::fs::read_to_string(root.join(&path))?;
        let file = syn::parse_file(&source)
            .map_err(|error| std::io::Error::other(format!("{}: {error}", path.display())))?;
        pending.extend(inspect(root, &path, &file, &mut offenders));
    }
    Ok(offenders)
}
