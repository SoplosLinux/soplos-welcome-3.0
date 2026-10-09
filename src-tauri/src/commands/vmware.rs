// VMware Workstation Pro install flow. Broadcom requires a login to
// download the Linux .bundle, so like DaVinci Resolve above, the user
// fetches it themselves and picks the file here — Welcome can't download it
// on its own. After the official installer runs, this also installs
// soplos-vmware-modules: a Soplos-built package (validated by the team and
// the community) that compiles and keeps the vmmon/vmnet kernel modules
// working across kernel updates, since VMware's own vmware-modconfig step
// is known to fail on systemd-based distros without it.

use tauri_plugin_dialog::DialogExt;

use super::software::start_pkexec_job;

// Non-async Tauri commands run on the main GTK thread, so a blocking native
// file dialog there freezes the whole UI until it's closed (same class of
// bug fixed in WebApp Manager's pick_icon_file). spawn_blocking moves the
// wait off the main thread.
#[tauri::command]
pub async fn pick_vmware_installer(app: tauri::AppHandle) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Select the VMware Workstation Pro bundle")
            .add_filter("VMware Workstation Pro Installer", &["bundle"])
            .blocking_pick_file()
            .and_then(|fp| fp.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .unwrap_or(None)
}

// --console: run the text installer instead of the GUI wizard.
// --required: install only the required components, skipping the optional
// ones the GUI would otherwise ask about one by one.
// --eulas-agreed: accept the EULA without an interactive prompt.
// All three are documented by VMware/Broadcom for exactly this kind of
// unattended install.
#[tauri::command]
pub fn install_vmware_bundle(installer_path: String) -> u64 {
    start_pkexec_job(format!(
        r#"set -e
chmod +x "{installer_path}"
"{installer_path}" --console --required --eulas-agreed
echo "VMware Workstation Pro installed.""#
    ))
}

#[tauri::command]
pub fn install_vmware_modules(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;

    let deb_path = app
        .path()
        .resolve(
            "resources/vmware/soplos-vmware-modules_1.0.0_amd64.deb",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| e.to_string())?;
    let deb_path = deb_path.to_string_lossy().into_owned();

    Ok(start_pkexec_job(format!(
        r#"set -e
apt-get install -y "{deb_path}"
echo "soplos-vmware-modules installed.""#
    )))
}

const UNINSTALL_SCRIPT: &str = r#"pkexec sh -c 'vmware-installer --uninstall-product vmware-workstation --console; apt-get remove -y soplos-vmware-modules'
echo
read -p "Press Enter to close this window..." _"#;

// VMware ships its own uninstaller (vmware-installer --uninstall-product),
// documented as asking whether to keep configuration files. Confirmed live
// (screenshot of a real run): the prompt text reaches Welcome's read-only
// output panel fine, but nothing short of a real answer unblocks it — and
// that panel has no way to send one back. Piping a guessed answer in
// (`yes no | ...`) never worked either, most likely because the prompt
// reads from /dev/tty directly rather than stdin.
// Rather than keep guessing at vmware-installer's exact interactive
// plumbing, this opens a real terminal running the uninstall under pkexec,
// so whatever it asks is something the user can actually read and answer
// themselves — same as running it by hand. That also means this one flow
// bypasses Welcome's own job/output system entirely (no runTracked/
// poll_job on the frontend): there's nothing to poll, this command's only
// job is opening that window.
// Ghostty ships by default on all three Soplos desktops, but a user can
// still remove it, so it's tried first and `x-terminal-emulator` — the
// Debian alternatives symlink that always points at whichever terminal is
// actually configured as default, if any is installed — is the fallback,
// rather than failing outright just because the one we assume is there
// happens not to be. Only if neither exists does this give up.
// The frontend has no way to know when the user is done in that window —
// there's no job id, nothing to poll. So this spawns a plain OS thread that
// waits on the terminal's child process (it only exits once the "Press
// Enter to close" line is cleared) and emits a `vmware-uninstall-finished`
// event once it does, so Recommended.svelte can refresh VMware's installed
// status at the right time instead of right after the window merely opens.
#[tauri::command]
pub fn uninstall_vmware(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;

    let args = ["-e", "sh", "-c", UNINSTALL_SCRIPT];
    let child = match std::process::Command::new("ghostty").args(args).spawn() {
        Ok(child) => child,
        Err(ghostty_err) => std::process::Command::new("x-terminal-emulator")
            .args(args)
            .spawn()
            .map_err(|fallback_err| format!("ghostty: {ghostty_err}; x-terminal-emulator: {fallback_err}"))?,
    };

    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
        let _ = app.emit("vmware-uninstall-finished", ());
    });

    Ok(())
}
