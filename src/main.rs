use anyhow::{Result, anyhow};
use constitute_git::{
    SourceImportRequest, SourceRefUpdateOptions, SourceRefUpdateRequest, apply_ref_update,
    build_ref_update, build_source_graph_fixture, build_status, default_now,
    default_source_graph_state, import_snapshot, load_source_graph_state,
    reduce_fixture_ref_update, save_source_graph_state, source_graph_status,
};
use constitute_protocol::validate_source_ref_update;
use std::env;

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
        "ref" => ref_command(args),
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
    let mut storage_object_refs = vec!["storage:object:pack-next".to_string()];
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

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_help() {
    println!(
        "constitute-git\n\nCommands:\n  init --state target/source-graph-state.json\n  fixture graph\n  import snapshot --state target/source-graph-state.json [--commit git:commit:next] [--storage-object storage:object:pack]\n  ref update [--state applied|blocked|rejected] [--branch main] [--from source:snapshot:old] [--to source:snapshot:new]\n  ref reduce [--branch main] [--from source:snapshot:parent] [--to source:snapshot:head] [--writer identity:device:agent]\n  ref apply --state target/source-graph-state.json [--branch main] [--from source:snapshot:parent] [--to source:snapshot:head]\n  status [--state target/source-graph-state.json]"
    );
}
