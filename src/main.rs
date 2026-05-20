use anyhow::{Result, anyhow};
use constitute_git::{
    SourceRefUpdateOptions, build_ref_update, build_source_graph_fixture, build_status, default_now,
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
        "ref" => ref_command(args),
        "status" => print_json(&build_status()?),
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
        Some("update") => {
            args.remove(0);
            let options = parse_ref_update_options(args)?;
            let update = build_ref_update(options);
            validate_source_ref_update(&update)?;
            print_json(&update)
        }
        Some(name) => Err(anyhow!("unsupported ref command: {name}")),
        None => Err(anyhow!("ref command requires a subcommand")),
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

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_help() {
    println!(
        "constitute-git\n\nCommands:\n  fixture graph\n  ref update [--state applied|blocked|rejected] [--branch main] [--from source:snapshot:old] [--to source:snapshot:new]\n  status"
    );
}
