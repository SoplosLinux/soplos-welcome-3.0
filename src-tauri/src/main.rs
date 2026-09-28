// Entry point delegates to the library so integration tests can reuse it.

fn main() {
    // Privileged-helper mode: this same binary, re-invoked by pkexec under
    // the polkit action declared in org.soploslinux.welcome.policy, instead
    // of a separate helper executable. Must be checked before Tauri starts,
    // since this path never opens a window — it just runs the script and
    // exits with its exit code. See software::start_pkexec_job.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--run-privileged-script" {
        std::process::exit(soplos_welcome_lib::run_privileged_script_file(&args[2]));
    }

    // WebKitGTK's DMA-BUF renderer can leave the window blank on virtual GPUs.
    // Must be set before the webview initializes, so it happens here in main().
    if soplos_welcome_lib::is_running_in_vm() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    soplos_welcome_lib::run();
}
