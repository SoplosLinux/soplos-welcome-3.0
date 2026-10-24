// What a Mac needs. The hardware scan recognises an Apple computer from its
// DMI data (hardware_scan.rs) and offers three things, each one tested on
// MacBooks by a user of the community: the fan daemon mbpfan, the battery
// manager TLP (both in Debian) and the driver of the FaceTime HD camera,
// which is not in Debian.
//
// The camera driver (facetimehd) is a community project that its authors call
// experimental. The sources of both of its repositories are fetched at the
// commit named below, never at a branch, so the script that runs as root is
// always the code that was reviewed; the firmware project's own script then
// downloads the camera firmware from Apple's update servers and checks every
// file against pinned checksums. A local package, facetimehd-dkms, is built
// from the sources so dpkg owns the module (it shows as installed and apt
// can remove it) and DKMS rebuilds it for every new kernel.

use super::software::start_pkexec_job;

const FACETIMEHD_COMMIT: &str = "6dc8cd85e87993ff21cb01eb333ff42afb058551";
const FACETIMEHD_FIRMWARE_COMMIT: &str = "60ee21228d9ca00a7bd84fdaefaff00a81f1db91";

const FACETIMEHD_SCRIPT: &str = r##"set -e
export DEBIAN_FRONTEND=noninteractive
echo "=== FaceTime HD camera (facetimehd, community driver) ==="
apt-get install -y dkms build-essential git curl xz-utils ca-certificates
apt-get install -y linux-headers-$(uname -r) || true
if [ ! -e /lib/modules/$(uname -r)/build ]; then
    echo "Kernel build tree missing or broken, reinstalling headers..."
    apt-get install -y --reinstall linux-headers-$(uname -r) || true
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

fetch_commit() {
    mkdir -p "$WORK/$1"
    git -C "$WORK/$1" init -q
    git -C "$WORK/$1" fetch -q --depth 1 "$2" "$3"
    git -C "$WORK/$1" checkout -q FETCH_HEAD
    if [ "$(git -C "$WORK/$1" rev-parse HEAD)" != "$3" ]; then
        echo "ERROR: $1 is not the commit that was expected."
        exit 1
    fi
}

echo "Downloading the driver sources..."
fetch_commit driver https://github.com/patjak/facetimehd @FACETIMEHD_COMMIT@
fetch_commit firmware https://github.com/patjak/facetimehd-firmware @FIRMWARE_COMMIT@

echo "Downloading the camera firmware from Apple and checking it..."
bash "$WORK/firmware/facetimehd-firmware-install.sh"
if [ ! -s /usr/lib/firmware/facetimehd/firmware.bin ] && [ ! -s /lib/firmware/facetimehd/firmware.bin ]; then
    echo "ERROR: the camera firmware was not installed."
    exit 1
fi

VERSION=$(sed -n 's/^PACKAGE_VERSION=//p' "$WORK/driver/dkms.conf" | tr -d '"')
if [ -z "$VERSION" ]; then
    echo "ERROR: could not read the driver version."
    exit 1
fi
PKG="$WORK/pkg"
SRC="$PKG/usr/src/facetimehd-$VERSION"
mkdir -p "$PKG/DEBIAN" "$SRC" "$PKG/usr/lib/modprobe.d"
cp -a "$WORK/driver/." "$SRC/"
rm -rf "$SRC/.git"
printf 'blacklist bdc_pci\n' > "$PKG/usr/lib/modprobe.d/facetimehd.conf"

cat > "$PKG/DEBIAN/control" <<CONTROL
Package: facetimehd-dkms
Version: $VERSION+git@SHORT_COMMIT@
Section: kernel
Priority: optional
Architecture: all
Depends: dkms
Description: FaceTime HD (Broadcom 1570) camera driver of the MacBook, DKMS source
 Community driver of the PCIe webcam of recent MacBooks, built by Soplos
 Welcome from https://github.com/patjak/facetimehd. The firmware of the camera
 is downloaded from Apple at install time and removed with this package.
CONTROL

cat > "$PKG/DEBIAN/postinst" <<POSTINST
#!/bin/sh
set -e
if [ "\$1" = configure ]; then
    dkms add -m facetimehd -v $VERSION > /dev/null 2>&1 || true
fi
POSTINST
cat > "$PKG/DEBIAN/prerm" <<PRERM
#!/bin/sh
set -e
if [ "\$1" = remove ]; then
    dkms remove -m facetimehd -v $VERSION --all > /dev/null 2>&1 || true
fi
PRERM
cat > "$PKG/DEBIAN/postrm" <<POSTRM
#!/bin/sh
set -e
if [ "\$1" = remove ] || [ "\$1" = purge ]; then
    rm -rf /usr/lib/firmware/facetimehd /lib/firmware/facetimehd
fi
POSTRM
chmod 755 "$PKG/DEBIAN/postinst" "$PKG/DEBIAN/prerm" "$PKG/DEBIAN/postrm"

dpkg-deb --root-owner-group --build "$PKG" "$WORK/facetimehd-dkms.deb"
apt-get install -y --no-remove "$WORK/facetimehd-dkms.deb"

echo "Building the module for the running kernel..."
if ! dkms install -m facetimehd -v "$VERSION" -k "$(uname -r)"; then
    echo "ERROR: the module did not build for $(uname -r)."
    apt-get remove -y facetimehd-dkms || true
    exit 1
fi
modprobe -r bdc_pci 2> /dev/null || true
modprobe facetimehd || echo "The module could not be loaded now; restart the computer."
if command -v mokutil > /dev/null 2>&1 && mokutil --sb-state 2> /dev/null | grep -qi "SecureBoot enabled"; then
    echo "Secure Boot is on: the module is signed with the DKMS key, which has to be enrolled for the camera to load."
fi
echo "FaceTime HD camera driver installed."
"##;

fn facetimehd_script() -> String {
    FACETIMEHD_SCRIPT
        .replace("@FACETIMEHD_COMMIT@", FACETIMEHD_COMMIT)
        .replace("@FIRMWARE_COMMIT@", FACETIMEHD_FIRMWARE_COMMIT)
        .replace("@SHORT_COMMIT@", &FACETIMEHD_COMMIT[..7])
}

#[tauri::command]
pub fn install_facetimehd() -> u64 {
    start_pkexec_job(facetimehd_script())
}

#[tauri::command]
pub fn uninstall_facetimehd() -> u64 {
    start_pkexec_job("apt remove -y facetimehd-dkms && echo \"FaceTime HD camera driver removed.\"".to_string())
}

// TLP conflicts with power-profiles-daemon, which GNOME and Plasma bring:
// --no-remove makes apt stop instead of removing it behind the user's back.
#[tauri::command]
pub fn install_tlp() -> u64 {
    start_pkexec_job(
        "if apt-get install -y --no-remove tlp tlp-rdw; then echo \"TLP installed.\"; else echo \"TLP was not installed: it conflicts with power-profiles-daemon, which this desktop uses. Remove that package first if you want TLP.\"; exit 1; fi"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn the_camera_script_is_valid_shell_and_pinned() {
        let script = facetimehd_script();
        let file = std::env::temp_dir().join("welcome-facetimehd.sh");
        std::fs::write(&file, &script).unwrap();
        let out = Command::new("bash").arg("-n").arg(&file).output().unwrap();
        let _ = std::fs::remove_file(&file);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(script.contains(FACETIMEHD_COMMIT) && script.contains(FACETIMEHD_FIRMWARE_COMMIT));
        assert!(!script.contains('@'));
    }
}
