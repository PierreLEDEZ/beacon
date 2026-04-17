// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--install-hooks") {
        // Release builds use the `windows` subsystem so no console is
        // attached by default; attach the parent's (cmd/pwsh) so
        // println! output reaches the user. Failure is ignored: in debug
        // builds a console is already present.
        #[cfg(windows)]
        unsafe {
            use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }

        match beacon_lib::install::run() {
            Ok(()) => return,
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    }

    beacon_lib::run();
}
