//! `beacon.exe --install-hooks` implementation.
//!
//! Approach A (per docs/ARCHITECTURE.md §11 and the bootstrap plan): the
//! Windows binary embeds both shell scripts at compile time (`include_str!`)
//! and copies them into the target WSL distribution's
//! `~/.local/bin/`. The user is then instructed to run
//! `beacon-install-hooks` from WSL to merge the entries into
//! `~/.claude/settings.json`.

use std::io::Write;
use std::process::{Command, Stdio};

/// Files are compiled into the binary so `--install-hooks` works even in a
/// standalone release build where the source tree is gone.
const HOOK_SCRIPT: &str = include_str!("../../../hooks/beacon-hook");
const INSTALLER_SCRIPT: &str = include_str!("../../../hooks/beacon-install-hooks");

const DEFAULT_DISTRO: &str = "Ubuntu";

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let distro =
        std::env::var("BEACON_WSL_DISTRO").unwrap_or_else(|_| DEFAULT_DISTRO.to_string());

    println!("Beacon: installing hook scripts into WSL distribution `{distro}`.");
    println!();

    write_to_wsl(&distro, "$HOME/.local/bin/beacon-hook", HOOK_SCRIPT)?;
    write_to_wsl(
        &distro,
        "$HOME/.local/bin/beacon-install-hooks",
        INSTALLER_SCRIPT,
    )?;

    println!();
    println!("Scripts copied to ~/.local/bin/ in `{distro}`.");
    println!();
    println!("Next step — run this INSIDE WSL to merge the hooks into");
    println!("~/.claude/settings.json (existing keys are preserved):");
    println!();
    println!("  ~/.local/bin/beacon-install-hooks");
    println!();
    println!("If jq is missing: `sudo apt install jq`.");
    Ok(())
}

/// Pipe `contents` via stdin to `cat > <dest>` inside WSL. Using cat+stdin
/// avoids any path-translation headaches (no need to mount or guess
/// `/mnt/<drive>/...` paths) and sidesteps `cp` ownership quirks.
fn write_to_wsl(
    distro: &str,
    dest: &str,
    contents: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Normalize to LF so scripts run cleanly in bash, no matter how git
    // checked them out on this Windows host.
    let normalized = contents.replace("\r\n", "\n");

    let cmd = format!(
        r#"mkdir -p "$(dirname {dest})" && cat > {dest} && chmod +x {dest}"#
    );

    let mut child = Command::new("wsl.exe")
        .args(["-d", distro, "--", "bash", "-lc", &cmd])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn wsl.exe: {e}"))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("wsl.exe stdin unavailable")?;
        stdin.write_all(normalized.as_bytes())?;
    }
    // Explicitly close stdin so `cat >` sees EOF and returns.
    drop(child.stdin.take());

    let status = child.wait()?;
    if !status.success() {
        return Err(format!("wsl.exe exited with {status}").into());
    }

    println!("  wrote {dest}");
    Ok(())
}
