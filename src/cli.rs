use crate::manifest::*;
use serde_json::{Map, Value, json};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    fs,
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
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
fn reject_removed_manifest_flag(args: &[String]) -> Result<(), SenpaiError> {
    if has(args, "--manifest") {
        return Err(SenpaiError::new(
            2,
            "invalid_arguments",
            "--manifest was removed; SenpAI discovers .senpai.jsonc or .senpai.local.jsonc from the current directory.",
        ));
    }
    Ok(())
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
fn summary(l: &Loaded) -> Value {
    let v = &l.value;
    let integrations = objects(v, "integrations").map(|x| x.iter().map(|(id, integration)| {
        json!({"id":id,"kind":integration["kind"],"platform":integration["platform"],"handles":integration["handles"]})
    }).collect::<Vec<_>>());
    let capsules=objects(v,"capsules").map(|x|x.iter().map(|(id,c)|json!({"id":id,"type":c.get("type"),"repo":c.get("repo"),"environment":c.get("environment")})).collect::<Vec<_>>());
    let repos = objects(v, "repos").map(|x| {
        x.iter()
            .map(|(id, repo)| json!({"id": id, "labels": repo.get("labels")}))
            .collect::<Vec<_>>()
    });
    json!({"manifest_path":l.path,"project":v["project"]["name"],"sections":v.as_object().unwrap().keys().filter(|x|*x!="$schema"&&*x!="version"&&*x!="project").collect::<Vec<_>>(),"integrations":integrations,"repos":repos,"environments":objects(v,"environments").map(|x|x.keys().collect::<Vec<_>>()),"capsules":capsules})
}

fn operation_kind(operation: &str) -> Option<&'static str> {
    if TICKET_OPERATIONS.contains(&operation) {
        Some("ticketing")
    } else if FORGE_OPERATIONS.contains(&operation) {
        Some("forge")
    } else {
        None
    }
}
fn expanded_policy(integration: &Value, operation: &str) -> Value {
    let operations = if integration["kind"] == "ticketing" {
        TICKET_OPERATIONS
    } else {
        FORGE_OPERATIONS
    };
    let policy = integration
        .get("workflow")
        .and_then(|workflow| workflow.get("policy"));
    let mut effective = Map::new();
    for policy_operation in operations {
        effective.insert(
            (*policy_operation).into(),
            policy
                .and_then(|policy| policy.get(*policy_operation))
                .cloned()
                .unwrap_or_else(|| {
                    Value::String(
                        if is_view_operation(policy_operation) {
                            "allow"
                        } else {
                            "deny"
                        }
                        .into(),
                    )
                }),
        );
    }
    json!({"effective": effective, "decision": effective[operation]})
}
fn is_view_operation(operation: &str) -> bool {
    operation.ends_with(".view") || operation == "pipeline.job.view_log"
}
fn effective_workflow(integration: &Value) -> Value {
    integration
        .get("workflow")
        .and_then(|workflow| workflow.get("skill"))
        .cloned()
        .unwrap_or_else(|| {
            Value::String(
                if integration["kind"] == "ticketing" {
                    "senpai-project-use-ticket-workflow"
                } else {
                    "senpai-project-use-code-hosting-workflow"
                }
                .into(),
            )
        })
}
fn effective_adapter(v: &Value, integration: &Value) -> Value {
    if let Some(adapter) = integration.get("adapter") {
        return adapter.clone();
    }
    let kind = integration["kind"].as_str().unwrap();
    let platform = integration["platform"].as_str().unwrap();
    if let Some(adapter) = v
        .get("adapter_overrides")
        .and_then(|overrides| overrides.get(kind))
        .and_then(|platforms| platforms.get(platform))
    {
        return adapter.clone();
    }
    json!({"kind":"shipped", "skill": if kind == "ticketing" { "senpai-project-management" } else { "senpai-code-hosting" }, "platform":platform})
}
fn choose_route<'a>(
    mut candidates: Vec<(&'a String, &'a Value)>,
    explicit: Option<&str>,
    label: &str,
) -> Result<(&'a String, &'a Value), SenpaiError> {
    if candidates.is_empty() {
        return Err(SenpaiError::new(
            3,
            "capability_not_declared",
            format!("No integration can handle this {label} operation."),
        ));
    }
    if let Some(id) = explicit {
        let selected = candidates
            .iter()
            .find(|(candidate, _)| candidate.as_str() == id)
            .copied()
            .ok_or_else(|| {
                SenpaiError::new(
                    3,
                    "capability_not_declared",
                    format!("Integration {id} cannot handle this {label} operation."),
                )
            })?;
        return Ok(selected);
    }
    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }
    candidates.sort_by_key(|(_, integration)| {
        integration
            .get("routing")
            .and_then(|routing| routing.get("priority"))
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX)
    });
    if candidates.len() > 1 {
        let first = candidates[0]
            .1
            .get("routing")
            .and_then(|routing| routing.get("priority"))
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX);
        let second = candidates[1]
            .1
            .get("routing")
            .and_then(|routing| routing.get("priority"))
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX);
        if first == second {
            let mut error = SenpaiError::new(
                5,
                "ambiguous_route",
                format!("More than one integration can handle this {label} operation."),
            );
            error.details = candidates.iter().map(|(id, _)| json!({"id":id})).collect();
            return Err(error);
        }
    }
    Ok(candidates.remove(0))
}
fn resolve_operation(l: &Loaded, args: &[String]) -> Result<Value, SenpaiError> {
    let operation = args.get(2).map(String::as_str).ok_or_else(|| {
        SenpaiError::new(
            2,
            "invalid_arguments",
            "resolve operation requires an operation.",
        )
    })?;
    let kind = operation_kind(operation).ok_or_else(|| {
        SenpaiError::new(
            2,
            "invalid_arguments",
            "Operation must start with ticket., pull_merge_request., or pipeline.",
        )
    })?;
    let ticket = get_flag(args, "--ticket");
    let repo = get_flag(args, "--repo");
    if (kind == "ticketing" && (ticket.is_none() || repo.is_some()))
        || (kind == "forge" && (repo.is_none() || ticket.is_some()))
    {
        return Err(SenpaiError::new(
            2,
            "invalid_arguments",
            "Ticket operations require exactly --ticket; pull/merge request and pipeline operations require exactly --repo.",
        ));
    }
    let integrations = objects(&l.value, "integrations").unwrap();
    let mut candidates: Vec<_> = integrations
        .iter()
        .filter(|(_, integration)| {
            integration["kind"] == kind
                && integration
                    .get("handles")
                    .and_then(Value::as_array)
                    .is_some_and(|handles| {
                        handles
                            .iter()
                            .any(|handled| handled.as_str() == Some(operation))
                    })
        })
        .collect();
    let route = if let Some(ticket_id) = ticket.as_deref() {
        let pattern_matches: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|(_, integration)| {
                integration
                    .get("routing")
                    .and_then(|routing| routing.get("ticket_id_patterns"))
                    .and_then(Value::as_array)
                    .is_some_and(|patterns| {
                        patterns.iter().filter_map(Value::as_str).any(|pattern| {
                            regex::Regex::new(pattern).is_ok_and(|regex| regex.is_match(ticket_id))
                        })
                    })
            })
            .collect();
        if !pattern_matches.is_empty() {
            candidates = pattern_matches;
        }
        json!({"ticket":ticket_id, "normalized_ticket":ticket_id})
    } else {
        let repo_id = repo.as_deref().unwrap();
        let repository = objects(&l.value, "repos")
            .and_then(|repos| repos.get(repo_id))
            .ok_or_else(|| SenpaiError::new(3, "not_found", format!("Unknown repo {repo_id}.")))?;
        candidates.retain(|(id, _)| {
            repository
                .get("integrations")
                .and_then(Value::as_object)
                .is_some_and(|mapped| mapped.contains_key(*id))
        });
        json!({"repo":repo_id})
    };
    let (id, integration) =
        choose_route(candidates, get_flag(args, "--integration").as_deref(), kind)?;
    let route = if kind == "forge" {
        let repo_id = repo.unwrap();
        let path = l.value["repos"][&repo_id]["integrations"][id].clone();
        json!({"repo":repo_id, "repository":path})
    } else {
        route
    };
    Ok(
        json!({"integration":{"id":id,"kind":integration["kind"],"platform":integration["platform"],"url":integration["url"],"scope":integration.get("scope"),"auth":integration.get("auth")},"route":route,"operation":operation,"policy":expanded_policy(integration, operation),"workflow":{"skill":effective_workflow(integration)},"adapter":effective_adapter(&l.value, integration)}),
    )
}
fn migrate_v1_workflow(workflow: &Value, mappings: &[(&str, &str)]) -> Value {
    let mut migrated = Map::new();
    if let Some(skill) = workflow.get("skill") {
        migrated.insert("skill".into(), skill.clone());
    }
    let mut policy = Map::new();
    for (legacy, operation) in mappings {
        if let Some(decision) = workflow.get("policy").and_then(|policy| policy.get(legacy)) {
            policy.insert((*operation).into(), decision.clone());
        }
    }
    if !policy.is_empty() {
        migrated.insert("policy".into(), Value::Object(policy));
    }
    Value::Object(migrated)
}
fn migrate_v1() -> Result<Value, SenpaiError> {
    let path = find_manifest(&std::env::current_dir().unwrap())?;
    let input = parse_jsonc(&path, 4, "invalid_manifest")?;
    if input["version"] != 1 {
        return Err(SenpaiError::new(
            4,
            "invalid_manifest",
            "migrate v1 accepts only a version 1 manifest.",
        ));
    }
    fn safe_auth(auth: Option<&Value>) -> Result<Option<Value>, SenpaiError> {
        let Some(auth) = auth else { return Ok(None) };
        let object = auth
            .as_object()
            .ok_or_else(|| SenpaiError::new(4, "invalid_manifest", "v1 auth must be an object."))?;
        if !object
            .get("mode")
            .and_then(Value::as_str)
            .is_some_and(|mode| matches!(mode, "preconfigured" | "env" | "interactive"))
            || object.iter().any(|(key, value)| {
                key != "mode"
                    && (!key.ends_with("_env")
                        || !value.as_str().is_some_and(|name| {
                            regex::Regex::new("^[A-Za-z_][A-Za-z0-9_]*$")
                                .unwrap()
                                .is_match(name)
                        }))
            })
        {
            return Err(SenpaiError::new(
                4,
                "invalid_manifest",
                "v1 auth contains an unsafe or invalid field.",
            ));
        }
        Ok(Some(auth.clone()))
    }
    let mut integrations = Map::new();
    let mut report = Vec::new();
    let ticket_workflow = input
        .get("workflows")
        .and_then(|workflows| workflows.get("tickets"))
        .map(|workflow| {
            migrate_v1_workflow(
                workflow,
                &[
                    ("read", "ticket.view"),
                    ("create", "ticket.create"),
                    ("update", "ticket.edit"),
                    ("comment", "ticket.comment"),
                    ("transition", "ticket.change_status"),
                    ("link", "ticket.link"),
                    ("log_time", "ticket.log_time"),
                ],
            )
        });
    let code_workflow = input
        .get("workflows")
        .and_then(|workflows| workflows.get("code_changes"))
        .map(|workflow| {
            migrate_v1_workflow(
                workflow,
                &[
                    ("read", "pull_merge_request.view"),
                    ("create", "pull_merge_request.create"),
                    ("update", "pull_merge_request.edit"),
                    ("comment", "pull_merge_request.comment"),
                    ("request_review", "pull_merge_request.request_review"),
                    ("merge", "pull_merge_request.merge"),
                    ("pipeline_read", "pipeline.view"),
                    ("pipeline_read", "pipeline.job.view_log"),
                    ("pipeline_trigger", "pipeline.trigger"),
                ],
            )
        });
    for (id, source) in objects(&input, "trackers")
        .and_then(|trackers| trackers.get("sources"))
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let mut integration = Map::new();
        integration.insert("kind".into(), Value::String("ticketing".into()));
        integration.insert("platform".into(), source["type"].clone());
        integration.insert("url".into(), source["url"].clone());
        integration.insert("scope".into(), json!({"project":source["project"]}));
        integration.insert("provides".into(), json!(["ticket.view"]));
        integration.insert("handles".into(), json!(["ticket.view"]));
        if let Some(auth) = safe_auth(source.get("auth"))? {
            integration.insert("auth".into(), auth);
        }
        if source.get("ticket_id_patterns").is_some() || source.get("priority").is_some() {
            let mut routing = Map::new();
            if let Some(patterns) = source.get("ticket_id_patterns") {
                routing.insert("ticket_id_patterns".into(), patterns.clone());
            }
            if let Some(priority) = source.get("priority") {
                routing.insert("priority".into(), priority.clone());
            }
            integration.insert("routing".into(), Value::Object(routing));
            report.push(json!({"code":"review_ticket_routing","integration":id,"message":"Review non-native ticket patterns and routing priority."}));
        }
        if let Some(workflow) = &ticket_workflow {
            integration.insert("workflow".into(), workflow.clone());
        }
        if let Some(skill) = source.get("skill") {
            integration.insert("adapter".into(), json!({"skill":skill}));
            report.push(json!({"code":"review_adapter","integration":id,"message":"Review the migrated technical adapter override."}));
        }
        integrations.insert(id.clone(), Value::Object(integration));
        report.push(json!({"code":"review_role_mapping","integration":id,"message":"Map v1 roles to v2 operations; the draft grants read only."}));
        if source.get("time_logging").is_some() {
            report.push(json!({"code":"review_time_logging_fallback","integration":id,"message":"Review the v1 time-log fallback manually."}));
        }
    }
    for (id, instance) in objects(&input, "code_hosting")
        .and_then(|hosting| hosting.get("instances"))
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let mut integration = Map::new();
        integration.insert("kind".into(), Value::String("forge".into()));
        integration.insert("platform".into(), instance["platform"].clone());
        integration.insert("url".into(), instance["url"].clone());
        integration.insert("provides".into(), json!(["pull_merge_request.view"]));
        integration.insert("handles".into(), json!(["pull_merge_request.view"]));
        if let Some(auth) = safe_auth(instance.get("auth"))? {
            integration.insert("auth".into(), auth);
        }
        if let Some(workflow) = &code_workflow {
            integration.insert("workflow".into(), workflow.clone());
        }
        if let Some(skill) = instance.get("skill") {
            integration.insert("adapter".into(), json!({"skill":skill}));
            report.push(json!({"code":"review_adapter","integration":id,"message":"Review the migrated technical adapter override."}));
        }
        integrations.insert(id.clone(), Value::Object(integration));
        report.push(json!({"code":"review_role_mapping","integration":id,"message":"Map v1 roles to v2 operations; the draft grants read only."}));
    }
    let mut draft = input.as_object().cloned().unwrap();
    draft.insert("version".into(), Value::from(2));
    draft.insert(
        "$schema".into(),
        Value::String(
            "https://raw.githubusercontent.com/MeryllEssig/senpai/main/schema/senpai.schema.json"
                .into(),
        ),
    );
    draft.remove("trackers");
    draft.remove("code_hosting");
    if draft.remove("workflows").is_some() {
        report.push(json!({"code":"review_workflows","message":"Split each v1 global workflow across integrations."}));
    }
    if let Some(rules) = draft
        .remove("rules")
        .and_then(|rules| rules.as_array().cloned())
    {
        for (index, _) in rules.iter().enumerate() {
            report.push(json!({"code":"review_rule","index":index,"message":"Move this free-text rule into a workflow or structured configuration."}));
        }
    }
    if let Some(repos) = draft.get_mut("repos").and_then(Value::as_object_mut) {
        for repo in repos.values_mut() {
            if let Some(map) = repo.as_object_mut().and_then(|repo| repo.remove("hosting")) {
                repo.as_object_mut()
                    .unwrap()
                    .insert("integrations".into(), map);
                report.push(json!({"code":"review_mirrored_repositories","message":"Review every migrated repository integration mapping."}));
            }
        }
    }
    draft.insert("integrations".into(), Value::Object(integrations));
    Ok(json!({"draft":draft,"report":report,"written":false}))
}
fn get_command(l: &Loaded, args: &[String]) -> Result<Value, SenpaiError> {
    let v = &l.value;
    let topic = args
        .get(1)
        .map(String::as_str)
        .ok_or_else(|| SenpaiError::new(2, "invalid_arguments", "get requires a topic."))?;
    if matches!(
        topic,
        "tracker" | "ticket-route" | "hosting" | "workflow" | "rules"
    ) {
        return Err(SenpaiError::new(
            2,
            "invalid_arguments",
            format!("get {topic} was removed in manifest v2; use resolve operation."),
        ));
    }
    match topic {
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
    let program = o["program"].as_str().unwrap();
    let declared_args = o["args"].as_array().unwrap();
    let place = re(r"\{([A-Za-z][A-Za-z0-9_-]*)\}");
    let argv: Vec<String> = declared_args
        .iter()
        .map(|a| {
            place
                .replace_all(a.as_str().unwrap(), |caps: &regex::Captures| {
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
    let mut command = Command::new(program);
    command
        .args(&argv)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|e| {
        SenpaiError::new(7, "capsule_failed", format!("Capsule could not start: {e}"))
    })?;
    let exceeded = Arc::new(AtomicBool::new(false));
    let captured = Arc::new(AtomicUsize::new(0));
    let stdout = capture_output(
        child.stdout.take().unwrap(),
        limit,
        Arc::clone(&captured),
        Arc::clone(&exceeded),
    );
    let stderr = capture_output(
        child.stderr.take().unwrap(),
        limit,
        Arc::clone(&captured),
        Arc::clone(&exceeded),
    );
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child
            .try_wait()
            .map_err(|e| SenpaiError::new(7, "capsule_failed", e.to_string()))?
        {
            break s;
        }
        if start.elapsed() > Duration::from_secs(timeout) {
            kill_capsule(&mut child);
            let (stdout, stderr) = join_output(stdout, stderr);
            return capsule_error(
                program,
                declared_args,
                &stdout,
                &stderr,
                None,
                "Capsule timed out.",
                private.values(),
            );
        }
        if exceeded.load(Ordering::Relaxed) {
            kill_capsule(&mut child);
            let (stdout, stderr) = join_output(stdout, stderr);
            return capsule_error(
                program,
                declared_args,
                &stdout,
                &stderr,
                None,
                "Capsule exceeded its output limit.",
                private.values(),
            );
        }
        thread::sleep(Duration::from_millis(10));
    };
    let (mut stdout, mut stderr) = join_output(stdout, stderr);
    if exceeded.load(Ordering::Relaxed) {
        return capsule_error(
            program,
            declared_args,
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
    let result = json!({"program":program,"args":declared_args,"stdout":stdout,"stderr":stderr,"exit_code":status.code()});
    if !status.success() {
        let mut e = SenpaiError::new(7, "capsule_failed", "Capsule process failed.");
        e.details = vec![result];
        return Err(e);
    }
    Ok(result)
}
fn capture_output<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
    captured: Arc<AtomicUsize>,
    exceeded: Arc<AtomicBool>,
) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0; 8192];
        while let Ok(read) = reader.read(&mut buffer) {
            if read == 0 {
                break;
            }
            let start = captured.fetch_add(read, Ordering::Relaxed);
            let available = limit.saturating_sub(start);
            output.extend_from_slice(&buffer[..read.min(available)]);
            if read > available {
                exceeded.store(true, Ordering::Relaxed);
            }
        }
        String::from_utf8_lossy(&output).into_owned()
    })
}

fn join_output(
    stdout: thread::JoinHandle<String>,
    stderr: thread::JoinHandle<String>,
) -> (String, String) {
    (
        stdout.join().unwrap_or_default(),
        stderr.join().unwrap_or_default(),
    )
}

fn kill_capsule(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    child.kill().ok();
    child.wait().ok();
}
fn capsule_error<'a>(
    program: &str,
    args: &[Value],
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
        vec![json!({"program":program,"args":args,"stdout":out,"stderr":err,"exit_code":exit})];
    Err(e)
}

const JOB_LOG_MAX_BYTES: usize = 1_048_576;
const JOB_LOG_MAX_LINES: usize = 10_000;

struct TailBuffer {
    bytes: VecDeque<u8>,
    limit_bytes: usize,
    limit_lines: Option<usize>,
    captured_bytes: usize,
    captured_lines: usize,
    retained_lines: usize,
    truncated: bool,
}

impl TailBuffer {
    fn new(limit_bytes: usize, limit_lines: Option<usize>) -> Self {
        Self {
            bytes: VecDeque::new(),
            limit_bytes,
            limit_lines,
            captured_bytes: 0,
            captured_lines: 0,
            retained_lines: 0,
            truncated: false,
        }
    }

    fn push(&mut self, input: &[u8]) {
        self.captured_bytes += input.len();
        self.captured_lines += input.iter().filter(|byte| **byte == b'\n').count();
        for byte in input {
            self.bytes.push_back(*byte);
            if *byte == b'\n' {
                self.retained_lines += 1;
            }
        }
        self.trim();
    }

    fn trim(&mut self) {
        while self
            .limit_lines
            .is_some_and(|limit| self.retained_lines > limit)
            || self.bytes.len() > self.limit_bytes
        {
            if self.bytes.pop_front() == Some(b'\n') {
                self.retained_lines -= 1;
            }
            self.truncated = true;
        }
    }

    fn text(&self) -> String {
        let marker = b"[... log truncated; showing the final bounded window ...]\n";
        let start = if self.truncated {
            self.bytes
                .len()
                .saturating_sub(self.limit_bytes - marker.len())
        } else {
            0
        };
        let bytes: Vec<_> = self.bytes.iter().skip(start).copied().collect();
        let text = String::from_utf8_lossy(&bytes);
        if self.truncated {
            format!("{}{}", String::from_utf8_lossy(marker), text)
        } else {
            text.into_owned()
        }
    }
}

fn capture_tail<R: Read + Send + 'static>(
    mut reader: R,
    output: Arc<Mutex<TailBuffer>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0; 8192];
        while let Ok(read) = reader.read(&mut buffer) {
            if read == 0 {
                break;
            }
            output.lock().unwrap().push(&buffer[..read]);
        }
    })
}

fn url_host(value: &str) -> Option<&str> {
    value
        .split_once("://")
        .map(|(_, rest)| rest.split('/').next().unwrap_or_default())
        .filter(|host| !host.is_empty())
}

fn numeric_id(value: &str, label: &str) -> Result<String, SenpaiError> {
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(value.into())
    } else {
        Err(SenpaiError::new(
            2,
            "invalid_arguments",
            format!("{label} must be a numeric id or a supported job URL."),
        ))
    }
}

fn parse_job_target(
    platform: &str,
    declared_host: &str,
    declared_repository: &str,
    job: &str,
    pipeline: Option<String>,
) -> Result<(Option<String>, String), SenpaiError> {
    if !job.starts_with("http://") && !job.starts_with("https://") {
        return Ok((pipeline, numeric_id(job, "--job")?));
    }
    if url_host(job) != Some(declared_host) {
        return Err(SenpaiError::new(
            2,
            "invalid_arguments",
            "Job URL host does not match the resolved integration.",
        ));
    }
    let segments: Vec<_> = job
        .split_once("://")
        .map(|(_, path)| path.split('/').collect())
        .unwrap_or_default();
    match platform {
        "github" => {
            let runs = segments
                .windows(2)
                .position(|parts| parts == ["actions", "runs"])
                .ok_or_else(|| {
                    SenpaiError::new(2, "invalid_arguments", "Unsupported GitHub job URL.")
                })?;
            let jobs_index = runs + 3;
            let job_index = jobs_index + 1;
            if segments.get(jobs_index) != Some(&"jobs") {
                return Err(SenpaiError::new(
                    2,
                    "invalid_arguments",
                    "Unsupported GitHub job URL.",
                ));
            }
            if segments[1..runs].join("/") != declared_repository {
                return Err(SenpaiError::new(
                    2,
                    "invalid_arguments",
                    "Job URL repository does not match the resolved route.",
                ));
            }
            Ok((
                Some(numeric_id(segments[runs + 2], "pipeline")?),
                numeric_id(segments[job_index], "job")?,
            ))
        }
        "gitlab" => {
            let marker = segments
                .windows(2)
                .position(|parts| parts == ["-", "jobs"])
                .ok_or_else(|| {
                    SenpaiError::new(2, "invalid_arguments", "Unsupported GitLab job URL.")
                })?;
            if segments[1..marker].join("/") != declared_repository {
                return Err(SenpaiError::new(
                    2,
                    "invalid_arguments",
                    "Job URL repository does not match the resolved route.",
                ));
            }
            Ok((
                pipeline,
                numeric_id(segments.get(marker + 2).copied().unwrap_or_default(), "job")?,
            ))
        }
        _ => Err(SenpaiError::new(
            7,
            "unsupported_adapter",
            "Native job-log reading supports GitHub and GitLab only.",
        )),
    }
}

fn pipeline_job_log(l: &Loaded, args: &[String]) -> Result<Value, SenpaiError> {
    let repo = get_flag(args, "--repo").ok_or_else(|| {
        SenpaiError::new(2, "invalid_arguments", "pipeline job-log requires --repo.")
    })?;
    let job = get_flag(args, "--job").ok_or_else(|| {
        SenpaiError::new(2, "invalid_arguments", "pipeline job-log requires --job.")
    })?;
    let requested_pipeline = get_flag(args, "--pipeline");
    let mut resolve_args = vec![
        "resolve".into(),
        "operation".into(),
        "pipeline.job.view_log".into(),
        "--repo".into(),
        repo.clone(),
    ];
    if let Some(integration) = get_flag(args, "--integration") {
        resolve_args.extend(["--integration".into(), integration]);
    }
    let resolved = resolve_operation(l, &resolve_args)?;
    let decision = resolved["policy"]["decision"].as_str().unwrap_or("deny");
    if decision == "deny" {
        return Err(SenpaiError::new(
            5,
            "policy_denied",
            "The resolved policy denies pipeline.job.view_log.",
        ));
    }
    if decision == "confirm" && !has(args, "--confirm") {
        return Err(SenpaiError::new(
            5,
            "confirmation_required",
            "pipeline.job.view_log requires explicit confirmation; rerun with --confirm after confirmation.",
        ));
    }
    let platform = resolved["integration"]["platform"]
        .as_str()
        .unwrap_or_default();
    let host =
        url_host(resolved["integration"]["url"].as_str().unwrap_or_default()).ok_or_else(|| {
            SenpaiError::new(
                4,
                "invalid_manifest",
                "Forge integration URL must include a host.",
            )
        })?;
    let repository = resolved["route"]["repository"].as_str().unwrap_or_default();
    let (pipeline, job) = parse_job_target(platform, host, repository, &job, requested_pipeline)?;
    if platform == "github" && pipeline.is_none() {
        return Err(SenpaiError::new(
            2,
            "invalid_arguments",
            "GitHub job logs require --pipeline when --job is an id.",
        ));
    }
    let (program, command_args): (&str, Vec<String>) = match platform {
        "github" => (
            "gh",
            vec![
                "run".into(),
                "view".into(),
                pipeline.clone().unwrap(),
                "--log".into(),
                "--job".into(),
                job.clone(),
                "--repo".into(),
                format!("{host}/{repository}"),
            ],
        ),
        "gitlab" => {
            let mut command = vec!["ci".into(), "trace".into(), job.clone()];
            if let Some(pipeline) = pipeline.clone() {
                command.extend(["--pipeline-id".into(), pipeline]);
            }
            command.extend(["--repo".into(), format!("https://{host}/{repository}")]);
            ("glab", command)
        }
        _ => {
            return Err(SenpaiError::new(
                7,
                "unsupported_adapter",
                "Native job-log reading supports GitHub and GitLab only.",
            ));
        }
    };
    let mut command = Command::new(program);
    command
        .args(&command_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|error| {
        SenpaiError::new(
            7,
            "job_log_failed",
            format!("Could not start {program}: {error}"),
        )
    })?;
    let output = Arc::new(Mutex::new(TailBuffer::new(
        JOB_LOG_MAX_BYTES,
        Some(JOB_LOG_MAX_LINES),
    )));
    let diagnostics = Arc::new(Mutex::new(TailBuffer::new(65_536, None)));
    let stdout = capture_tail(child.stdout.take().unwrap(), Arc::clone(&output));
    let stderr = capture_tail(child.stderr.take().unwrap(), Arc::clone(&diagnostics));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| SenpaiError::new(7, "job_log_failed", error.to_string()))?
        {
            break status;
        }
        if start.elapsed() > Duration::from_secs(30) {
            kill_capsule(&mut child);
            stdout.join().ok();
            stderr.join().ok();
            return Err(SenpaiError::new(
                7,
                "job_log_failed",
                "Job log command timed out.",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    stdout.join().ok();
    stderr.join().ok();
    let output = output.lock().unwrap();
    let diagnostics = diagnostics.lock().unwrap();
    let result = json!({"operation":"pipeline.job.view_log", "repo":repo, "pipeline":pipeline, "job":job, "log":output.text(), "truncated":output.truncated, "captured_bytes":output.captured_bytes, "captured_lines":output.captured_lines, "stderr":diagnostics.text(), "stderr_truncated":diagnostics.truncated, "exit_code":status.code()});
    if !status.success() {
        let mut error = SenpaiError::new(7, "job_log_failed", "Job log command failed.");
        error.details = vec![result];
        return Err(error);
    }
    Ok(result)
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
        if args.first().is_some_and(|command| command == "migrate")
            && args.get(1).is_some_and(|version| version == "v1")
        {
            reject_removed_manifest_flag(&args)?;
            return migrate_v1();
        }
        if args.first().is_some_and(|command| command == "resolve")
            && args.get(1).is_some_and(|command| command == "operation")
        {
            reject_removed_manifest_flag(&args)?;
            let l = load()?;
            return resolve_operation(&l, &args);
        }
        if args[0] == "resolve" {
            let from = get_flag(&args, "--from")
                .map(PathBuf::from)
                .unwrap_or(std::env::current_dir().unwrap());
            let l = load_from(&from)?;
            return Ok(
                json!({"manifest_path":l.path,"manifest_directory":l.dir,"project":l.value["project"]["name"]}),
            );
        }
        reject_removed_manifest_flag(&args)?;
        let l = load()?;
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
            "pipeline"
                if args
                    .get(1)
                    .is_some_and(|subcommand| subcommand == "job-log") =>
            {
                pipeline_job_log(&l, &args)
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
