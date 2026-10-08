// DaVinci Resolve install flow — ported from recommended_tab.py's
// _install_davinci_resolve and its step-by-step helpers. Blackmagic doesn't
// let redistribute the installer, so the user has to download the ZIP/RUN
// themselves; this converts it to a .deb with Daniel Tufvesson's
// makeresolvedeb (bundled as a Tauri resource) and installs it, then offers
// the two optional hardware patches.

use tauri_plugin_dialog::DialogExt;

use super::software::{start_job, start_pkexec_job};

fn work_dir_for(installer_path: &str) -> String {
    let dir = std::path::Path::new(installer_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    format!("{dir}/soplos-davinci-work")
}

// Non-async Tauri commands run on the main GTK thread, so a blocking native
// file dialog there freezes the whole UI until it's closed (same class of
// bug fixed in WebApp Manager's pick_icon_file / Welcome's
// pick_vmware_installer). spawn_blocking moves the wait off the main thread.
#[tauri::command]
pub async fn pick_davinci_installer(app: tauri::AppHandle) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Select DaVinci Resolve Installer")
            .add_filter("DaVinci Resolve Installer", &["zip", "run"])
            .blocking_pick_file()
            .and_then(|fp| fp.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .unwrap_or(None)
}

#[tauri::command]
pub fn install_davinci_deps() -> u64 {
    start_pkexec_job("apt-get install -y fakeroot xorriso unzip".to_string())
}

// Extracts the installer into a work directory next to it, copies the
// bundled makeresolvedeb script in, and runs it — as the current user, NOT
// root, exactly like the 2.x tab: makeresolvedeb refuses to run as root.
#[tauri::command]
pub fn davinci_extract_and_convert(app: tauri::AppHandle, installer_path: String) -> Result<u64, String> {
    use tauri::Manager;

    let makeresolvedeb = app
        .path()
        .resolve(
            "resources/davinci/makeresolvedeb_1.10.1_multi.sh",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| e.to_string())?;
    let makeresolvedeb = makeresolvedeb.to_string_lossy().into_owned();

    let work_dir = work_dir_for(&installer_path);
    let is_zip = installer_path.to_lowercase().ends_with(".zip");

    let extract_step = if is_zip {
        format!(r#"unzip -o "{installer_path}" -d "{work_dir}""#)
    } else {
        format!(r#"cp "{installer_path}" "{work_dir}/""#)
    };

    let script = format!(
        r#"set -e
rm -rf "{work_dir}"
mkdir -p "{work_dir}"
{extract_step}
RUN_FILE=$(find "{work_dir}" -maxdepth 1 -name '*.run' | head -1)
if [ -z "$RUN_FILE" ]; then
    echo "No .run installer found inside the downloaded file."
    exit 1
fi
cp "{makeresolvedeb}" "{work_dir}/makeresolvedeb.sh"
chmod +x "{work_dir}/makeresolvedeb.sh" "$RUN_FILE"
cd "{work_dir}"
./makeresolvedeb.sh "$(basename "$RUN_FILE")"
"#
    );

    Ok(start_job("bash", vec!["-c".to_string(), script]))
}

#[tauri::command]
pub fn install_davinci_debs(installer_path: String) -> u64 {
    let work_dir = work_dir_for(&installer_path);
    let script = format!(
        r#"set -e
DEBS=$(find "{work_dir}" -maxdepth 1 -name '*.deb' | sort)
if [ -z "$DEBS" ]; then
    echo "makeresolvedeb did not produce any .deb package."
    exit 1
fi
apt-get install -y $DEBS
"#
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn davinci_cleanup(installer_path: String) -> Result<(), String> {
    std::fs::remove_dir_all(work_dir_for(&installer_path)).or(Ok(()))
}

const DAVINCI_VIRTUAL_MIC_SCRIPT: &str = r#"set -e
CONF_DIR="/etc/pipewire/pipewire-pulse.conf.d"
CONF_FILE="$CONF_DIR/virtual-mic.conf"

if arecord -l 2>/dev/null | grep -q "card"; then
    echo "Capture device detected. Patch not needed."
    exit 0
fi

mkdir -p "$CONF_DIR"
cat > "$CONF_FILE" << 'EOF'
pulse.cmd = [
    { cmd = "load-module" args = "module-null-sink sink_name=virtmic sink_properties=device.description=VirtualMic" flags = [] }
    { cmd = "load-module" args = "module-virtual-source source_name=virtmic master=virtmic.monitor source_properties=device.description=VirtualMic" flags = [] }
]
EOF
echo "$CONF_FILE installed."
"#;

#[tauri::command]
pub fn run_davinci_mic_patch() -> u64 {
    start_pkexec_job(DAVINCI_VIRTUAL_MIC_SCRIPT.to_string())
}

const DAVINCI_GPU_PATCH_SCRIPT: &str = r##"set -e
if [ -n "$PKEXEC_UID" ]; then
    REAL_USER=$(getent passwd "$PKEXEC_UID" | cut -d: -f1)
    REAL_HOME=$(getent passwd "$PKEXEC_UID" | cut -d: -f6)
else
    REAL_USER="$USER"
    REAL_HOME="$HOME"
fi

detect_gpu() {
    if lspci 2>/dev/null | grep -qi "VGA.*Intel\|Display.*Intel\|3D.*Intel"; then
        echo "intel"
    elif lspci 2>/dev/null | grep -qi "VGA.*AMD\|Display.*AMD\|3D.*AMD\|VGA.*ATI"; then
        echo "amd"
    else
        echo "unknown"
    fi
}

GPU_VENDOR="$(detect_gpu)"
echo "Detected vendor: $GPU_VENDOR"

if [ "$GPU_VENDOR" = "unknown" ]; then
    echo "Could not detect an integrated AMD or Intel GPU automatically — skipping."
    exit 1
fi

echo "Step 1: Installing dependencies for $GPU_VENDOR..."
apt-get update
if [ "$GPU_VENDOR" = "amd" ]; then
    apt-get install -y mesa-opencl-icd gcc ocl-icd-opencl-dev
elif [ "$GPU_VENDOR" = "intel" ]; then
    apt-get install -y mesa-opencl-icd intel-opencl-icd intel-media-va-driver-non-free libva-drm2 gcc ocl-icd-opencl-dev
fi

if [ "$GPU_VENDOR" = "intel" ]; then
    echo "Step 2: Checking i915 firmware..."
    FIRMWARE_BASE="https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/plain/i915"
    FIRMWARE_DIR="/lib/firmware/i915"
    NEED_INITRAMFS=0
    mkdir -p "$FIRMWARE_DIR"
    for FW in adlp_dmc.bin tgl_guc_70.bin tgl_huc.bin; do
        if [ ! -f "$FIRMWARE_DIR/$FW" ]; then
            curl -L "$FIRMWARE_BASE/$FW" -o "$FIRMWARE_DIR/$FW"
            NEED_INITRAMFS=1
        fi
    done
    if [ "$NEED_INITRAMFS" = "1" ]; then
        update-initramfs -u -k all
        REBOOT_NEEDED=1
    fi
fi

if [ -f "/opt/resolve/libs/libProResRAW.so" ]; then
    echo "Step 3: Building and injecting dummy ProResRAW library..."
    cat << 'CEOF' > /tmp/dummy_prores.c
void* PRRawCreateBooleanSettingPropertyValue()      { return 0; }
void* PRRawCreateClipSettingsContext()              { return 0; }
void* PRRawCreateClipSettingsRawConversionParams()  { return 0; }
void* PRRawCreateCUDAProcessor()                    { return 0; }
void* PRRawCreateDecoder()                          { return 0; }
void* PRRawCreateFloat32SettingPropertyValue()      { return 0; }
void* PRRawCreateInt32SettingPropertyValue()        { return 0; }
void* PRRawCreateOpenCLProcessorForQueue()          { return 0; }
void* PRRawCreateRawImageMetadata()                 { return 0; }
void* PRRawCreateUInt32SettingPropertyValue()       { return 0; }
void* PRRawCreateUTF8StringSettingPropertyValue()   { return 0; }
void* PRRawCUDAProcessFrame_Texture()               { return 0; }
void* PRRawDecodeFrame()                            { return 0; }
void* PRRawDestroyClipSettingsContext()             { return 0; }
void* PRRawDestroyClipSettingsRawConversionParams() { return 0; }
void* PRRawDestroyCUDAProcessor()                   { return 0; }
void* PRRawDestroyDecoder()                         { return 0; }
void* PRRawDestroyOpenCLProcessor()                 { return 0; }
void* PRRawDestroyRawImageMetadata()                { return 0; }
void* PRRawDestroySettingPropertyValue()            { return 0; }
void* PRRawGetArraySettingPropertyValue()           { return 0; }
void* PRRawGetArraySettingPropertyValueSize()       { return 0; }
void* PRRawGetBooleanSettingPropertyValue()         { return 0; }
void* PRRawGetFloat32SettingPropertyValue()         { return 0; }
void* PRRawGetFrameHeaderMetadata()                 { return 0; }
void* PRRawGetHeight()                              { return 0; }
void* PRRawGetInt32SettingPropertyValue()           { return 0; }
void* PRRawGetOutputColorSpecifier()                { return 0; }
void* PRRawGetRawConversionPlugInIdentifier()       { return 0; }
void* PRRawGetRawConversionPlugInLoadingErrorInfo() { return 0; }
void* PRRawGetRawConversionPlugInName()             { return 0; }
void* PRRawGetRawConversionPlugIns()                { return 0; }
void* PRRawGetRawConversionPlugInVersionMajor()     { return 0; }
void* PRRawGetRawConversionPlugInVersionMinor()     { return 0; }
void* PRRawGetSettingPropertyValue()                { return 0; }
void* PRRawGetSettingPropertyValueType()            { return 0; }
void* PRRawGetSupportedSettings()                   { return 0; }
void* PRRawGetUInt32SettingPropertyValue()          { return 0; }
void* PRRawGetUTF8StringSettingPropertyValue()      { return 0; }
void* PRRawGetUTF8StringSettingPropertyValueLength(){ return 0; }
void* PRRawGetWidth()                               { return 0; }
void* PRRawImageBytesPerRowNeeded()                 { return 0; }
void* PRRawImageMetadataGetBytesPerRow()            { return 0; }
void* PRRawImageMetadataGetHeight()                 { return 0; }
void* PRRawImageMetadataGetWidth()                  { return 0; }
void* PRRawIsRawConversionPlugInCompatibleWithClip(){ return 0; }
void* PRRawMetadataDictionaryAddBinaryDataItem()    { return 0; }
void* PRRawMetadataDictionaryAddFloat32Item()       { return 0; }
void* PRRawMetadataDictionaryAddUInt32Item()        { return 0; }
void* PRRawMetadataDictionaryAddUTF8StringItem()    { return 0; }
void* PRRawMetadataDictionaryCreate()               { return 0; }
void* PRRawMetadataDictionaryDestroy()              { return 0; }
void* PRRawOpenCLProcessFrame_Texture()             { return 0; }
void* PRRawSetSettingCurrentValue()                 { return 0; }
CEOF
    gcc -shared -fPIC /tmp/dummy_prores.c -o /tmp/libProResRAW.so
    if [ ! -f "/opt/resolve/libs/libProResRAW.so.bak" ]; then
        mv /opt/resolve/libs/libProResRAW.so /opt/resolve/libs/libProResRAW.so.bak
    fi
    cp /tmp/libProResRAW.so /opt/resolve/libs/libProResRAW.so
    rm -f /tmp/dummy_prores.c /tmp/libProResRAW.so
fi

echo "Step 4: Patching .desktop launcher..."
DESKTOP_SYS="/usr/share/applications/com.blackmagicdesign.resolve.desktop"
DESKTOP_LOCAL="$REAL_HOME/.local/share/applications/com.blackmagicdesign.resolve.desktop"
LIBSTDC=$(find /usr/lib -name "libstdc++.so.6" -print -quit 2>/dev/null)
if [ -z "$LIBSTDC" ]; then
    LIBSTDC=$(ldconfig -p 2>/dev/null | grep "libstdc++.so.6" | awk '{print $NF}' | head -1)
fi
if [ -z "$LIBSTDC" ]; then
    echo "libstdc++.so.6 not found. Install libstdc++6."
    exit 1
fi

if [ "$GPU_VENDOR" = "amd" ]; then
    ENV_VARS="RUSTICL_ENABLE=radeonsi OCL_ICD_VENDORS=rusticl.icd"
else
    ENV_VARS="OCL_ICD_VENDORS=intel.icd"
fi

sudo -u "$REAL_USER" mkdir -p "$REAL_HOME/.local/share/applications/"
if [ -f "$DESKTOP_SYS" ]; then
    cp "$DESKTOP_SYS" "$DESKTOP_LOCAL"
    chown "$REAL_USER":"$REAL_USER" "$DESKTOP_LOCAL"
    sed -i "s|^Exec=.*|Exec=env $ENV_VARS LD_PRELOAD=$LIBSTDC /opt/resolve/bin/resolve|g" "$DESKTOP_LOCAL"
    update-desktop-database "$REAL_HOME/.local/share/applications/" 2>/dev/null || true
    echo "Launcher patched: $ENV_VARS LD_PRELOAD=$LIBSTDC"
else
    echo "Launcher not found at $DESKTOP_SYS — launch manually with:"
    echo "env $ENV_VARS LD_PRELOAD=$LIBSTDC /opt/resolve/bin/resolve"
fi

echo "Step 5: Checking /dev/dri permissions..."
if id -nG "$REAL_USER" | grep -qw "render"; then
    echo "User already in 'render' group."
else
    usermod -aG render,video "$REAL_USER"
    echo "Added $REAL_USER to render and video groups. Log out to apply."
    REBOOT_NEEDED=1
fi

echo "Patch applied ($GPU_VENDOR)."
if [ "${REBOOT_NEEDED:-0}" = "1" ]; then
    echo "*** REBOOT REQUIRED for firmware/group changes ***"
fi
"##;

#[tauri::command]
pub fn run_davinci_gpu_patch() -> u64 {
    start_pkexec_job(DAVINCI_GPU_PATCH_SCRIPT.to_string())
}

// makeresolvedeb produces a real apt package (named after whatever the
// downloaded installer identifies itself as, e.g. "resolve" for the free
// edition or "resolve-studio" for the paid one) — not matched against a
// single hardcoded name here, since either can be installed depending on
// what the user downloaded. Found dynamically instead of guessing.
#[tauri::command]
pub fn uninstall_davinci() -> u64 {
    start_pkexec_job(
        r#"set -e
PKGS=$(dpkg-query -W -f='${Package}\n' 2>/dev/null | grep -i '^resolve' || true)
if [ -z "$PKGS" ]; then
    echo "No DaVinci Resolve package found installed via apt."
    exit 1
fi
apt-get remove -y $PKGS
echo "DaVinci Resolve uninstalled.""#
            .to_string(),
    )
}
