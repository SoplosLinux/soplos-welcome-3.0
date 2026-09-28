use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::utils::{is_valid_flatpak_id, is_valid_pkg_list, is_valid_pkg_name};

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatus {
    running: bool,
    success: bool,
    output: String,
    // True while a confirmable job is paused after its "pre" step, waiting
    // for confirm_pending_job() to say whether to run the "post" step —
    // used so a single pkexec authentication covers both (e.g. `apt update`
    // then, only if the user confirms, `apt upgrade -y`) instead of asking
    // for the password again for the second step.
    waiting_confirmation: bool,
}

static JOBS: Mutex<Option<HashMap<u64, JobStatus>>> = Mutex::new(None);
static JOB_STDIN: Mutex<Option<HashMap<u64, std::process::ChildStdin>>> = Mutex::new(None);

fn jobs() -> std::sync::MutexGuard<'static, Option<HashMap<u64, JobStatus>>> {
    let mut guard = JOBS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

fn job_stdin() -> std::sync::MutexGuard<'static, Option<HashMap<u64, std::process::ChildStdin>>> {
    let mut guard = JOB_STDIN.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

fn append_output_line(id: u64, line: &str) {
    if let Some(job) = jobs().as_mut().unwrap().get_mut(&id) {
        job.output.push_str(line);
        job.output.push('\n');
    }
}

/// Marker the confirmable job's shell script prints right after its "pre"
/// step, so the reader thread can tell the frontend to ask for confirmation
/// without that marker line itself showing up in the visible output.
const AWAIT_MARKER: &str = "__SOPLOS_AWAIT_CONFIRM__";

/// Starts a command on a plain OS thread and immediately returns a job id;
/// the command handler itself never blocks. stdout/stderr are streamed line
/// by line into the job's output as they arrive (mirroring the 2.x Python
/// version's GLib.idle_add-per-line pattern) instead of waiting for the
/// whole process to finish, so the frontend's poll_job() sees live progress.
pub(crate) fn start_job(program: &str, args: Vec<String>) -> u64 {
    let id = NEXT_JOB_ID.fetch_add(1, Ordering::SeqCst);
    jobs().as_mut().unwrap().insert(
        id,
        JobStatus {
            running: true,
            success: false,
            output: String::new(),
            waiting_confirmation: false,
        },
    );

    let program = program.to_string();
    std::thread::spawn(move || {
        let child = Command::new(&program)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(child) => child,
            Err(e) => {
                jobs().as_mut().unwrap().insert(
                    id,
                    JobStatus {
                        running: false,
                        success: false,
                        output: e.to_string(),
                        waiting_confirmation: false,
                    },
                );
                return;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().flatten() {
                append_output_line(id, &line);
            }
        });
        let stderr_reader = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                append_output_line(id, &line);
            }
        });

        let exit_status = child.wait();
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();

        let success = exit_status.map(|status| status.success()).unwrap_or(false);
        if let Some(job) = jobs().as_mut().unwrap().get_mut(&id) {
            job.running = false;
            job.success = success;
        }
    });

    id
}

// Writes `script` to a private temp file (mode 600) and returns the path.
// pkexec re-invokes this same binary with --run-privileged-script <path>
// instead of `pkexec sh -c "<script>"` directly, so polkit's action is tied
// to this specific binary (see privilege::run_privileged_script_file and
// resources/polkit/org.soploslinux.welcome.policy) rather than to /bin/sh,
// which every other program on the system also happens to use.
fn write_privileged_script(script: &str) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write as _;
    let n = NEXT_JOB_ID.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("soplos-welcome-privileged-{}-{n}.sh", std::process::id()));
    let mut file = std::fs::File::create(&path)?;
    file.write_all(script.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(path)
}

fn self_exe_path() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "soplos-welcome".to_string())
}

fn failed_job(message: String) -> u64 {
    let id = NEXT_JOB_ID.fetch_add(1, Ordering::SeqCst);
    jobs().as_mut().unwrap().insert(
        id,
        JobStatus {
            running: false,
            success: false,
            output: message,
            waiting_confirmation: false,
        },
    );
    id
}

pub(crate) fn start_pkexec_job(script: String) -> u64 {
    match write_privileged_script(&script) {
        Ok(path) => start_job(
            "pkexec",
            vec![self_exe_path(), "--run-privileged-script".to_string(), path.to_string_lossy().into_owned()],
        ),
        Err(e) => failed_job(format!("Could not prepare privileged script: {e}")),
    }
}

/// Like start_job, but for a single pkexec session split into two steps: the
/// `pre` step runs immediately, then the job pauses (`waiting_confirmation`)
/// until confirm_pending_job() sends the exact follow-up command to run (or
/// an empty line to skip it) — still under the SAME already-authenticated
/// pkexec process. This is how "Update" then confirming in the popup (with
/// only the packages the user picked) avoids a second password prompt,
/// without caching admin rights any more broadly than this one flow.
fn start_confirmable_pkexec_job(pre: String) -> u64 {
    let id = NEXT_JOB_ID.fetch_add(1, Ordering::SeqCst);
    jobs().as_mut().unwrap().insert(
        id,
        JobStatus {
            running: true,
            success: false,
            output: String::new(),
            waiting_confirmation: false,
        },
    );

    let script = format!(
        "{pre}\necho {AWAIT_MARKER}\nread -r __soplos_post\nif [ -n \"$__soplos_post\" ]; then\nsh -c \"$__soplos_post\"\nfi"
    );

    std::thread::spawn(move || {
        let script_path = match write_privileged_script(&script) {
            Ok(path) => path,
            Err(e) => {
                jobs().as_mut().unwrap().insert(
                    id,
                    JobStatus {
                        running: false,
                        success: false,
                        output: format!("Could not prepare privileged script: {e}"),
                        waiting_confirmation: false,
                    },
                );
                return;
            }
        };

        let child = Command::new("pkexec")
            .args([self_exe_path(), "--run-privileged-script".to_string(), script_path.to_string_lossy().into_owned()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(child) => child,
            Err(e) => {
                jobs().as_mut().unwrap().insert(
                    id,
                    JobStatus {
                        running: false,
                        success: false,
                        output: e.to_string(),
                        waiting_confirmation: false,
                    },
                );
                return;
            }
        };

        let stdin = child.stdin.take().unwrap();
        job_stdin().as_mut().unwrap().insert(id, stdin);

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().flatten() {
                if line == AWAIT_MARKER {
                    if let Some(job) = jobs().as_mut().unwrap().get_mut(&id) {
                        job.waiting_confirmation = true;
                    }
                } else {
                    append_output_line(id, &line);
                }
            }
        });
        let stderr_reader = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                append_output_line(id, &line);
            }
        });

        let exit_status = child.wait();
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();
        job_stdin().as_mut().unwrap().remove(&id);

        let success = exit_status.map(|status| status.success()).unwrap_or(false);
        if let Some(job) = jobs().as_mut().unwrap().get_mut(&id) {
            job.running = false;
            job.success = success;
            job.waiting_confirmation = false;
        }
    });

    id
}

#[tauri::command]
pub fn confirm_pending_job(job_id: u64, selection: Option<UpgradeSelection>) -> Result<(), String> {
    let command = match selection {
        Some(sel) => build_upgrade_command(&sel)?,
        None => String::new(),
    };
    let mut guard = job_stdin();
    let stdin = guard
        .as_mut()
        .unwrap()
        .get_mut(&job_id)
        .ok_or_else(|| "Job is not waiting for confirmation".to_string())?;
    // A blank line makes the paused shell's `[ -n "$__soplos_post" ]` check
    // false, skipping the follow-up step entirely.
    writeln!(stdin, "{command}").map_err(|e| e.to_string())?;
    if let Some(job) = jobs().as_mut().unwrap().get_mut(&job_id) {
        job.waiting_confirmation = false;
    }
    Ok(())
}

#[tauri::command]
pub fn poll_job(job_id: u64) -> JobStatus {
    let mut guard = jobs();
    let map = guard.as_mut().unwrap();
    match map.get(&job_id) {
        // The frontend stops polling as soon as it sees `running: false`, so
        // this is the one and only time it reads a finished job's status —
        // safe to drop it here instead of leaving it in the map forever.
        Some(status) if !status.running => map.remove(&job_id).unwrap(),
        Some(status) => status.clone(),
        None => JobStatus {
            running: false,
            success: false,
            output: "Unknown job".to_string(),
            waiting_confirmation: false,
        },
    }
}

#[tauri::command]
pub fn is_apt_package_installed(package: String) -> bool {
    Command::new("dpkg-query")
        .args(["-W", "-f=${Status}", &package])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("install ok installed"))
        .unwrap_or(false)
}

#[derive(Deserialize)]
pub struct PackageStatusQuery {
    apt: Vec<String>,
    flatpak: Vec<String>,
    paths: Vec<String>,
}

#[derive(Serialize)]
pub struct PackageStatusResult {
    apt: HashMap<String, bool>,
    flatpak: HashMap<String, bool>,
    paths: HashMap<String, bool>,
}

/// Installed-state of many packages in ONE call. Recommended used to fire one
/// IPC call and one subprocess per app (~100 concurrent dpkg-query /
/// flatpak info spawns, each also forcing a full re-render on the frontend),
/// which made the tab very slow to open; dpkg-query and flatpak list both take
/// every name at once (50 packages: 1.04 s one by one vs 0.023 s batched).
/// Same semantics as is_apt_package_installed / is_flatpak_installed /
/// check_path_exists: anything unknown, invalid or absent is simply false.
#[tauri::command]
pub fn get_package_statuses(query: PackageStatusQuery) -> PackageStatusResult {
    let mut apt: HashMap<String, bool> = query.apt.iter().map(|p| (p.clone(), false)).collect();
    let valid_apt: Vec<&String> = query.apt.iter().filter(|p| is_valid_pkg_name(p)).collect();
    if !valid_apt.is_empty() {
        // "--" so a name starting with '-' can never be read as an option.
        if let Ok(output) = Command::new("dpkg-query")
            .args(["-W", "-f=${Package} ${Status}\n", "--"])
            .args(&valid_apt)
            .output()
        {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if let Some((name, status)) = line.split_once(' ') {
                    if status.contains("install ok installed") {
                        apt.insert(name.to_string(), true);
                    }
                }
            }
        }
    }

    let mut flatpak: HashMap<String, bool> = query.flatpak.iter().map(|id| (id.clone(), false)).collect();
    if !query.flatpak.is_empty() {
        if let Ok(output) = Command::new("flatpak").args(["list", "--columns=application"]).output() {
            let installed: std::collections::HashSet<String> =
                String::from_utf8_lossy(&output.stdout).lines().map(|l| l.trim().to_string()).collect();
            for id in &query.flatpak {
                flatpak.insert(id.clone(), installed.contains(id));
            }
        }
    }

    let paths = query
        .paths
        .iter()
        .map(|p| (p.clone(), super::gaming::check_path_exists(p.clone())))
        .collect();

    PackageStatusResult { apt, flatpak, paths }
}

#[tauri::command]
pub fn install_apt_packages(packages: String) -> Result<u64, String> {
    if !is_valid_pkg_list(&packages) {
        return Err(format!("Invalid package name: {packages}"));
    }
    Ok(start_pkexec_job(format!("apt install -y {packages} && echo \"Installation complete.\"")))
}

#[tauri::command]
pub fn uninstall_apt_packages(packages: String) -> Result<u64, String> {
    if !is_valid_pkg_list(&packages) {
        return Err(format!("Invalid package name: {packages}"));
    }
    Ok(start_pkexec_job(format!("apt remove -y {packages} && echo \"Removal complete.\"")))
}

#[tauri::command]
pub fn is_snap_installed(name: String) -> bool {
    Command::new("snap")
        .args(["list", &name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn install_snap(name: String) -> Result<u64, String> {
    if !is_valid_pkg_name(&name) {
        return Err(format!("Invalid snap name: {name}"));
    }
    // snapd may not be present yet; installing it again is a harmless no-op.
    Ok(start_pkexec_job(format!("apt install -y snapd && snap install {name}")))
}

#[tauri::command]
pub fn uninstall_snap(name: String) -> Result<u64, String> {
    if !is_valid_pkg_name(&name) {
        return Err(format!("Invalid snap name: {name}"));
    }
    Ok(start_pkexec_job(format!("snap remove {name}")))
}

#[tauri::command]
pub fn is_flatpak_installed(app_id: String) -> bool {
    Command::new("flatpak")
        .args(["info", &app_id])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn install_flatpak(app_id: String) -> u64 {
    start_job("flatpak", vec!["install".to_string(), "-y".to_string(), "flathub".to_string(), app_id])
}

#[tauri::command]
pub fn uninstall_flatpak(app_id: String) -> u64 {
    start_job("flatpak", vec!["uninstall".to_string(), "-y".to_string(), app_id])
}

// Which scope (if any) an app is actually installed in — needed since a
// plain `flatpak info` without --user/--system can't tell you which one
// matched, and uninstalling from the wrong scope is a silent no-op.
#[tauri::command]
pub fn flatpak_app_scope(app_id: String) -> Option<String> {
    if Command::new("flatpak")
        .args(["info", "--user", &app_id])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("user".to_string());
    }
    if Command::new("flatpak")
        .args(["info", "--system", &app_id])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("system".to_string());
    }
    None
}

// Installs into whichever scope actually has Flathub — determined by the
// caller via flatpak_state() first. System-level installs write to
// /var/lib/flatpak (root-owned), so that one needs pkexec; user-level
// doesn't.
#[tauri::command]
pub fn install_flatpak_scoped(app_id: String, scope: String) -> Result<u64, String> {
    if !is_valid_flatpak_id(&app_id) {
        return Err(format!("Invalid Flatpak app id: {app_id}"));
    }
    Ok(if scope == "system" {
        start_pkexec_job(format!("flatpak install -y --system flathub {app_id}"))
    } else {
        start_job(
            "flatpak",
            vec!["install".to_string(), "-y".to_string(), "--user".to_string(), "flathub".to_string(), app_id],
        )
    })
}

#[tauri::command]
pub fn uninstall_flatpak_scoped(app_id: String, scope: String) -> Result<u64, String> {
    if !is_valid_flatpak_id(&app_id) {
        return Err(format!("Invalid Flatpak app id: {app_id}"));
    }
    Ok(if scope == "system" {
        start_pkexec_job(format!("flatpak uninstall -y --system {app_id}"))
    } else {
        start_job("flatpak", vec!["uninstall".to_string(), "-y".to_string(), "--user".to_string(), app_id])
    })
}

// Which scope(s) Flathub is actually reachable from: "user" (only the
// per-user remote — the default this app installs into), "system" (only
// system-wide — what Bazaar leaves behind, since it moves Flathub to the
// system level), "both" (caller should let the person pick), or "missing"
// (no flatpak / no Flathub anywhere).
#[tauri::command]
pub fn flatpak_state() -> String {
    if Command::new("flatpak").arg("--version").output().is_err() {
        return "missing".to_string();
    }
    let user = Command::new("flatpak")
        .args(["remotes", "--user"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
        .unwrap_or_default();
    let has_user = user.contains("flathub");

    let system = Command::new("flatpak")
        .args(["remotes", "--system"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
        .unwrap_or_default();
    let has_system = system.contains("flathub");

    match (has_user, has_system) {
        (true, true) => "both".to_string(),
        (true, false) => "user".to_string(),
        (false, true) => "system".to_string(),
        (false, false) => "missing".to_string(),
    }
}

#[tauri::command]
pub fn add_flathub_remote() -> u64 {
    start_job(
        "flatpak",
        vec![
            "remote-add".to_string(),
            "--if-not-exists".to_string(),
            "--user".to_string(),
            "flathub".to_string(),
            "https://dl.flathub.org/repo/flathub.flatpakrepo".to_string(),
        ],
    )
}

// System-wide Flathub writes to /var/lib/flatpak (root-owned), so this one
// needs pkexec — the --user one above doesn't.
#[tauri::command]
pub fn add_flathub_remote_system() -> u64 {
    start_pkexec_job(
        "flatpak remote-add --if-not-exists --system flathub https://dl.flathub.org/repo/flathub.flatpakrepo".to_string(),
    )
}

#[tauri::command]
pub fn remove_flathub_remote_user() -> u64 {
    start_job(
        "flatpak",
        vec!["remote-delete".to_string(), "--user".to_string(), "--force".to_string(), "flathub".to_string()],
    )
}

#[tauri::command]
pub fn remove_flathub_remote_system() -> u64 {
    start_pkexec_job("flatpak remote-delete --system --force flathub".to_string())
}

#[tauri::command]
pub fn update_repositories() -> u64 {
    start_pkexec_job("apt update".to_string())
}

#[tauri::command]
pub fn upgrade_system() -> u64 {
    // Root part: APT, Snap and system-wide Flatpak. User-installed Flatpaks
    // can only be updated as the user (root would touch a different
    // installation) — see update_flatpak_user.
    let script = r#"rc=0
echo "== APT =="
apt upgrade -y || rc=1
if command -v snap >/dev/null 2>&1; then
  echo "== Snap =="
  snap refresh || rc=1
fi
if command -v flatpak >/dev/null 2>&1; then
  echo "== Flatpak (system) =="
  flatpak update --system -y || rc=1
fi
exit $rc"#;
    start_pkexec_job(script.to_string())
}

/// Updates Flatpaks installed for the current user, without pkexec: run as
/// root, `flatpak update` only sees the system installation. `ids` = None
/// updates every user-installed Flatpak.
#[tauri::command]
pub fn update_flatpak_user(ids: Option<Vec<String>>) -> Result<u64, String> {
    let mut args = vec!["update".to_string(), "--user".to_string(), "-y".to_string()];
    if let Some(ids) = ids {
        if let Some(bad) = ids.iter().find(|id| !is_valid_flatpak_id(id)) {
            return Err(format!("Invalid Flatpak id: {bad}"));
        }
        args.extend(ids);
    }
    Ok(start_job("flatpak", args))
}

/// What the user ticked in the Update popup for the root-side session.
/// User-scope Flatpaks are not part of it (see update_flatpak_user).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeSelection {
    apt_all: bool,
    apt: Vec<String>,
    snap: Vec<String>,
    flatpak_system: Vec<String>,
}

// The follow-up command is built here from validated names, never taken
// verbatim from the frontend: it runs as root inside the pkexec session.
fn build_upgrade_command(sel: &UpgradeSelection) -> Result<String, String> {
    let mut steps: Vec<String> = Vec::new();

    if sel.apt_all {
        steps.push("apt upgrade -y".to_string());
    } else if !sel.apt.is_empty() {
        if let Some(bad) = sel.apt.iter().find(|p| !is_valid_pkg_name(p)) {
            return Err(format!("Invalid package name: {bad}"));
        }
        steps.push(format!("apt install --only-upgrade -y {}", sel.apt.join(" ")));
    }
    if !sel.snap.is_empty() {
        if let Some(bad) = sel.snap.iter().find(|p| !is_valid_pkg_name(p)) {
            return Err(format!("Invalid snap name: {bad}"));
        }
        steps.push(format!("snap refresh {}", sel.snap.join(" ")));
    }
    if !sel.flatpak_system.is_empty() {
        if let Some(bad) = sel.flatpak_system.iter().find(|id| !is_valid_flatpak_id(id)) {
            return Err(format!("Invalid Flatpak id: {bad}"));
        }
        steps.push(format!("flatpak update --system -y {}", sel.flatpak_system.join(" ")));
    }

    if steps.is_empty() {
        return Ok(String::new());
    }
    // Every step runs even if an earlier one fails; the session's exit
    // status is a failure if any of them failed.
    let body: Vec<String> = steps.iter().map(|s| format!("{s} || rc=1")).collect();
    Ok(format!("rc=0; {}; exit $rc", body.join("; ")))
}

/// Update as a single pkexec authentication: runs `apt update`, then pauses
/// (see start_confirmable_pkexec_job) until the frontend has shown the "N
/// updates available" popup and the user picks which packages to upgrade —
/// only then does it run `apt install --only-upgrade -y <picked packages>`,
/// without asking for the password again.
#[tauri::command]
pub fn start_update_session() -> u64 {
    start_confirmable_pkexec_job("apt update".to_string())
}

/// Everything that has an update pending, by package manager. Update and
/// Upgrade both cover APT, Flatpak and Snap (each only if present).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradableSet {
    apt: Vec<String>,
    flatpak_user: Vec<String>,
    flatpak_system: Vec<String>,
    snap: Vec<String>,
}

// Pending Flatpak updates for one installation ("--user" or "--system").
// Locale/Debug/Sources extensions are hidden: they update together with the
// app or runtime they belong to.
fn flatpak_updates(scope: &str) -> Vec<String> {
    let output = match Command::new("flatpak")
        .args(["remote-ls", scope, "--updates", "--columns=application"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    let mut ids: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|id| {
            is_valid_flatpak_id(id) && !id.ends_with(".Locale") && !id.ends_with(".Debug") && !id.ends_with(".Sources")
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

// LC_ALL=C so the "Name ..." header of `snap refresh --list` is not localized.
fn snap_updates() -> Vec<String> {
    let output = match Command::new("snap").env("LC_ALL", "C").args(["refresh", "--list"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|name| *name != "Name" && is_valid_pkg_name(name))
        .map(str::to_string)
        .collect()
}

#[tauri::command]
pub fn check_upgradable() -> Result<UpgradableSet, String> {
    let output = Command::new("apt")
        .args(["list", "--upgradable"])
        .output()
        .map_err(|e| e.to_string())?;

    let text = String::from_utf8_lossy(&output.stdout);
    let apt = text
        .lines()
        .filter(|line| line.contains('/') && !line.starts_with("Listing"))
        .filter_map(|line| line.split('/').next().map(str::to_string))
        .collect();

    // remote-ls and snap refresh --list query the network, so run them side
    // by side instead of one after another.
    let (flatpak_user, flatpak_system, snap) = std::thread::scope(|s| {
        let user = s.spawn(|| flatpak_updates("--user"));
        let system = s.spawn(|| flatpak_updates("--system"));
        let snap = s.spawn(snap_updates);
        (
            user.join().unwrap_or_default(),
            system.join().unwrap_or_default(),
            snap.join().unwrap_or_default(),
        )
    });

    Ok(UpgradableSet { apt, flatpak_user, flatpak_system, snap })
}

#[tauri::command]
pub fn clean_system() -> u64 {
    // Flatpak and Snap only if present — ported from 2.x's Plasma/Gnome tabs
    // (which already did the Flatpak step; Xfce's was missing it, an
    // inconsistency fixed here rather than carried over). Snap keeps disabled
    // old revisions of every installed snap around for rollback by default;
    // removing them is Canonical's own documented cleanup snippet.
    //
    // The Flatpak step does NOT just run `flatpak uninstall --unused -y`
    // directly (2.x did, and so did this function until a real user report:
    // it silently took out VAAPI/NVIDIA GL runtimes that OBS Studio needed
    // for hardware encoding, since Flatpak's own dependency resolver can't
    // trace an app's use of an *extension* the way it traces a runtime).
    // soplos-sys-cleaner already solved this the same way: ask
    // `flatpak uninstall --unused` (no -y) what it WOULD remove, answer "n"
    // so nothing is actually deleted, then only uninstall-for-real the refs
    // that aren't one of these dynamically-loaded extension kinds.
    let script = r#"echo "== APT =="
apt autoremove -y && apt clean
if command -v flatpak >/dev/null 2>&1; then
  echo "== Flatpak =="
  UNUSED_REFS=$(echo n | flatpak uninstall --unused 2>&1 | awk '
    /^[[:space:]]*[0-9]+\.[[:space:]]/ {
      id=$2
      if (id !~ /\.GL\.|\.GL32\.|\.VulkanLayer\.|\.ffmpeg|\.VAAPI\.|\.openh264|\.Compat32\./) print id
    }
  ')
  if [ -n "$UNUSED_REFS" ]; then
    flatpak uninstall -y $UNUSED_REFS
  else
    echo "No unused Flatpak runtimes to remove."
  fi
fi
if command -v snap >/dev/null 2>&1; then
  echo "== Snap =="
  snap list --all | awk '/disabled/{print $1, $3}' | while read -r snapname revision; do
    snap remove "$snapname" --revision="$revision"
  done
fi"#;
    start_pkexec_job(script.to_string())
}

#[tauri::command]
pub fn integrate_snap_whisker_menu() -> u64 {
    start_pkexec_job(
        "apt install -y snapd && \
         ln -sf /var/lib/snapd/desktop/applications /usr/share/applications/snap && \
         update-desktop-database"
            .to_string(),
    )
}

#[tauri::command]
pub fn open_repo_selector() -> Result<(), String> {
    Command::new("soplos-repo-selector")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
