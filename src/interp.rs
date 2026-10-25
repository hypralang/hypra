//! Runs an IR [`Program`] with JSON data, producing HTML, Markdown, JSON and WebMCP outputs.

use crate::diag::{self, Diagnostic, suggest};
use crate::ir::{ActionDef, At, Channel, Expr, Op, Program, apply, display, md_span, md_text};
use crate::lexer::Pos;
use crate::rules;
use serde_json::{Map, Value as Json, json};

#[derive(Debug, Clone, Default)]
pub struct Output {
    pub html: String,
    pub markdown: String,
    pub json: Json,
    pub webmcp: Json,
    pub diagnostics: Vec<Diagnostic>,
}

/// An action with its values filled in.
struct Action {
    name: String,
    description: String,
    method: String,
    endpoint: String,
    params: Map<String, Json>,
    required: Vec<String>,
}

pub fn run(program: &Program, data: &Json) -> Output {
    let mut it = Interp {
        data,
        vars: vec![Json::Null; program.vars],
        html: String::new(),
        md: String::new(),
        actions: vec![],
        diags: vec![],
    };
    for prop in &program.props {
        if data.get(&prop.name).is_none() {
            it.diags.push(
                Diagnostic::error(
                    "TYPE-001",
                    format!("prop `{}` is missing from the data", prop.name),
                    prop.pos,
                    prop.name.len(),
                )
                .help("pass it with --data file.json"),
            );
        }
    }
    it.ops(&program.ops);

    let meta = &program.meta;
    let title = display(&it.eval(&meta.title));
    let full_title = display(&it.eval(&meta.full_title));
    let description = meta.description.as_ref().map(|d| display(&it.eval(d)));
    let canonical = meta.canonical.as_ref().map(|c| display(&it.eval(c)));
    let lang = display(&it.eval(&meta.lang));
    // fixed text was checked at compile time; text from data is checked now
    let mut check = |expr: Option<&Expr>, at: Option<At>, text: Option<&String>, rule: rules::Rule| {
        let from_data = expr.is_some_and(|e| !matches!(e, Expr::Lit(_)));
        if let (true, Some(at), Some(text)) = (from_data, at, text)
            && let Some(d) = rule(text, at)
            && !meta.allowed.contains(&d.code)
        {
            it.diags.push(d);
        }
    };
    check(Some(&meta.full_title), meta.title_at, Some(&full_title), rules::title);
    check(Some(&meta.lang), meta.lang_at, Some(&lang), rules::lang);
    check(meta.description.as_ref(), meta.description_at, description.as_ref(), rules::description);
    check(meta.canonical.as_ref(), meta.canonical_at, canonical.as_ref(), rules::canonical);

    let mut markdown = it.md.trim_end().to_string();
    markdown.push('\n');
    if !it.actions.is_empty() {
        markdown.push_str("\n## Actions\n\n");
        for a in &it.actions {
            markdown.push_str(&format!(
                "- {}: {} → {} {}\n",
                md_span(&format!("{}({})", a.name, signature(a))),
                md_text(&a.description),
                a.method,
                md_text(&a.endpoint)
            ));
        }
    }

    let actions: Vec<Json> = it
        .actions
        .iter()
        .map(|a| {
            json!({
                "name": a.name,
                "description": a.description,
                "method": a.method,
                "endpoint": a.endpoint,
                "params": a.params,
                "required": a.required,
            })
        })
        .collect();
    let tools: Vec<Json> = it
        .actions
        .iter()
        .map(|a| {
            json!({
                "name": a.name,
                "description": a.description,
                "inputSchema": { "type": "object", "properties": a.params, "required": a.required },
            })
        })
        .collect();

    // loops repeat the same data error once per item; report each once
    let diagnostics = diag::unique(it.diags);

    Output {
        html: it.html,
        markdown,
        json: json!({
            "title": title,
            "description": description,
            "lang": lang,
            "canonical": canonical,
            "actions": actions,
        }),
        webmcp: Json::Array(tools),
        diagnostics,
    }
}

struct Interp<'a> {
    data: &'a Json,
    vars: Vec<Json>,
    html: String,
    md: String,
    actions: Vec<Action>,
    diags: Vec<Diagnostic>,
}

impl Interp<'_> {
    fn ops(&mut self, ops: &[Op]) {
        for op in ops {
            match op {
                Op::Html(text) => self.html.push_str(text),
                Op::Md(text) => self.md.push_str(text),
                Op::Value { to, expr, escape } => {
                    let text = apply(*escape, display(&self.eval(expr)));
                    match to {
                        Channel::Html => self.html.push_str(&text),
                        Channel::Md => self.md.push_str(&text),
                    }
                }
                Op::For { var, list, source, pos, len, body } => match self.eval(list) {
                    Json::Array(items) => {
                        for item in items {
                            self.vars[*var] = item;
                            self.ops(body);
                        }
                    }
                    // a missing value was already reported where it was looked up
                    Json::Null => {}
                    _ => self.diags.push(Diagnostic::error("TYPE-003", format!("`{source}` isn't a list"), *pos, *len)),
                },
                Op::If { cond, then, otherwise } => {
                    let branch = if truthy(&self.eval(cond)) { then } else { otherwise };
                    self.ops(branch);
                }
                Op::Set { var, value } => self.vars[*var] = self.eval(value),
                Op::Action(def) => {
                    let action = self.action(def);
                    add_action(&mut self.actions, action);
                }
                // static output has no server to add fields
                Op::FormFields { .. } => {}
                Op::AlternateLink { indent, stem } => self.html.push_str(&format!(
                    "{indent}<link rel=\"alternate\" type=\"text/markdown\" href=\"{}.md\">\n",
                    crate::ir::esc(stem)
                )),
            }
        }
    }

    fn action(&mut self, def: &ActionDef) -> Action {
        let mut params = Map::new();
        let mut required = vec![];
        for p in &def.params {
            let schema = match &p.value {
                Some(expr) => {
                    let value = self.eval(expr);
                    json!({ "type": json_type(&value), "const": value })
                }
                None => {
                    let mut schema = p.schema.clone();
                    let description = p.description.as_ref().map(|d| display(&self.eval(d))).unwrap_or_default();
                    if !description.is_empty() {
                        schema["description"] = json!(description);
                    }
                    schema
                }
            };
            params.insert(p.name.clone(), schema);
            if p.required {
                required.push(p.name.clone());
            }
        }
        Action {
            name: def.name.clone(),
            description: display(&self.eval(&def.description)),
            method: def.method.clone(),
            endpoint: display(&self.eval(&def.endpoint)),
            params,
            required,
        }
    }

    fn eval(&mut self, expr: &Expr) -> Json {
        let found = match expr {
            Expr::Lit(value) => return value.clone(),
            Expr::Concat(parts) => return Json::String(parts.iter().map(|p| display(&self.eval(p))).collect()),
            Expr::Equals(value, text) => return Json::Bool(display(&self.eval(value)) == *text),
            Expr::Data { path, pos, len } => match self.data.get(&path[0]) {
                Some(root) => walk(&path[0], root, &path[1..], *pos, *len),
                // reported once as a missing prop
                None => return Json::Null,
            },
            Expr::Var { id, name, path, pos, len } => walk(name, &self.vars[*id], path, *pos, *len),
        };
        // only the value the path ends at is copied, however large the data around it
        match found {
            Ok(value) => value.clone(),
            Err(d) => {
                self.diags.push(d);
                Json::Null
            }
        }
    }
}

/// Follows a path like `.name` from a value.
fn walk<'j>(name: &str, mut current: &'j Json, path: &[String], pos: Pos, len: usize) -> Result<&'j Json, Diagnostic> {
    let mut walked = name.to_string();
    for part in path {
        let Some(next) = current.get(part) else {
            let keys: Vec<&str> =
                current.as_object().map(|o| o.keys().map(String::as_str).collect()).unwrap_or_default();
            let mut d = Diagnostic::error("TYPE-002", format!("`{walked}` has no field `{part}`"), pos, len);
            if let Some(close) = suggest(part, keys) {
                d = d.help(format!("did you mean `{close}`?"));
            }
            return Err(d);
        };
        current = next;
        walked.push('.');
        walked.push_str(part);
    }
    Ok(current)
}

/// An action repeated in a loop is one tool; values that differ between the copies become the choices of
/// that parameter, e.g. the ids of the items a remove button is shown for.
fn add_action(actions: &mut Vec<Action>, action: Action) {
    let Some(existing) = actions.iter_mut().find(|a| a.name == action.name) else {
        actions.push(action);
        return;
    };
    for (key, schema) in action.params {
        let (Some(old), Some(value)) = (existing.params.get_mut(&key), schema.get("const")) else { continue };
        let mut values = match (old.get("const"), old.get("enum")) {
            (Some(same), _) if same == value => continue,
            (Some(first), _) => vec![first.clone()],
            (None, Some(Json::Array(choices))) => choices.clone(),
            _ => continue,
        };
        if !values.contains(value) {
            values.push(value.clone());
        }
        *old = json!({ "type": old["type"], "enum": values });
    }
}

/// Compact parameter list for the Markdown action index, e.g. `stars: integer 1..5, body?: string ..2000`.
fn signature(a: &Action) -> String {
    a.params
        .iter()
        .map(|(k, s)| {
            if let Some(v) = s.get("const") {
                return format!("{k}: {v}");
            }
            let mut t = s.get("format").or(s.get("type")).and_then(Json::as_str).unwrap_or("any").to_string();
            if let Some(options) = s.get("enum").and_then(Json::as_array) {
                t = options.iter().map(display).collect::<Vec<_>>().join("|");
            }
            let lo = s.get("minimum").or(s.get("minLength")).map(display);
            let hi = s.get("maximum").or(s.get("maxLength")).map(display);
            if lo.is_some() || hi.is_some() {
                t.push_str(&format!(" {}..{}", lo.unwrap_or_default(), hi.unwrap_or_default()));
            }
            let optional = if a.required.contains(k) { "" } else { "?" };
            format!("{k}{optional}: {t}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn json_type(j: &Json) -> &'static str {
    match j {
        Json::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Json::Number(_) => "number",
        Json::String(_) => "string",
        Json::Bool(_) => "boolean",
        Json::Array(_) => "array",
        Json::Object(_) => "object",
        Json::Null => "null",
    }
}

fn truthy(j: &Json) -> bool {
    match j {
        Json::Null => false,
        Json::Bool(b) => *b,
        Json::Array(a) => !a.is_empty(),
        Json::String(s) => !s.is_empty(),
        Json::Number(n) => n.as_f64() != Some(0.0),
        // PHP can't tell `{}` from `[]`, so both runtimes treat an empty object as false
        Json::Object(o) => !o.is_empty(),
    }
}
