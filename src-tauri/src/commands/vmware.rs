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

// VMware ships its own uninstaller (vmware-installer --uninstall-product),
// which interactively asks twice whether to keep configuration files —
// `yes no` auto-answers "no" (full removal) to both so it doesn't hang
// waiting on a TTY that doesn't exist under pkexec. Runs without `set -e`:
// a non-zero exit here (e.g. the product was already partially removed by
// hand) shouldn't skip removing soplos-vmware-modules below.
#[tauri::command]
pub fn uninstall_vmware() -> u64 {
    start_pkexec_job(
        r#"yes no | vmware-installer --uninstall-product vmware-workstation --console || true
apt-get remove -y soplos-vmware-modules || true
echo "VMware Workstation Pro uninstalled.""#
            .to_string(),
    )
}
