// Compiled by the regression tests into a native cargo/cargo.exe. No shell needed.
use std::{env, fs, io::Write, process::{Command, ExitCode}};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => { eprintln!("{error}"); ExitCode::FAILURE }
    }
}

fn run() -> Result<u8, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let kind = args.first().map(String::as_str).unwrap_or("");
    let program = env::current_exe()?.file_stem().ok_or("missing program")?.to_string_lossy().to_string();
    let state = std::path::PathBuf::from(env::var("TOOL_STATE_DIR")?);
    if program == "rustup" && kind == "component" {
        let component = args.get(2).ok_or("missing component")?;
        let mut log = fs::OpenOptions::new().append(true).create(true).open(env::var("INSTALL_LOG")?)?;
        writeln!(log, "rustup {}", args.join(" "))?;
        if env::var("FAIL_INSTALL").ok().as_deref() == Some(component) { return Ok(23); }
        fs::create_dir_all(&state)?;
        fs::write(state.join(if component == "rustfmt" { "fmt" } else { component }), "installed")?;
        return Ok(0);
    }
    if kind == "install" {
        let mut log = fs::OpenOptions::new().append(true).create(true).open(env::var("INSTALL_LOG")?)?;
        writeln!(log, "cargo {}", args.join(" "))?;
        if env::var("FAIL_INSTALL").ok().as_deref() == Some("deny") { return Ok(23); }
        fs::create_dir_all(&state)?;
        fs::write(state.join("deny"), "installed")?;
        return Ok(0);
    }
    if kind == "run" {
        // Exercise the real shared Rust entry point called by the installed hook.
        let code = args.iter().position(|s| s == "code").ok_or("missing code command")?;
        let status = Command::new(env::var_os("CODE_TEST_BIN").ok_or("missing test binary")?)
            .args(&args[code + 1..]).status()?;
        return Ok(if status.success() { 0 } else { 1 });
    }
    if kind == "metadata" {
        print!("{}", fs::read_to_string(env::var("METADATA_FILE")?)?);
        return Ok(0);
    }
    if args.get(1).map(String::as_str) == Some("--version") {
        let missing = env::var("MISSING_TOOL").ok().as_deref() == Some(kind)
            || env::var("MISSING_TOOLS").ok().is_some_and(|tools| tools.split(',').any(|tool| tool == kind));
        return Ok(if missing && !state.join(kind).exists() { 23 } else { 0 });
    }
    let logged_args = if kind == "clippy" {
        let strict = args.len() >= 3
            && args[args.len() - 3] == "--"
            && args[args.len() - 2] == "-D"
            && args[args.len() - 1] == "warnings";
        if !strict {
            return Err("code test clippy must end with `-- -D warnings`".into());
        }
        &args[..args.len() - 3]
    } else {
        &args[..]
    };
    let mut log = fs::OpenOptions::new().append(true).create(true).open(env::var("CHECK_LOG")?)?;
    writeln!(log, "{}", logged_args.join(" "))?;
    if kind == "test" {
        println!("Running unittests src/lib.rs");
        if env::var("FAIL_CHECK").ok().as_deref() == Some("test") {
            println!("test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured");
        } else {
            println!("test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured");
            println!("Running tests/regression.rs");
            println!("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured");
            println!("Doc-tests fixture");
            println!("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured");
        }
    }
    Ok(if env::var("FAIL_CHECK").ok().as_deref() == Some(kind) { 23 } else { 0 })
}
