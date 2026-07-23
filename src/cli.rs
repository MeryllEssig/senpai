use crate::manifest::*;
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn err(e: SenpaiError, json_mode: bool) -> i32 {
    if json_mode {
        println!(
            "{}",
            json!({"ok":false,"error":{"code":e.name,"message":e.message,"details":e.details}})
        );
    } else {
        eprintln!("{}: {}", e.name, e.message);
    }
    e.code
}
fn ok(data: Value, json_mode: bool) {
    if json_mode {
        println!("{}", json!({"ok":true,"data":data,"warnings":[]}));
    } else {
        println!("{}", markdown(&data));
    }
}
fn markdown(v: &Value) -> String {
    match v {
        Value::Object(o) => o
            .iter()
            .map(|(k, v)| {
                format!(
                    "- {k}: {}",
                    if v.is_string() {
                        v.as_str().unwrap().to_string()
                    } else {
                        v.to_string()
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => v.to_string(),
    }
}
fn get_flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|x| x[0] == name).map(|x| x[1].clone())
}
fn has(args: &[String], s: &str) -> bool {
    args.iter().any(|x| x == s)
}
fn manifest_arg(args: &[String]) -> Result<Option<&str>, SenpaiError> {
    if let Some(i) = args.iter().position(|x| x == "--manifest") {
        return args.get(i + 1).map(|x| Some(x.as_str())).ok_or_else(|| {
            SenpaiError::new(2, "invalid_arguments", "--manifest needs an absolute path.")
        });
    }
    Ok(None)
}
fn map_with_id(id: &str, v: &Value, section: &str) -> Value {
    let mut o = v.as_object().cloned().unwrap_or_default();
    o.insert("id".into(), Value::String(id.into()));
    o.insert("section".into(), Value::String(section.into()));
    Value::Object(o)
}
fn objects<'a>(v: &'a Value, key: &str) -> Option<&'a Map<String, Value>> {
    v.get(key)?.as_object()
}
fn role_select(
    items: &Map<String, Value>,
    role: &str,
    explicit: Option<&str>,
    kind: &str,
) -> Result<(String, Value), SenpaiError> {
    let candidates: Vec<_> = items
        .iter()
        .filter(|(_, v)| {
            v.get("roles")
                .and_then(Value::as_array)
                .is_some_and(|rs| rs.iter().any(|x| x.as_str() == Some(role)))
        })
        .collect();
    if let Some(x) = explicit {
        let v = items
            .get(x)
            .ok_or_else(|| SenpaiError::new(3, "not_found", format!("Unknown {kind} {x}.")))?;
        if !candidates.iter().any(|(i, _)| *i == x) {
            return Err(SenpaiError::new(
                3,
                "capability_not_declared",
                format!("{kind} {x} does not have role {role}."),
            ));
        }
        return Ok((x.into(), v.clone()));
    }
    if candidates.len() == 1 {
        return Ok((candidates[0].0.clone(), candidates[0].1.clone()));
    }
    if candidates.is_empty() {
        return Err(SenpaiError::new(
            3,
            "capability_not_declared",
            format!("No {kind} has role {role}."),
        ));
    }
    let mut e = SenpaiError::new(
        5,
        "ambiguous_route",
        format!("More than one {kind} has role {role}."),
    );
    e.details = candidates.iter().map(|(x, _)| json!({"id":x})).collect();
    Err(e)
}
fn summary(l: &Loaded) -> Value {
    let v = &l.value;
    let trackers = objects(v, "trackers")
        .and_then(|x| x.get("sources"))
        .and_then(Value::as_object)
        .map(|x| {
            x.iter()
                .map(|(id, s)| json!({"id":id,"roles":s.get("roles")}))
                .collect::<Vec<_>>()
        });
    let hosting = objects(v, "code_hosting")
        .and_then(|x| x.get("instances"))
        .and_then(Value::as_object)
        .map(|x| {
            x.iter()
                .map(|(id, s)| json!({"id":id,"roles":s.get("roles")}))
                .collect::<Vec<_>>()
        });
    let capsules=objects(v,"capsules").map(|x|x.iter().map(|(id,c)|json!({"id":id,"type":c.get("type"),"repo":c.get("repo"),"environment":c.get("environment")})).collect::<Vec<_>>());
    json!({"manifest_path":l.path,"project":v["project"]["name"],"sections":v.as_object().unwrap().keys().filter(|x|*x!="$schema"&&*x!="version"&&*x!="project").collect::<Vec<_>>(),"trackers":trackers,"hosting":hosting,"repos":objects(v,"repos").map(|x|x.keys().collect::<Vec<_>>()),"environments":objects(v,"environments").map(|x|x.keys().collect::<Vec<_>>()),"capsules":capsules,"workflows":{"tickets":workflow(v,"tickets"),"code_changes":workflow(v,"code_changes")}})
}
fn workflow(v: &Value, domain: &str) -> Value {
    let caps = if domain == "tickets" {
        vec![
            "read",
            "create",
            "update",
            "comment",
            "transition",
            "link",
            "log_time",
        ]
    } else {
        vec![
            "read",
            "create",
            "update",
            "comment",
            "request_review",
            "merge",
            "pipeline_read",
            "pipeline_trigger",
        ]
    };
    let d = v.get("workflows").and_then(|x| x.get(domain));
    let skill = d
        .and_then(|x| x.get("skill"))
        .cloned()
        .unwrap_or(Value::String(
            if domain == "tickets" {
                "senpai-project-use-ticket-workflow"
            } else {
                "senpai-project-use-code-hosting-workflow"
            }
            .into(),
        ));
    let mut p = Map::new();
    for cap in caps {
        p.insert(
            cap.into(),
            d.and_then(|x| x.get("policy"))
                .and_then(|p| p.get(cap))
                .cloned()
                .unwrap_or(Value::String(
                    if cap == "read" { "allow" } else { "deny" }.into(),
                )),
        );
    }
    json!({"domain":domain,"skill":skill,"policy":p})
}
fn get_command(l: &Loaded, args: &[String]) -> Result<Value, SenpaiError> {
    let v = &l.value;
    let topic = args
        .get(1)
        .map(String::as_str)
        .ok_or_else(|| SenpaiError::new(2, "invalid_arguments", "get requires a topic."))?;
    match topic {
        "tracker" => {
            let role = get_flag(args, "--role")
                .ok_or_else(|| SenpaiError::new(2, "invalid_arguments", "--role is required."))?;
            let x = objects(v, "trackers")
                .and_then(|x| x.get("sources"))
                .and_then(Value::as_object)
                .ok_or_else(|| SenpaiError::new(3, "not_found", "No trackers declared."))?;
            let (id, val) = role_select(
                x,
                &role,
                get_flag(args, "--source").as_deref(),
                "tracker source",
            )?;
            Ok(map_with_id(&id, &val, "trackers.sources"))
        }
        "ticket-route" => ticket_route(v, args),
        "hosting" => {
            let role = get_flag(args, "--role")
                .ok_or_else(|| SenpaiError::new(2, "invalid_arguments", "--role is required."))?;
            let repo = get_flag(args, "--repo")
                .ok_or_else(|| SenpaiError::new(2, "invalid_arguments", "--repo is required."))?;
            let r = objects(v, "repos")
                .and_then(|x| x.get(&repo))
                .ok_or_else(|| SenpaiError::new(3, "not_found", format!("Unknown repo {repo}.")))?;
            let x = objects(v, "code_hosting")
                .and_then(|x| x.get("instances"))
                .and_then(Value::as_object)
                .ok_or_else(|| SenpaiError::new(3, "not_found", "No code hosting declared."))?;
            let (id, val) = role_select(
                x,
                &role,
                get_flag(args, "--instance").as_deref(),
                "hosting instance",
            )?;
            if !r
                .get("hosting")
                .and_then(Value::as_object)
                .is_some_and(|h| h.contains_key(&id))
            {
                return Err(SenpaiError::new(
                    3,
                    "capability_not_declared",
                    format!("Hosting instance {id} is not declared for repo {repo}."),
                ));
            }
            let mut result = map_with_id(&id, &val, "code_hosting.instances");
            result
                .as_object_mut()
                .unwrap()
                .insert("repo".into(), Value::String(repo));
            result
                .as_object_mut()
                .unwrap()
                .insert("repository".into(), r["hosting"][&id].clone());
            Ok(result)
        }
        "workflow" => {
            let d = get_flag(args, "--domain")
                .ok_or_else(|| SenpaiError::new(2, "invalid_arguments", "--domain is required."))?;
            if d != "tickets" && d != "code_changes" {
                return Err(SenpaiError::new(
                    2,
                    "invalid_arguments",
                    "--domain must be tickets or code_changes.",
                ));
            }
            Ok(workflow(v, &d))
        }
        "repo" => repo_get(l, args),
        "environment" => one(
            v,
            "environments",
            get_flag(args, "--id").as_deref(),
            "environments",
        ),
        "capsule" => one(v, "capsules", get_flag(args, "--id").as_deref(), "capsules"),
        "docs" => {
            if let Some(id) = get_flag(args, "--id") {
                one(v, "docs", Some(&id), "docs")
            } else {
                Ok(Value::Array(
                    objects(v, "docs")
                        .map(|x| x.iter().map(|(i, a)| map_with_id(i, a, "docs")).collect())
                        .unwrap_or_default(),
                ))
            }
        }
        "rules" => Ok(v.get("rules").cloned().unwrap_or(Value::Array(vec![]))),
        _ => Err(SenpaiError::new(
            2,
            "invalid_arguments",
            format!("Unknown get topic {topic}."),
        )),
    }
}
fn one(v: &Value, section: &str, id: Option<&str>, label: &str) -> Result<Value, SenpaiError> {
    let id = id.ok_or_else(|| SenpaiError::new(2, "invalid_arguments", "--id is required."))?;
    let x = objects(v, section)
        .and_then(|x| x.get(id))
        .ok_or_else(|| SenpaiError::new(3, "not_found", format!("Unknown {label} id {id}.")))?;
    Ok(map_with_id(id, x, section))
}
fn ticket_route(v: &Value, args: &[String]) -> Result<Value, SenpaiError> {
    let id = get_flag(args, "--id")
        .ok_or_else(|| SenpaiError::new(2, "invalid_arguments", "--id is required."))?;
    let x = objects(v, "trackers")
        .and_then(|x| x.get("sources"))
        .and_then(Value::as_object)
        .ok_or_else(|| SenpaiError::new(3, "not_found", "No trackers declared."))?;
    if let Some(s) = get_flag(args, "--source") {
        let a = x.get(&s).ok_or_else(|| {
            SenpaiError::new(3, "not_found", format!("Unknown tracker source {s}."))
        })?;
        return Ok(map_with_id(&s, a, "trackers.sources"));
    }
    let mut candidates: Vec<(&String, &Value, i64)> = vec![];
    for (sid, s) in x {
        let matches = s
            .get("ticket_id_patterns")
            .and_then(Value::as_array)
            .is_some_and(|ps| {
                ps.iter().filter_map(Value::as_str).any(|p| {
                    regex::Regex::new(p)
                        .map(|r| r.is_match(&id))
                        .unwrap_or(false)
                })
            });
        if matches {
            candidates.push((
                sid,
                s,
                s.get("priority")
                    .and_then(Value::as_i64)
                    .unwrap_or(i64::MAX),
            ));
        }
    }
    if candidates.len() == 1 {
        return Ok(map_with_id(
            candidates[0].0,
            candidates[0].1,
            "trackers.sources",
        ));
    }
    if candidates.is_empty() {
        return Err(SenpaiError::new(
            3,
            "not_found",
            format!("No tracker source matches ticket id {id}."),
        ));
    }
    candidates.sort_by_key(|x| x.2);
    if candidates.len() > 1 && candidates[0].2 == candidates[1].2 {
        let mut e = SenpaiError::new(5, "ambiguous_route", "Ticket id matches multiple sources.");
        e.details = candidates.iter().map(|x| json!({"id":x.0})).collect();
        return Err(e);
    }
    Ok(map_with_id(
        candidates[0].0,
        candidates[0].1,
        "trackers.sources",
    ))
}
fn repo_get(l: &Loaded, args: &[String]) -> Result<Value, SenpaiError> {
    let rs = objects(&l.value, "repos")
        .ok_or_else(|| SenpaiError::new(3, "not_found", "No repos declared."))?;
    if let Some(id) = get_flag(args, "--id") {
        let r = rs
            .get(&id)
            .ok_or_else(|| SenpaiError::new(3, "not_found", format!("Unknown repo {id}.")))?;
        return repo_deps(rs, &id, r, has(args, "--with-dependencies"));
    }
    let raw = if has(args, "--current") {
        std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string()
    } else {
        get_flag(args, "--path").ok_or_else(|| {
            SenpaiError::new(2, "invalid_arguments", "Use --id, --path, or --current.")
        })?
    };
    let p = normalize_under(&l.dir, &raw)?;
    let mut candidates: Vec<_> = rs
        .iter()
        .filter(|(_, r)| {
            let rp = r["path"].as_str().unwrap();
            rp == "." || p == *rp || p.starts_with(&(rp.to_owned() + "/"))
        })
        .collect();
    candidates.sort_by_key(|(_, r)| std::cmp::Reverse(r["path"].as_str().unwrap().len()));
    if candidates.is_empty() {
        return Err(SenpaiError::new(
            3,
            "not_found",
            "No declared repo contains this path.",
        ));
    }
    if candidates.len() > 1
        && candidates[0].1["path"].as_str().unwrap().len()
            == candidates[1].1["path"].as_str().unwrap().len()
    {
        return Err(SenpaiError::new(
            5,
            "ambiguous_route",
            "Multiple repo declarations match this path.",
        ));
    }
    repo_deps(
        rs,
        candidates[0].0,
        candidates[0].1,
        has(args, "--with-dependencies"),
    )
}
fn repo_deps(
    rs: &Map<String, Value>,
    id: &str,
    r: &Value,
    with: bool,
) -> Result<Value, SenpaiError> {
    let mut x = map_with_id(id, r, "repos");
    if with {
        x.as_object_mut().unwrap().insert(
            "dependencies".into(),
            Value::Array(
                r.get("depends_on")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .filter_map(|d| rs.get(d).map(|v| map_with_id(d, v, "repos")))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
        );
    }
    Ok(x)
}
fn list_capsules(v: &Value, args: &[String]) -> Value {
    let repo = get_flag(args, "--repo");
    let env = get_flag(args, "--env");
    let typ = get_flag(args, "--type");
    let envs = objects(v, "environments");
    let results = objects(v, "capsules")
        .map(|capsules| {
            capsules
                .iter()
                .filter_map(|(id, capsule)| {
                    let effective_repo = capsule.get("repo").and_then(Value::as_str).or_else(|| {
                        capsule
                            .get("environment")
                            .and_then(Value::as_str)
                            .and_then(|environment| envs.and_then(|all| all.get(environment)))
                            .and_then(|environment| environment.get("repo"))
                            .and_then(Value::as_str)
                    });
                    if repo.as_deref().is_some_and(|value| effective_repo != Some(value))
                        || env.as_deref().is_some_and(|value| capsule.get("environment").and_then(Value::as_str) != Some(value))
                        || typ.as_deref().is_some_and(|value| capsule.get("type").and_then(Value::as_str) != Some(value))
                    {
                        return None;
                    }
                    Some(json!({"id":id,"label":capsule.get("label"),"type":capsule.get("type"),"repo":effective_repo,"environment":capsule.get("environment"),"mcp":capsule.get("mcp"),"access":capsule.get("access")}))
                })
                .collect()
        })
        .unwrap_or_default();
    Value::Array(results)
}
fn init(l: &Loaded) -> Result<Value, SenpaiError> {
    let needs = capsule_locals(&l.value)?;
    if needs.values().all(BTreeSet::is_empty) {
        return Ok(json!({"values_file_created":false,"required_capsules":[]}));
    }
    let d = l.dir.join(".senpai");
    fs::create_dir_all(&d)
        .map_err(|e| SenpaiError::new(6, "local_configuration", e.to_string()))?;
    let p = d.join("capsules.local.json");
    let mut current: Map<String, Value> = if p.exists() {
        parse_jsonc(&p, 6, "local_configuration")?
            .as_object()
            .cloned()
            .ok_or_else(|| {
                SenpaiError::new(
                    6,
                    "local_configuration",
                    "Capsule values root must be object.",
                )
            })?
    } else {
        Map::new()
    };
    for (id, names) in &needs {
        if names.is_empty() {
            continue;
        }
        let entry = current
            .entry(id.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        let o = entry.as_object_mut().unwrap();
        for name in names {
            o.entry(name.clone())
                .or_insert(Value::String("REPLACE_ME".into()));
        }
    }
    let was = !p.exists();
    fs::write(&p, serde_json::to_string_pretty(&current).unwrap() + "\n")
        .map_err(|e| SenpaiError::new(6, "local_configuration", e.to_string()))?;
    let ignore = l.dir.join(".gitignore");
    let line = ".senpai/capsules.local.json";
    let contents = fs::read_to_string(&ignore).unwrap_or_default();
    if !contents.lines().any(|x| x.trim() == line) {
        fs::write(
            &ignore,
            format!(
                "{}{}\n",
                contents,
                if contents.is_empty() { "" } else { "\n" }
            ) + line,
        )
        .map_err(|e| SenpaiError::new(6, "local_configuration", e.to_string()))?;
    }
    Ok(
        json!({"values_file_created":was,"path":p,"required_capsules":needs.into_iter().filter(|(_,x)|!x.is_empty()).map(|(id,n)|json!({"id":id,"placeholders":n})).collect::<Vec<_>>() }),
    )
}
fn validate_local(l: &Loaded) -> Result<Value, SenpaiError> {
    let needs = capsule_locals(&l.value)?;
    if needs.values().all(BTreeSet::is_empty) {
        return Ok(json!({"valid":true,"values_file_required":false}));
    }
    let p = l.dir.join(".senpai/capsules.local.json");
    if !p.is_file() {
        return Err(SenpaiError::new(
            6,
            "local_configuration",
            "Local capsule values file is required but missing.",
        ));
    }
    let values = parse_jsonc(&p, 6, "local_configuration")?;
    let obj = values.as_object().ok_or_else(|| {
        SenpaiError::new(
            6,
            "local_configuration",
            "Capsule values root must be object.",
        )
    })?;
    let env = re("^[A-Za-z_][A-Za-z0-9_]*$");
    for (id, names) in needs {
        if names.is_empty() {
            continue;
        }
        let x = obj.get(&id).and_then(Value::as_object).ok_or_else(|| {
            SenpaiError::new(
                6,
                "local_configuration",
                format!("Missing local values for capsule {id}."),
            )
        })?;
        for name in names {
            let s = x.get(&name).and_then(Value::as_str).ok_or_else(|| {
                SenpaiError::new(
                    6,
                    "local_configuration",
                    format!("Missing local value {name} for capsule {id}."),
                )
            })?;
            if s.is_empty() || s == "REPLACE_ME" {
                return Err(SenpaiError::new(
                    6,
                    "local_configuration",
                    format!("Local value {name} for capsule {id} is empty or still a stub."),
                ));
            }
            if let Some(var) = s.strip_prefix('$')
                && !env.is_match(var)
            {
                return Err(SenpaiError::new(
                    6,
                    "local_configuration",
                    format!(
                        "Local value {name} for capsule {id} has invalid environment reference."
                    ),
                ));
            }
        }
    }
    Ok(json!({"valid":true,"values_file_required":true}))
}
fn re(s: &str) -> regex::Regex {
    regex::Regex::new(s).unwrap()
}
fn run_capsule(l: &Loaded, args: &[String]) -> Result<Value, SenpaiError> {
    let id = args
        .get(1)
        .ok_or_else(|| SenpaiError::new(2, "invalid_arguments", "run requires a capsule id."))?;
    let c = objects(&l.value, "capsules")
        .and_then(|x| x.get(id))
        .ok_or_else(|| SenpaiError::new(3, "not_found", format!("Unknown capsule {id}.")))?;
    let o = c.as_object().unwrap();
    let supplied: BTreeSet<String> = o
        .get("supplied")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let mut supplied_values = HashMap::new();
    let mut i = 2;
    while i < args.len() {
        let flag = &args[i];
        if flag == "--json" {
            i += 1;
            continue;
        }
        if flag == "--manifest" {
            if i + 1 >= args.len() {
                return Err(SenpaiError::new(
                    2,
                    "invalid_arguments",
                    "--manifest needs an absolute path.",
                ));
            }
            i += 2;
            continue;
        }
        if !flag.starts_with("--") || i + 1 >= args.len() {
            return Err(SenpaiError::new(
                2,
                "invalid_arguments",
                "Supplied parameters use --name value.",
            ));
        }
        let n = flag.trim_start_matches("--");
        if !supplied.contains(n)
            || supplied_values
                .insert(n.to_string(), args[i + 1].clone())
                .is_some()
        {
            return Err(SenpaiError::new(
                2,
                "invalid_arguments",
                format!("Unknown or repeated supplied parameter {flag}."),
            ));
        }
        i += 2;
    }
    if supplied_values.len() != supplied.len() {
        return Err(SenpaiError::new(
            2,
            "invalid_arguments",
            "Every declared supplied parameter must occur exactly once.",
        ));
    }
    let mut private = HashMap::new();
    let local_names = capsule_locals(&l.value)?.remove(id).unwrap_or_default();
    if !local_names.is_empty() {
        validate_local(l)?;
        let p = l.dir.join(".senpai/capsules.local.json");
        let vals = parse_jsonc(&p, 6, "local_configuration")?;
        for n in local_names {
            let raw = vals[id][&n].as_str().unwrap();
            let value = if let Some(env) = raw.strip_prefix('$') {
                std::env::var(env).map_err(|_| {
                    SenpaiError::new(
                        6,
                        "local_configuration",
                        format!("Environment reference for {n} is unavailable."),
                    )
                })?
            } else {
                raw.into()
            };
            private.insert(n, value);
        }
    }
    let template = o["command"].as_str().unwrap();
    let argv0 = shell_words::split(template)
        .map_err(|_| SenpaiError::new(4, "invalid_manifest", "Invalid capsule command."))?;
    let place = re(r"\{([A-Za-z][A-Za-z0-9_-]*)\}");
    let argv: Vec<String> = argv0
        .iter()
        .map(|a| {
            place
                .replace_all(a, |caps: &regex::Captures| {
                    supplied_values
                        .get(&caps[1])
                        .or_else(|| private.get(&caps[1]))
                        .unwrap()
                        .as_str()
                })
                .to_string()
        })
        .collect();
    let cwd = l
        .dir
        .join(o.get("cwd").and_then(Value::as_str).unwrap_or("."));
    let timeout = o
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(30);
    let limit = o
        .get("max_output_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(1_048_576) as usize;
    let stdout_file = tempfile::tempfile()
        .map_err(|error| SenpaiError::new(7, "capsule_failed", error.to_string()))?;
    let stderr_file = tempfile::tempfile()
        .map_err(|error| SenpaiError::new(7, "capsule_failed", error.to_string()))?;
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file.try_clone().map_err(|error| {
            SenpaiError::new(7, "capsule_failed", error.to_string())
        })?))
        .stderr(Stdio::from(stderr_file.try_clone().map_err(|error| {
            SenpaiError::new(7, "capsule_failed", error.to_string())
        })?))
        .spawn()
        .map_err(|e| {
            SenpaiError::new(7, "capsule_failed", format!("Capsule could not start: {e}"))
        })?;
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child
            .try_wait()
            .map_err(|e| SenpaiError::new(7, "capsule_failed", e.to_string()))?
        {
            break s;
        }
        if start.elapsed() > Duration::from_secs(timeout) {
            child.kill().ok();
            child.wait().ok();
            return capsule_error(
                template,
                "",
                "",
                None,
                "Capsule timed out.",
                private.values(),
            );
        }
        let bytes = stdout_file
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0)
            + stderr_file
                .metadata()
                .map(|metadata| metadata.len())
                .unwrap_or(0);
        if bytes > limit as u64 {
            child.kill().ok();
            child.wait().ok();
            let (stdout, stderr) = read_output_files(&stdout_file, &stderr_file);
            return capsule_error(
                template,
                &stdout,
                &stderr,
                None,
                "Capsule exceeded its output limit.",
                private.values(),
            );
        }
        thread::sleep(Duration::from_millis(10));
    };
    let (mut stdout, mut stderr) = read_output_files(&stdout_file, &stderr_file);
    let all = stdout.len() + stderr.len();
    if all > limit {
        return capsule_error(
            template,
            &stdout,
            &stderr,
            status.code(),
            "Capsule exceeded its output limit.",
            private.values(),
        );
    }
    for secret in private.values() {
        if !secret.is_empty() {
            stdout = stdout.replace(secret, "{redacted}");
            stderr = stderr.replace(secret, "{redacted}");
        }
    }
    let result = json!({"command_template":template,"stdout":stdout,"stderr":stderr,"exit_code":status.code()});
    if !status.success() {
        let mut e = SenpaiError::new(7, "capsule_failed", "Capsule process failed.");
        e.details = vec![result];
        return Err(e);
    }
    Ok(result)
}

fn read_output_files(stdout_file: &std::fs::File, stderr_file: &std::fs::File) -> (String, String) {
    let mut stdout = String::new();
    let mut stderr = String::new();
    stdout_file
        .try_clone()
        .and_then(|mut file| {
            file.seek(SeekFrom::Start(0))?;
            file.read_to_string(&mut stdout)
        })
        .ok();
    stderr_file
        .try_clone()
        .and_then(|mut file| {
            file.seek(SeekFrom::Start(0))?;
            file.read_to_string(&mut stderr)
        })
        .ok();
    (stdout, stderr)
}
fn capsule_error<'a>(
    template: &str,
    stdout: &str,
    stderr: &str,
    exit: Option<i32>,
    message: &str,
    secrets: impl Iterator<Item = &'a String>,
) -> Result<Value, SenpaiError> {
    let mut out = stdout.to_owned();
    let mut err = stderr.to_owned();
    for x in secrets {
        out = out.replace(x, "{redacted}");
        err = err.replace(x, "{redacted}");
    }
    let mut e = SenpaiError::new(7, "capsule_failed", message);
    e.details =
        vec![json!({"command_template":template,"stdout":out,"stderr":err,"exit_code":exit})];
    Err(e)
}
pub fn run(args: Vec<String>) -> i32 {
    let json_mode = has(&args, "--json");
    let result = (|| -> Result<Value, SenpaiError> {
        if args.is_empty() {
            return Err(SenpaiError::new(
                2,
                "invalid_arguments",
                "Expected a command.",
            ));
        }
        if args[0] == "--version" || args[0] == "version" {
            return Ok(json!({"version":env!("CARGO_PKG_VERSION")}));
        }
        if args[0] == "resolve" {
            let from = get_flag(&args, "--from")
                .map(PathBuf::from)
                .unwrap_or(std::env::current_dir().unwrap());
            let p = find_manifest(&from)?;
            let l = load(Some(p.to_str().unwrap()))?;
            return Ok(
                json!({"manifest_path":l.path,"manifest_directory":l.dir,"project":l.value["project"]["name"]}),
            );
        }
        let manifest = manifest_arg(&args)?;
        let l = load(manifest)?;
        match args[0].as_str() {
            "summary" => Ok(summary(&l)),
            "get" => get_command(&l, &args),
            "list" => match args.get(1).map(String::as_str) {
                Some("repos") => Ok(Value::Array(
                    objects(&l.value, "repos")
                        .map(|x| x.iter().map(|(i, a)| map_with_id(i, a, "repos")).collect())
                        .unwrap_or_default(),
                )),
                Some("capsules") => Ok(list_capsules(&l.value, &args)),
                _ => Err(SenpaiError::new(
                    2,
                    "invalid_arguments",
                    "list requires repos or capsules.",
                )),
            },
            "init" => init(&l),
            "validate" => match args.get(1).map(String::as_str) {
                Some("manifest") => Ok(json!({"valid":true})),
                Some("local") => validate_local(&l),
                _ => Err(SenpaiError::new(
                    2,
                    "invalid_arguments",
                    "validate requires manifest or local.",
                )),
            },
            "doctor" => {
                validate_local(&l)?;
                Ok(json!({"valid":true}))
            }
            "run" => run_capsule(&l, &args),
            _ => Err(SenpaiError::new(
                2,
                "invalid_arguments",
                format!("Unknown command {}.", args[0]),
            )),
        }
    })();
    match result {
        Ok(v) => {
            ok(v, json_mode);
            0
        }
        Err(e) => err(e, json_mode),
    }
}
