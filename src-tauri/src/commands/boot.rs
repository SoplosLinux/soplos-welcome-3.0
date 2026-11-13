// The boot loader of this system, for the scripts that change the kernel
// command line or the boot entries (NVIDIA, PRIME, kernels). Soplos Linux
// Installer offers GRUB, Limine and systemd-boot, and purges GRUB when
// another one is chosen, so only one is ever installed:
//   - Limine: the settings limine-soplos saves at install time;
//   - systemd-boot: its package installed and its folder on the ESP;
//   - GRUB otherwise, as every Soplos installed by Calamares.
// GRUB reads its command line from /etc/default/grub (update-grub writes the
// menu); Limine and systemd-boot from /etc/kernel/cmdline (limine-update
// writes limine.conf; kernel-install the entry of each kernel, as Debian's
// own post-update hook does it).
//
// The functions are bash, put at the top of the pkexec scripts.

pub(crate) const BOOT_FUNCTIONS: &str = r#"
soplos_bootloader() {
    if [ -f /etc/limine-soplos/install.conf ]; then
        echo limine
        return
    fi
    if dpkg-query -W -f='${Status}' systemd-boot 2>/dev/null | grep -q "install ok installed"; then
        for esp in /boot/efi /efi /boot; do
            if [ -d "$esp/loader" ]; then
                echo systemd-boot
                return
            fi
        done
    fi
    echo grub
}

# /etc/kernel/cmdline, created from the running command line when missing.
soplos_kernel_cmdline() {
    if [ ! -s /etc/kernel/cmdline ]; then
        mkdir -p /etc/kernel
        sed -e 's/BOOT_IMAGE=[^ ]* *//' -e 's/initrd=[^ ]* *//g' /proc/cmdline > /etc/kernel/cmdline
    fi
}

# Adds a kernel parameter where the boot loader reads it.
soplos_cmdline_add() {
    local param="$1"
    case "$(soplos_bootloader)" in
        grub)
            if ! grep -qF -- "$param" /etc/default/grub; then
                sed -i "s|GRUB_CMDLINE_LINUX_DEFAULT=\"\([^\"]*\)\"|GRUB_CMDLINE_LINUX_DEFAULT=\"\1 $param\"|" /etc/default/grub
            fi
            ;;
        *)
            soplos_kernel_cmdline
            if ! grep -qF -- "$param" /etc/kernel/cmdline; then
                sed -i "s|\$| $param|" /etc/kernel/cmdline
            fi
            ;;
    esac
}

# Takes a kernel parameter out of where the boot loader reads it.
soplos_cmdline_remove() {
    local param="$1"
    local file=/etc/default/grub
    [ "$(soplos_bootloader)" = grub ] || file=/etc/kernel/cmdline
    [ -f "$file" ] && sed -i "s| $param||g" "$file"
    return 0
}

# Writes the boot menu again with the kernels of /boot.
soplos_boot_update() {
    case "$(soplos_bootloader)" in
        limine)
            echo "Updating the Limine menu..."
            limine-update
            ;;
        systemd-boot)
            echo "Updating the systemd-boot entries..."
            for image in /boot/vmlinuz-*; do
                [ -e "$image" ] || continue
                version="${image#/boot/vmlinuz-}"
                if [ -f "/boot/initrd.img-$version" ]; then
                    kernel-install add "$version" "$image" "/boot/initrd.img-$version"
                else
                    kernel-install add "$version" "$image"
                fi
            done
            ;;
        *)
            echo "Updating GRUB..."
            if [ -x /usr/sbin/update-grub ]; then
                /usr/sbin/update-grub
            else
                update-grub
            fi
            ;;
    esac
}
"#;

/// The same detection as soplos_bootloader, for the window: "grub", "limine"
/// or "systemd-boot". The GRUB Editor card is greyed out with the other two.
#[tauri::command]
pub fn get_bootloader() -> String {
    use std::path::Path;
    if Path::new("/etc/limine-soplos/install.conf").exists() {
        return "limine".into();
    }
    let status = std::fs::read_to_string("/var/lib/dpkg/status").unwrap_or_default();
    let systemd_boot = status
        .split("\n\n")
        .any(|block| block.lines().any(|l| l == "Package: systemd-boot") && block.lines().any(|l| l == "Status: install ok installed"));
    if systemd_boot && ["/boot/efi", "/efi", "/boot"].iter().any(|esp| Path::new(esp).join("loader").is_dir()) {
        return "systemd-boot".into();
    }
    "grub".into()
}
