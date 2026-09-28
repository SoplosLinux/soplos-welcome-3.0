// Privileged-execution helper, gated by a real polkit policy
// (resources/polkit/org.soploslinux.welcome.policy, installed to
// /usr/share/polkit-1/actions/). Every privileged script goes through here:
// software::start_pkexec_job writes the (already Rust-fixed or validated —
// see security plan points 1 and 3) script to a private temp file, then
// spawns `pkexec <this binary> --run-privileged-script <path>`, which
// main() catches before Tauri starts and routes to run_privileged_script_file
// below. polkit authorizes pkexec to run this specific binary under the
// declared action; nothing else on the system gets that authorization only
// because it also happens to call pkexec.

use std::fs;
use std::process::Command;

/// Reads the script written by start_pkexec_job, deletes the temp file (it
/// may contain package names or paths — no reason to leave it lying around
/// after use) and runs it via `sh -c`, inheriting stdio so the caller keeps
/// seeing live output exactly as it did when this ran as `pkexec sh -c`
/// directly. Returns the process exit code for `std::process::exit`.
pub fn run_privileged_script_file(path: &str) -> i32 {
    let script = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("soplos-welcome: could not read privileged script at {path}: {e}");
            return 1;
        }
    };
    let _ = fs::remove_file(path);

    match Command::new("sh").arg("-c").arg(&script).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("soplos-welcome: failed to run privileged script: {e}");
            1
        }
    }
}
