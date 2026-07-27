use regex::Regex;
use serde_json::{Map, Value};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug)]
pub struct SenpaiError {
    pub code: i32,
    pub name: &'static str,
    pub message: String,
    pub details: Vec<Value>,
}
impl SenpaiError {
    pub fn new(code: i32, name: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            name,
            message: message.into(),
            details: vec![],
        }
    }
}

pub fn strip_jsonc(input: &str) -> Result<String, SenpaiError> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut quoted = false;
    let mut escaped = false;
    while let Some(c) = chars.next() {
        if quoted {
            out.push(c);
            if escaped {
                escaped = false
            } else if c == '\\' {
                escaped = true
            } else if c == '"' {
                quoted = false
            };
            continue;
        }
        if c == '"' {
            quoted = true;
            out.push(c);
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for x in chars.by_ref() {
                if x == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut closed = false;
            while let Some(x) = chars.next() {
                if x == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    closed = true;
                    break;
                }
                if x == '\n' {
                    out.push('\n');
                }
            }
            if !closed {
                return Err(SenpaiError::new(
                    4,
                    "invalid_manifest",
                    "Unterminated JSONC block comment.",
                ));
            }
            continue;
        }
        out.push(c);
    }
    Ok(out)
}

pub fn parse_jsonc(path: &Path, code: i32, kind: &'static str) -> Result<Value, SenpaiError> {
    let input = fs::read_to_string(path).map_err(|e| {
        SenpaiError::new(code, kind, format!("Cannot read {}: {e}", path.display()))
    })?;
    serde_json::from_str(&strip_jsonc(&input)?).map_err(|e| {
        SenpaiError::new(
            code,
            kind,
            format!("Invalid JSONC in {}: {e}", path.display()),
        )
    })
}

pub fn find_manifest(from: &Path) -> Result<PathBuf, SenpaiError> {
    let mut here = fs::canonicalize(from).map_err(|e| {
        SenpaiError::new(
            3,
            "manifest_not_found",
            format!("Cannot resolve launch directory {}: {e}", from.display()),
        )
    })?;
    if here.is_file() {
        here.pop();
    }
    loop {
        for name in [".senpai.jsonc", ".senpai.local.jsonc"] {
            let candidate = here.join(name);
            if candidate.is_file() {
                return fs::canonicalize(candidate)
                    .map_err(|e| SenpaiError::new(3, "manifest_not_found", e.to_string()));
            }
        }
        if !here.pop() {
            break;
        }
    }
    Err(SenpaiError::new(
        3,
        "manifest_not_found",
        "No .senpai.jsonc or .senpai.local.jsonc found while walking upward from the requested directory.",
    ))
}

pub fn deep_merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                if v.is_null() {
                    a.remove(&k);
                } else if let Some(old) = a.get_mut(&k) {
                    deep_merge(old, v);
                } else {
                    a.insert(k, v);
                }
            }
        }
        (a, b) => *a = b,
    }
}

pub struct Loaded {
    pub path: PathBuf,
    pub dir: PathBuf,
    pub value: Value,
}
pub fn load() -> Result<Loaded, SenpaiError> {
    load_from(&std::env::current_dir().unwrap())
}
pub fn load_from(from: &Path) -> Result<Loaded, SenpaiError> {
    let path = find_manifest(from)?;
    let dir = path.parent().unwrap().to_path_buf();
    let mut value = parse_jsonc(&path, 4, "invalid_manifest")?;
    let overlay = dir.join(".senpai.local.jsonc");
    if path.file_name().is_some_and(|name| name == ".senpai.jsonc") && overlay.is_file() {
        let o = parse_jsonc(&overlay, 4, "invalid_overlay")?;
        deep_merge(&mut value, o);
    }
    validate(&value)?;
    Ok(Loaded { path, dir, value })
}
fn object<'a>(v: &'a Value, key: &str) -> Option<&'a Map<String, Value>> {
    v.get(key)?.as_object()
}
fn ids(v: &Value, key: &str) -> BTreeSet<String> {
    object(v, key)
        .map(|x| x.keys().cloned().collect())
        .unwrap_or_default()
}
fn relative_path(value: &str) -> bool {
    value == "."
        || (!value.contains('\\')
            && !value.starts_with('/')
            && !value
                .split('/')
                .any(|p| p == "." || p == ".." || p.is_empty()))
}
pub fn validate(v: &Value) -> Result<(), SenpaiError> {
    let schema: Value = serde_json::from_str(include_str!("../schema/senpai.schema.json"))
        .expect("the bundled manifest schema must be valid JSON");
    let validator = jsonschema::validator_for(&schema)
        .expect("the bundled manifest schema must be a valid JSON Schema");
    if let Some(error) = validator.iter_errors(v).next() {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            format!("Manifest does not match the v2 schema: {error}"),
        ));
    }
    let root = v.as_object().ok_or_else(|| {
        SenpaiError::new(4, "invalid_manifest", "Manifest root must be an object.")
    })?;
    if root.get("version") != Some(&Value::from(2)) {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            "Only manifest version 2 is supported.",
        ));
    }
    let project = object(v, "project")
        .ok_or_else(|| SenpaiError::new(4, "invalid_manifest", "project is required."))?;
    for key in ["name", "label", "context", "stack"] {
        if !project.contains_key(key) {
            return Err(SenpaiError::new(
                4,
                "invalid_manifest",
                format!("project.{key} is required."),
            ));
        }
    }
    let repos = ids(v, "repos");
    let envs = ids(v, "environments");
    let integrations = object(v, "integrations")
        .ok_or_else(|| SenpaiError::new(4, "invalid_manifest", "integrations is required."))?;
    validate_integrations(v, integrations)?;
    if let Some(rs) = object(v, "repos") {
        for (id, r) in rs {
            let o = r.as_object().ok_or_else(|| {
                SenpaiError::new(
                    4,
                    "invalid_manifest",
                    format!("repos.{id} must be an object."),
                )
            })?;
            let p = o.get("path").and_then(Value::as_str).ok_or_else(|| {
                SenpaiError::new(
                    4,
                    "invalid_manifest",
                    format!("repos.{id}.path is required."),
                )
            })?;
            if !relative_path(p) {
                return Err(SenpaiError::new(
                    4,
                    "invalid_manifest",
                    format!("repos.{id}.path must be normalized relative POSIX path."),
                ));
            }
            for dep in o
                .get("depends_on")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if !repos.contains(dep) {
                    return Err(SenpaiError::new(
                        4,
                        "invalid_manifest",
                        format!("repos.{id} depends_on unknown repo {dep}."),
                    ));
                }
            }
            if let Some(h) = o.get("integrations").and_then(Value::as_object) {
                for inst in h.keys() {
                    if !integrations
                        .get(inst)
                        .is_some_and(|integration| integration["kind"] == "forge")
                    {
                        return Err(SenpaiError::new(
                            4,
                            "invalid_manifest",
                            format!(
                                "repos.{id} references unknown code-platform integration {inst}."
                            ),
                        ));
                    }
                }
            }
        }
    }
    if let Some(es) = object(v, "environments") {
        for (id, e) in es {
            if let Some(r) = e.get("repo").and_then(Value::as_str)
                && !repos.contains(r)
            {
                return Err(SenpaiError::new(
                    4,
                    "invalid_manifest",
                    format!("environments.{id} references unknown repo {r}."),
                ));
            }
        }
    }
    if let Some(cs) = object(v, "capsules") {
        for (id, c) in cs {
            validate_capsule(id, c, &repos, &envs, object(v, "environments"))?;
        }
    }
    if let Some(ds) = object(v, "docs") {
        for (id, d) in ds {
            if let Some(r) = d.get("repo").and_then(Value::as_str)
                && !repos.contains(r)
            {
                return Err(SenpaiError::new(
                    4,
                    "invalid_manifest",
                    format!("docs.{id} references unknown repo {r}."),
                ));
            }
        }
    }
    Ok(())
}
const TICKET_OPERATIONS: &[&str] = &[
    "ticket.read",
    "ticket.create",
    "ticket.update",
    "ticket.comment",
    "ticket.transition",
    "ticket.link",
    "ticket.log_time",
];
const CODE_OPERATIONS: &[&str] = &[
    "code.read",
    "code.create",
    "code.update",
    "code.comment",
    "code.request_review",
    "code.merge",
    "code.pipeline_read",
    "code.pipeline_trigger",
];
const TICKET_POLICY: &[&str] = &[
    "read",
    "create",
    "update",
    "comment",
    "transition",
    "link",
    "log_time",
];
const CODE_POLICY: &[&str] = &[
    "read",
    "create",
    "update",
    "comment",
    "request_review",
    "merge",
    "pipeline_read",
    "pipeline_trigger",
];

fn validate_integrations(
    root: &Value,
    integrations: &Map<String, Value>,
) -> Result<(), SenpaiError> {
    for (id, integration) in integrations {
        let kind = integration["kind"].as_str().unwrap_or_default();
        let (operations, policies) = if kind == "ticketing" {
            (TICKET_OPERATIONS, TICKET_POLICY)
        } else {
            (CODE_OPERATIONS, CODE_POLICY)
        };
        let provides = integration["provides"].as_array().unwrap();
        for operation in provides.iter().filter_map(Value::as_str) {
            if !operations.contains(&operation) {
                return Err(SenpaiError::new(
                    4,
                    "invalid_manifest",
                    format!(
                        "integrations.{id}.provides contains unsupported operation {operation}."
                    ),
                ));
            }
        }
        for operation in integration["handles"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
        {
            if !operations.contains(&operation)
                || !provides
                    .iter()
                    .any(|provided| provided.as_str() == Some(operation))
            {
                return Err(SenpaiError::new(
                    4,
                    "invalid_manifest",
                    format!(
                        "integrations.{id}.handles operation {operation} is not provided by its adapter."
                    ),
                ));
            }
        }
        if let Some(policy) = integration
            .get("workflow")
            .and_then(|workflow| workflow.get("policy"))
            .and_then(Value::as_object)
        {
            for capability in policy.keys() {
                if !policies.contains(&capability.as_str()) {
                    return Err(SenpaiError::new(
                        4,
                        "invalid_manifest",
                        format!(
                            "integrations.{id}.workflow.policy contains unsupported capability {capability}."
                        ),
                    ));
                }
            }
        }
        if kind != "ticketing" && integration.get("routing").is_some() {
            return Err(SenpaiError::new(
                4,
                "invalid_manifest",
                format!("integrations.{id}.routing is only valid for ticketing integrations."),
            ));
        }
        let has_override = integration.get("adapter").is_some()
            || root
                .get("adapter_overrides")
                .and_then(|overrides| overrides.get(kind))
                .and_then(|platforms| platforms.get(integration["platform"].as_str()?))
                .is_some();
        let shipped = matches!(
            (kind, integration["platform"].as_str()),
            ("ticketing", Some("jira" | "redmine")) | ("forge", Some("github" | "gitlab"))
        );
        if !shipped && !has_override {
            return Err(SenpaiError::new(
                4,
                "invalid_manifest",
                format!(
                    "integrations.{id} has no shipped adapter for {kind}/{}; declare an adapter override.",
                    integration["platform"]
                ),
            ));
        }
    }
    Ok(())
}
fn placeholders(value: &str) -> Result<Vec<String>, SenpaiError> {
    let re = Regex::new(r"\{([A-Za-z][A-Za-z0-9_-]*)\}").unwrap();
    let stripped = re.replace_all(value, "");
    if stripped.contains('{') || stripped.contains('}') {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            "Capsule program or argument has unmatched braces.",
        ));
    }
    Ok(re.captures_iter(value).map(|c| c[1].to_string()).collect())
}

fn is_interpreter(program: &str, args: &[Value]) -> bool {
    let name = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program)
        .to_ascii_lowercase();
    if name == "bun" {
        return !matches!(
            args,
            [first, second, ..]
                if first.as_str() == Some("run")
                    && second
                        .as_str()
                        .is_some_and(|script| {
                            Regex::new("^[A-Za-z][A-Za-z0-9:_-]*$")
                                .unwrap()
                                .is_match(script)
                        })
        );
    }
    matches!(
        name.as_str(),
        "sh" | "bash"
            | "dash"
            | "zsh"
            | "fish"
            | "ksh"
            | "csh"
            | "tcsh"
            | "pwsh"
            | "powershell"
            | "cmd"
            | "python"
            | "python3"
            | "node"
            | "deno"
            | "ruby"
            | "perl"
            | "php"
            | "lua"
            | "busybox"
            | "env"
    )
}
fn validate_capsule(
    id: &str,
    c: &Value,
    repos: &BTreeSet<String>,
    envs: &BTreeSet<String>,
    envdefs: Option<&Map<String, Value>>,
) -> Result<(), SenpaiError> {
    let o = c.as_object().ok_or_else(|| {
        SenpaiError::new(
            4,
            "invalid_manifest",
            format!("capsules.{id} must be object."),
        )
    })?;
    let program = o.get("program").and_then(Value::as_str).ok_or_else(|| {
        SenpaiError::new(
            4,
            "invalid_manifest",
            format!("capsules.{id}.program is required."),
        )
    })?;
    if !placeholders(program)?.is_empty() {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            format!("capsules.{id}.program cannot contain a placeholder."),
        ));
    }
    let args = o.get("args").and_then(Value::as_array).ok_or_else(|| {
        SenpaiError::new(
            4,
            "invalid_manifest",
            format!("capsules.{id}.args must be an array."),
        )
    })?;
    if is_interpreter(program, args) {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            format!("capsules.{id}.program must not be a shell or language interpreter."),
        ));
    }
    let mut ph = Vec::new();
    for arg in args {
        let arg = arg.as_str().ok_or_else(|| {
            SenpaiError::new(
                4,
                "invalid_manifest",
                format!("capsules.{id}.args must contain only strings."),
            )
        })?;
        let names = placeholders(arg)?;
        if names.len() > 1 {
            return Err(SenpaiError::new(
                4,
                "invalid_manifest",
                format!("capsules.{id} has multiple placeholders in one argv element."),
            ));
        }
        ph.extend(names);
    }
    let unique: BTreeSet<_> = ph.iter().collect();
    if unique.len() != ph.len() {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            format!("capsules.{id} repeats a placeholder."),
        ));
    }
    let supplied: BTreeSet<String> = o
        .get("supplied")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    for x in &supplied {
        if !ph.iter().any(|p| p == x) {
            return Err(SenpaiError::new(
                4,
                "invalid_manifest",
                format!("capsules.{id}.supplied contains undeclared placeholder {x}."),
            ));
        }
    }
    if let Some(cwd) = o.get("cwd").and_then(Value::as_str)
        && !relative_path(cwd)
    {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            format!("capsules.{id}.cwd must be normalized."),
        ));
    }
    let repo = o.get("repo").and_then(Value::as_str);
    if let Some(r) = repo
        && !repos.contains(r)
    {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            format!("capsules.{id} references unknown repo {r}."),
        ));
    }
    if let Some(e) = o.get("environment").and_then(Value::as_str) {
        if !envs.contains(e) {
            return Err(SenpaiError::new(
                4,
                "invalid_manifest",
                format!("capsules.{id} references unknown environment {e}."),
            ));
        }
        if let (Some(r), Some(defs)) = (repo, envdefs)
            && let Some(er) = defs
                .get(e)
                .and_then(|x| x.get("repo"))
                .and_then(Value::as_str)
            && r != er
        {
            return Err(SenpaiError::new(
                4,
                "invalid_manifest",
                format!("capsules.{id} repo and environment repo disagree."),
            ));
        }
    }
    Ok(())
}
pub fn capsule_locals(v: &Value) -> Result<HashMap<String, BTreeSet<String>>, SenpaiError> {
    let mut out = HashMap::new();
    if let Some(cs) = object(v, "capsules") {
        for (id, c) in cs {
            let o = c.as_object().unwrap();
            let supplied: BTreeSet<String> = o
                .get("supplied")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            let locals = o["args"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .flat_map(|arg| placeholders(arg).unwrap())
                .filter(|x| !supplied.contains(x))
                .collect();
            out.insert(id.clone(), locals);
        }
    }
    Ok(out)
}
pub fn normalize_under(dir: &Path, input: &str) -> Result<String, SenpaiError> {
    let candidate = if Path::new(input).is_absolute() {
        PathBuf::from(input)
    } else {
        std::env::current_dir().unwrap().join(input)
    };
    let normalized = canonicalize_existing_prefix(&normalize(&candidate))?;
    let base = canonicalize_existing_prefix(&normalize(dir))?;
    if !normalized.starts_with(&base) {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            "Path is outside the manifest directory.",
        ));
    }
    Ok(normalized
        .strip_prefix(base)
        .unwrap()
        .to_string_lossy()
        .trim_start_matches('/')
        .to_string()
        .if_empty("."))
}

fn canonicalize_existing_prefix(path: &Path) -> Result<PathBuf, SenpaiError> {
    let mut existing = path;
    let mut suffix = Vec::new();
    while !existing.exists() {
        let component = existing.file_name().ok_or_else(|| {
            SenpaiError::new(4, "invalid_manifest", "Cannot normalize filesystem path.")
        })?;
        suffix.push(component.to_os_string());
        existing = existing.parent().ok_or_else(|| {
            SenpaiError::new(4, "invalid_manifest", "Cannot normalize filesystem path.")
        })?;
    }
    let mut canonical = fs::canonicalize(existing)
        .map_err(|error| SenpaiError::new(4, "invalid_manifest", error.to_string()))?;
    for component in suffix.iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}
trait IfEmpty {
    fn if_empty(self, other: &str) -> String;
}
impl IfEmpty for String {
    fn if_empty(self, other: &str) -> String {
        if self.is_empty() { other.into() } else { self }
    }
}
pub fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for x in p.components() {
        match x {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest(program: &str, args: Value) -> Value {
        json!({
            "version": 2,
            "project": {"name": "demo", "label": "Demo", "context": "Test", "stack": []},
            "integrations": {"origin": {"kind": "forge", "platform": "gitlab", "url": "https://git.example", "provides": ["code.read"], "handles": ["code.read"]}},
            "repos": {"app": {"path": "."}},
            "environments": {"local": {"label": "Local", "repo": "app"}},
            "capsules": {
                "test": {"label": "Test", "type": "test", "program": program, "args": args, "repo": "app", "environment": "local"}
            }
        })
    }

    #[test]
    fn jsonc_removes_comments_without_corrupting_urls_or_strings() {
        let parsed: Value = serde_json::from_str(
            &strip_jsonc(
                r#"{/* metadata */ "url": "https://example.test//path", // comment
                "text": "/* literal */"}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(parsed["url"], "https://example.test//path");
        assert_eq!(parsed["text"], "/* literal */");
        assert!(strip_jsonc("{/* unfinished").is_err());
    }

    #[test]
    fn overlay_merging_recurses_and_null_removes_a_shared_value() {
        let mut base = json!({"project": {"label": "Shared", "stack": ["Rust"]}, "docs": {"guide": {"url": "https://example.test"}}});
        deep_merge(
            &mut base,
            json!({"project": {"label": "Personal"}, "docs": {"guide": null}}),
        );
        assert_eq!(base["project"]["label"], "Personal");
        assert_eq!(base["project"]["stack"], json!(["Rust"]));
        assert!(base["docs"].get("guide").is_none());
    }

    #[test]
    fn manifest_discovery_accepts_a_standalone_local_manifest() {
        let workspace = tempfile::tempdir().unwrap();
        let nested = workspace.path().join("app/src");
        fs::create_dir_all(&nested).unwrap();
        let local = workspace.path().join(".senpai.local.jsonc");
        fs::write(&local, "{}").unwrap();

        assert_eq!(
            find_manifest(&nested).unwrap(),
            fs::canonicalize(&local).unwrap()
        );

        let shared = workspace.path().join(".senpai.jsonc");
        fs::write(&shared, "{}").unwrap();
        assert_eq!(
            find_manifest(&nested).unwrap(),
            fs::canonicalize(&shared).unwrap()
        );
    }

    #[test]
    fn validation_rejects_shell_and_language_interpreters_in_capsules() {
        let error = validate(&manifest("sh", json!(["-c", "echo shell-executed"]))).unwrap_err();
        assert_eq!(error.code, 4);
        assert!(error.message.contains("interpreter"));
        assert!(validate(&manifest("python3", json!(["-c", "print('executed')"]))).is_err());
        assert!(validate(&manifest("bun", json!(["run", "test:cli"]))).is_ok());
        assert!(validate(&manifest("bun", json!(["-e", "console.log('executed')"]))).is_err());
    }

    #[test]
    fn validation_accepts_repo_labels_and_rejects_duplicates() {
        let mut valid = manifest("printf", json!(["ok"]));
        valid["repos"]["app"]["labels"] = json!(["backend", "critical"]);
        assert!(validate(&valid).is_ok());

        valid["repos"]["app"]["labels"] = json!(["backend", "backend"]);
        assert!(validate(&valid).is_err());
    }

    #[test]
    fn capsule_locals_excludes_agent_supplied_values() {
        let value = manifest("printf", json!(["%s:%s", "{token}", "{message}"]));
        let mut value = value;
        value["capsules"]["test"]["supplied"] = json!(["message"]);
        assert_eq!(
            capsule_locals(&value).unwrap()["test"],
            BTreeSet::from(["token".to_owned()])
        );
    }

    #[test]
    fn path_lookup_accepts_a_symlinked_manifest_path_but_rejects_outside_paths() {
        use std::os::unix::fs::symlink;

        let workspace = tempfile::tempdir().unwrap();
        let real = workspace.path().join("real");
        let link = workspace.path().join("link");
        fs::create_dir(&real).unwrap();
        symlink(&real, &link).unwrap();

        assert_eq!(
            normalize_under(&real, &link.join("nested/missing").to_string_lossy()).unwrap(),
            "nested/missing"
        );
        assert!(
            normalize_under(&real, &workspace.path().join("elsewhere").to_string_lossy()).is_err()
        );
    }
}
