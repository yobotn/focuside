fn main() {
    // The target triple names the adapter sidecar: claude-agent-acp-<triple>[.exe].
    println!(
        "cargo:rustc-env=FOCUSIDE_TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}
