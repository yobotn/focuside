//! `focuside-hook [agent]`: see lib.rs. Prints nothing and always exits 0.

fn main() {
    // A panic must not print to stderr: replace the default hook, which also reads
    // RUST_BACKTRACE.
    std::panic::set_hook(Box::new(|_| {}));
    let _ = std::panic::catch_unwind(|| {
        // The only environment variable read. Missing means this is not a FocusIDE pane.
        let Some(dir) = std::env::var_os("FOCUSIDE_EVENTS") else {
            return;
        };
        let arg = std::env::args_os().nth(1);
        let agent = focuside_hook::agent_arg(arg.as_ref().and_then(|a| a.to_str()));
        focuside_hook::run(
            Some(std::path::Path::new(&dir)),
            &agent,
            std::io::stdin().lock(),
        );
    });
    std::process::exit(0);
}
