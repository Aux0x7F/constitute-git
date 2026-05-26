use anyhow::{Result, anyhow};
use constitute_git::{
    SourceImportRequest, SourceProjectLinkRequest, SourceRefStoreJournalRequest,
    SourceRefUpdateOptions, SourceRefUpdateRequest, apply_ref_update, build_ref_update,
    build_source_graph_fixture, build_source_ref_store_journal, build_status, default_now,
    default_source_graph_state, import_snapshot, link_project_work, load_source_graph_state,
    reduce_fixture_ref_update, replay_source_ref_store_journal, save_source_graph_state,
    source_graph_status,
};
use constitute_protocol::{
    SourceAppliedRefProjection, SourceRefStoreCurrentEntry, SourceRefStoreJournal, SourceRefUpdate,
    sha256_hex, validate_source_ref_update,
};
use std::{env, fs};

fn main() -> Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "--help" || args[0] == "help" {
        print_help();
        return Ok(());
    }

    match args.remove(0).as_str() {
        "fixture" => fixture_command(args),
        "init" => init_command(args),
        "import" => import_command(args),
        "project" => project_command(args),
        "ref" => ref_command(args),
        "store" => store_command(args),
        "status" => status_command(args),
        command => Err(anyhow!("unsupported constitute-git command: {command}")),
    }
}

fn fixture_command(args: Vec<String>) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("graph") | None => print_json(&build_source_graph_fixture(default_now())?),
        Some(name) => Err(anyhow!("unsupported fixture: {name}")),
    }
}

fn ref_command(mut args: Vec<String>) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("apply") => {
            args.remove(0);
            let (state_path, request) = parse_ref_apply_args(args)?;
            let mut state = load_source_graph_state(&state_path, request.now)?;
            let update = apply_ref_update(&mut state, request)?;
            save_source_graph_state(&state_path, &state)?;
            print_json(&update)
        }
        Some("update") => {
            args.remove(0);
            let options = parse_ref_update_options(args)?;
            let update = build_ref_update(options);
            validate_source_ref_update(&update)?;
            print_json(&update)
        }
        Some("reduce") => {
            args.remove(0);
            let request = parse_ref_update_request(args)?;
            print_json(&reduce_fixture_ref_update(request)?)
        }
        Some(name) => Err(anyhow!("unsupported ref command: {name}")),
        None => Err(anyhow!("ref command requires a subcommand")),
    }
}

fn init_command(args: Vec<String>) -> Result<()> {
    let (state_path, now) = parse_state_and_now(args)?;
    let state = default_source_graph_state(now)?;
    save_source_graph_state(&state_path, &state)?;
    print_json(&source_graph_status(&state)?)
}

fn import_command(mut args: Vec<String>) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("snapshot") => {
            args.remove(0);
            let (state_path, request) = parse_import_snapshot_args(args)?;
            let mut state = load_source_graph_state(&state_path, request.now)?;
            let outcome = import_snapshot(&mut state, request)?;
            save_source_graph_state(&state_path, &state)?;
            print_json(&outcome)
        }
        Some(name) => Err(anyhow!("unsupported import command: {name}")),
        None => Err(anyhow!("import command requires a subcommand")),
    }
}

fn project_command(mut args: Vec<String>) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("link") => {
            args.remove(0);
            let (state_path, request) = parse_project_link_args(args)?;
            let mut state = load_source_graph_state(&state_path, request.now)?;
            let outcome = link_project_work(&mut state, request)?;
            save_source_graph_state(&state_path, &state)?;
            print_json(&outcome)
        }
        Some(name) => Err(anyhow!("unsupported project command: {name}")),
        None => Err(anyhow!("project command requires a subcommand")),
    }
}

fn store_command(mut args: Vec<String>) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("journal") => {
            args.remove(0);
            let request = parse_store_journal_args(args)?;
            print_json(&build_source_ref_store_journal(request)?)
        }
        Some("replay") => {
            args.remove(0);
            let (journal, expected_target_ref, evidence_refs, observed_at) =
                parse_store_replay_args(args)?;
            let expected_target_ref =
                expected_target_ref.unwrap_or_else(|| journal.target_ref.clone());
            print_json(&replay_source_ref_store_journal(
                &journal,
                &expected_target_ref,
                evidence_refs,
                observed_at,
            )?)
        }
        Some(name) => Err(anyhow!("unsupported store command: {name}")),
        None => Err(anyhow!("store command requires a subcommand")),
    }
}

fn status_command(args: Vec<String>) -> Result<()> {
    let (state_path, now, has_state) = parse_optional_state_and_now(args)?;
    if has_state {
        let state = load_source_graph_state(&state_path, now)?;
        print_json(&source_graph_status(&state)?)
    } else {
        print_json(&build_status()?)
    }
}

fn parse_ref_update_options(args: Vec<String>) -> Result<SourceRefUpdateOptions> {
    let mut state = "applied".to_string();
    let mut branch = "main".to_string();
    let mut from_snapshot_ref = Some("source:snapshot:parent".to_string());
    let mut to_snapshot_ref = "source:snapshot:head".to_string();
    let mut writer_ref = "identity:device:agent".to_string();
    let mut now = default_now();

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| anyhow!("{flag} requires a value"))?;
        match flag.as_str() {
            "--state" => state = value,
            "--branch" => branch = value,
            "--from" => from_snapshot_ref = Some(value),
            "--no-from" => {
                if value != "true" {
                    return Err(anyhow!("--no-from expects true"));
                }
                from_snapshot_ref = None;
            }
            "--to" => to_snapshot_ref = value,
            "--writer" => writer_ref = value,
            "--now" => now = value.parse::<u64>()?,
            _ => return Err(anyhow!("unsupported ref update flag: {flag}")),
        }
    }

    Ok(SourceRefUpdateOptions {
        state,
        branch,
        from_snapshot_ref,
        to_snapshot_ref,
        writer_ref,
        now,
    })
}

fn parse_ref_update_request(args: Vec<String>) -> Result<SourceRefUpdateRequest> {
    let mut branch = "main".to_string();
    let mut from_snapshot_ref = Some("source:snapshot:parent".to_string());
    let mut to_snapshot_ref = "source:snapshot:head".to_string();
    let mut writer_ref = "identity:device:agent".to_string();
    let mut evidence_refs = vec!["source:evidence:signed-update".to_string()];
    let mut witness_refs = vec!["source:witness:runtime".to_string()];
    let mut now = default_now();

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| anyhow!("{flag} requires a value"))?;
        match flag.as_str() {
            "--branch" => branch = value,
            "--from" => from_snapshot_ref = Some(value),
            "--no-from" => {
                if value != "true" {
                    return Err(anyhow!("--no-from expects true"));
                }
                from_snapshot_ref = None;
            }
            "--to" => to_snapshot_ref = value,
            "--writer" => writer_ref = value,
            "--no-evidence" => {
                if value != "true" {
                    return Err(anyhow!("--no-evidence expects true"));
                }
                evidence_refs.clear();
            }
            "--no-witness" => {
                if value != "true" {
                    return Err(anyhow!("--no-witness expects true"));
                }
                witness_refs.clear();
            }
            "--now" => now = value.parse::<u64>()?,
            _ => return Err(anyhow!("unsupported ref reduce flag: {flag}")),
        }
    }

    Ok(SourceRefUpdateRequest {
        branch,
        from_snapshot_ref,
        to_snapshot_ref,
        writer_ref,
        evidence_refs,
        witness_refs,
        now,
    })
}

fn parse_ref_apply_args(args: Vec<String>) -> Result<(String, SourceRefUpdateRequest)> {
    let mut state_path = "target/source-graph-state.json".to_string();
    let mut branch = "main".to_string();
    let mut from_snapshot_ref = Some("source:snapshot:parent".to_string());
    let mut to_snapshot_ref = "source:snapshot:head".to_string();
    let mut writer_ref = "identity:device:agent".to_string();
    let mut evidence_refs = vec!["source:evidence:signed-update".to_string()];
    let mut witness_refs = vec!["source:witness:runtime".to_string()];
    let mut now = default_now();

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| anyhow!("{flag} requires a value"))?;
        match flag.as_str() {
            "--state" => state_path = value,
            "--branch" => branch = value,
            "--from" => from_snapshot_ref = Some(value),
            "--no-from" => {
                if value != "true" {
                    return Err(anyhow!("--no-from expects true"));
                }
                from_snapshot_ref = None;
            }
            "--to" => to_snapshot_ref = value,
            "--writer" => writer_ref = value,
            "--evidence" => evidence_refs.push(value),
            "--witness" => witness_refs.push(value),
            "--no-default-evidence" => {
                if value != "true" {
                    return Err(anyhow!("--no-default-evidence expects true"));
                }
                evidence_refs.clear();
            }
            "--no-default-witness" => {
                if value != "true" {
                    return Err(anyhow!("--no-default-witness expects true"));
                }
                witness_refs.clear();
            }
            "--now" => now = value.parse::<u64>()?,
            _ => return Err(anyhow!("unsupported ref apply flag: {flag}")),
        }
    }

    Ok((
        state_path,
        SourceRefUpdateRequest {
            branch,
            from_snapshot_ref,
            to_snapshot_ref,
            writer_ref,
            evidence_refs,
            witness_refs,
            now,
        },
    ))
}

fn parse_import_snapshot_args(args: Vec<String>) -> Result<(String, SourceImportRequest)> {
    let mut state_path = "target/source-graph-state.json".to_string();
    let mut commit_ref = "git:commit:0000003".to_string();
    let mut tree_ref = "git:tree:0000003".to_string();
    let mut parent_snapshot_refs = vec!["source:snapshot:head".to_string()];
    let mut storage_object_refs = vec![content_addressed_storage_ref("pack-next")];
    let mut author_ref = "identity:device:agent".to_string();
    let mut message_digest_ref = "digest:sha256:next-message".to_string();
    let mut signature_refs = vec!["signature:source:next".to_string()];
    let mut evidence_refs = vec!["source:evidence:pack-import".to_string()];
    let mut tool_ref = "tool:git:pack-import".to_string();
    let mut input_ref = "git:pack:next".to_string();
    let mut now = default_now() + 1;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| anyhow!("{flag} requires a value"))?;
        match flag.as_str() {
            "--state" => state_path = value,
            "--commit" => commit_ref = value,
            "--tree" => tree_ref = value,
            "--parent" => parent_snapshot_refs.push(value),
            "--clear-default-parent" => {
                if value != "true" {
                    return Err(anyhow!("--clear-default-parent expects true"));
                }
                parent_snapshot_refs.clear();
            }
            "--storage-object" => storage_object_refs.push(value),
            "--clear-default-storage-object" => {
                if value != "true" {
                    return Err(anyhow!("--clear-default-storage-object expects true"));
                }
                storage_object_refs.clear();
            }
            "--author" => author_ref = value,
            "--message-digest" => message_digest_ref = value,
            "--signature" => signature_refs.push(value),
            "--clear-default-signature" => {
                if value != "true" {
                    return Err(anyhow!("--clear-default-signature expects true"));
                }
                signature_refs.clear();
            }
            "--evidence" => evidence_refs.push(value),
            "--tool" => tool_ref = value,
            "--input" => input_ref = value,
            "--now" => now = value.parse::<u64>()?,
            _ => return Err(anyhow!("unsupported import snapshot flag: {flag}")),
        }
    }

    Ok((
        state_path,
        SourceImportRequest {
            commit_ref,
            tree_ref,
            parent_snapshot_refs,
            storage_object_refs,
            author_ref,
            message_digest_ref,
            signature_refs,
            evidence_refs,
            tool_ref,
            input_ref,
            now,
        },
    ))
}

fn content_addressed_storage_ref(value: &str) -> String {
    format!("storage:object:{}", sha256_hex(value))
}

fn parse_project_link_args(args: Vec<String>) -> Result<(String, SourceProjectLinkRequest)> {
    let mut state_path = "target/source-graph-state.json".to_string();
    let mut project_refs = vec!["project:constituency".to_string()];
    let mut work_item_refs = vec!["work-item:git-project-hardening".to_string()];
    let mut actor_ref = "identity:device:agent".to_string();
    let mut evidence_refs = vec!["adapter:project:workflow-link".to_string()];
    let mut now = default_now() + 4;
    let mut expires_at = Some(now + 86_400_000);

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| anyhow!("{flag} requires a value"))?;
        match flag.as_str() {
            "--state" => state_path = value,
            "--project" => project_refs.push(value),
            "--clear-default-project" => {
                if value != "true" {
                    return Err(anyhow!("--clear-default-project expects true"));
                }
                project_refs.clear();
            }
            "--work-item" => work_item_refs.push(value),
            "--clear-default-work-item" => {
                if value != "true" {
                    return Err(anyhow!("--clear-default-work-item expects true"));
                }
                work_item_refs.clear();
            }
            "--actor" => actor_ref = value,
            "--evidence" => evidence_refs.push(value),
            "--clear-default-evidence" => {
                if value != "true" {
                    return Err(anyhow!("--clear-default-evidence expects true"));
                }
                evidence_refs.clear();
            }
            "--now" => {
                now = value.parse::<u64>()?;
                expires_at = Some(now + 86_400_000);
            }
            "--expires-at" => expires_at = Some(value.parse::<u64>()?),
            "--no-expires" => {
                if value != "true" {
                    return Err(anyhow!("--no-expires expects true"));
                }
                expires_at = None;
            }
            _ => return Err(anyhow!("unsupported project link flag: {flag}")),
        }
    }

    Ok((
        state_path,
        SourceProjectLinkRequest {
            project_refs,
            work_item_refs,
            actor_ref,
            evidence_refs,
            now,
            expires_at,
        },
    ))
}

fn parse_store_journal_args(args: Vec<String>) -> Result<SourceRefStoreJournalRequest> {
    let mut input_path = None;
    let mut source_graph_ref = String::new();
    let mut prior_path = None;
    let mut source_ref_update_paths = Vec::new();
    let mut storage_object_refs = Vec::new();
    let mut storage_availability_refs = Vec::new();
    let mut storage_pin_intent_refs = Vec::new();
    let mut storage_pin_attestation_refs = Vec::new();
    let mut evidence_refs = vec!["evidence:source-ref-store:constitute-git".to_string()];
    let mut updated_at = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| anyhow!("{flag} requires a value"))?;
        match flag.as_str() {
            "--input" => input_path = Some(value),
            "--source-graph" => source_graph_ref = value,
            "--prior" => prior_path = Some(value),
            "--source-ref-update" => source_ref_update_paths.push(value),
            "--storage-object" => storage_object_refs.push(value),
            "--storage-availability" => storage_availability_refs.push(value),
            "--storage-pin-intent" => storage_pin_intent_refs.push(value),
            "--storage-pin-attestation" => storage_pin_attestation_refs.push(value),
            "--evidence" => evidence_refs.push(value),
            "--updated-at" => updated_at = Some(value),
            _ => return Err(anyhow!("unsupported store journal flag: {flag}")),
        }
    }

    let input_path = input_path.ok_or_else(|| anyhow!("store journal requires --input"))?;
    let applied_projection = read_json_file::<SourceAppliedRefProjection>(&input_path)?;
    let prior_transitions = if let Some(path) = prior_path {
        read_json_file::<Vec<SourceRefStoreCurrentEntry>>(&path)?
    } else {
        Vec::new()
    };
    let mut source_ref_updates = Vec::new();
    for path in source_ref_update_paths {
        source_ref_updates.push(read_json_file::<SourceRefUpdate>(&path)?);
    }
    Ok(SourceRefStoreJournalRequest {
        source_graph_ref,
        applied_projection,
        prior_transitions,
        source_ref_updates,
        storage_object_refs,
        storage_availability_refs,
        storage_pin_intent_refs,
        storage_pin_attestation_refs,
        evidence_refs,
        updated_at,
    })
}

fn parse_store_replay_args(
    args: Vec<String>,
) -> Result<(
    SourceRefStoreJournal,
    Option<String>,
    Vec<String>,
    Option<String>,
)> {
    let mut input_path = None;
    let mut expected_target_ref = None;
    let mut evidence_refs = vec!["evidence:source-ref-store-replay:constitute-git".to_string()];
    let mut observed_at = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| anyhow!("{flag} requires a value"))?;
        match flag.as_str() {
            "--input" => input_path = Some(value),
            "--expected-target" => expected_target_ref = Some(value),
            "--evidence" => evidence_refs.push(value),
            "--observed-at" => observed_at = Some(value),
            _ => return Err(anyhow!("unsupported store replay flag: {flag}")),
        }
    }

    let input_path = input_path.ok_or_else(|| anyhow!("store replay requires --input"))?;
    Ok((
        read_json_file::<SourceRefStoreJournal>(&input_path)?,
        expected_target_ref,
        evidence_refs,
        observed_at,
    ))
}

fn parse_state_and_now(args: Vec<String>) -> Result<(String, u64)> {
    let (state_path, now, _) = parse_optional_state_and_now(args)?;
    Ok((state_path, now))
}

fn parse_optional_state_and_now(args: Vec<String>) -> Result<(String, u64, bool)> {
    let mut state_path = "target/source-graph-state.json".to_string();
    let mut now = default_now();
    let mut has_state = false;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| anyhow!("{flag} requires a value"))?;
        match flag.as_str() {
            "--state" => {
                state_path = value;
                has_state = true;
            }
            "--now" => now = value.parse::<u64>()?,
            _ => return Err(anyhow!("unsupported status/init flag: {flag}")),
        }
    }

    Ok((state_path, now, has_state))
}

fn read_json_file<T: serde::de::DeserializeOwned>(path: &str) -> Result<T> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_help() {
    println!(
        "constitute-git\n\nCommands:\n  init --state target/source-graph-state.json\n  fixture graph\n  import snapshot --state target/source-graph-state.json [--commit git:commit:next] [--storage-object storage:object:pack]\n  project link --state target/source-graph-state.json [--project project:constituency] [--work-item work-item:git-project-hardening]\n  ref update [--state applied|blocked|rejected] [--branch main] [--from source:snapshot:old] [--to source:snapshot:new]\n  ref reduce [--branch main] [--from source:snapshot:parent] [--to source:snapshot:head] [--writer identity:device:agent]\n  ref apply --state target/source-graph-state.json [--branch main] [--from source:snapshot:parent] [--to source:snapshot:head]\n  store journal --input target/applied-source-ref.json [--source-ref-update target/source-ref-update.json] [--storage-object storage:object:store]\n  store replay --input target/source-ref-store.json [--expected-target source:ref:native-dev:repo:main]\n  status [--state target/source-graph-state.json]"
    );
}
