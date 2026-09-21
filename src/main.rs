mod config;
mod engine;
mod keys;
mod platform;

use config::Config;
use rdev::EventType;
use std::{
    io::{self, Read},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop_signal(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

fn run() -> Result<(), String> {
    let mut path = config::config_path();
    let mut mode = "run";
    let mut controlled = false;
    let mut permission_request = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => path = PathBuf::from(args.next().ok_or("--config requires a path")?),
            "--identify" | "-i" => mode = "identify",
            "--check-config" => mode = "check",
            "--ui-decode" => mode = "ui-decode",
            "--ui-encode" => mode = "ui-encode",
            "--print-config" => mode = "print",
            "--controlled" => controlled = true,
            "--permissions" => mode = "permissions",
            "--request-permission" => {
                let kind = args
                    .next()
                    .ok_or("--request-permission requires keyboard or input")?;
                if kind != "keyboard" && kind != "input" {
                    return Err("Expected keyboard or input".into());
                }
                permission_request = Some(kind);
                mode = "permissions";
            }
            "--help" | "-h" => {
                println!("mouse-macro [--config PATH] [--identify | --print-config | --check-config]\n--check-config validates TOML from stdin.\n--controlled stops gracefully on stdin input or EOF.\nmacOS UI: run scripts/build-macos.sh, then open target/Mouse Macro.app");
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }
    if mode == "permissions" {
        println!(
            "{}",
            serde_json::to_string(&platform::permissions(permission_request.as_deref()))
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if matches!(mode, "check" | "ui-decode" | "ui-encode") {
        let mut text = String::new();
        io::stdin()
            .take(4 * 1024 * 1024 + 1)
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        if text.len() > 4 * 1024 * 1024 {
            return Err("Configuration exceeds 4 MB".into());
        }
        match mode {
            "ui-decode" => {
                let mut config = Config::parse(&text)?;
                config.bindings = config.normalized_bindings();
                config.mouse_button = None;
                config.keys = None;
                println!(
                    "{}",
                    serde_json::to_string(&config).map_err(|e| e.to_string())?
                );
            }
            "ui-encode" => {
                let config: Config = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                config.resolve()?;
                println!(
                    "{}",
                    toml::to_string_pretty(&config).map_err(|e| e.to_string())?
                );
            }
            _ => {
                Config::parse(&text)?;
                println!("Configuration is valid");
            }
        }
        return Ok(());
    }
    if mode == "identify" {
        println!("Click mouse buttons to identify them. Ctrl+C exits.");
        return platform::listen(|event| {
            if let EventType::ButtonPress(button) = event.event_type {
                println!("{button:?}");
            }
        });
    }
    let config = Config::load(&path)?;
    if mode == "print" {
        println!(
            "{}",
            toml::to_string_pretty(&config).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let bindings = config.resolve()?;
    if bindings.is_empty() {
        return Err("No bindings configured".into());
    }
    platform::check_playback_permission()?;
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGINT, stop_signal as *const () as libc::sighandler_t);
        libc::signal(
            libc::SIGTERM,
            stop_signal as *const () as libc::sighandler_t,
        );
    }
    if controlled {
        thread::spawn(|| {
            let _ = io::stdin().read(&mut [0]);
            STOP.store(true, Ordering::Relaxed);
        });
    }
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut engine = engine::Engine::new(bindings, Instant::now());
        let mut output = platform::simulate_key;
        let result = (|| -> Result<(), String> {
            while !STOP.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(2)) {
                    Ok((button, down)) => {
                        engine.button(button, down, Instant::now(), &mut output)?
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                engine.tick(Instant::now(), &mut output)?;
            }
            Ok(())
        })();
        let result = result.and(engine.stop(&mut output));
        if let Err(e) = &result {
            eprintln!("Playback error: {e}");
        }
        if result.is_ok() && !STOP.load(Ordering::Relaxed) {
            return;
        }
        // The platform listener owns an OS run loop; exit only after all keys are released.
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    });
    platform::listen(move |event| {
        let message = match event.event_type {
            EventType::ButtonPress(b) => Some((b, true)),
            EventType::ButtonRelease(b) => Some((b, false)),
            _ => None,
        };
        if let Some(message) = message {
            let _ = tx.send(message);
        }
    })
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
