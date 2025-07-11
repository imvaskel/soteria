fn main() {
    let polkit_location = std::env::var("POLKIT_AGENT_HELPER_PATH")
        .unwrap_or("/usr/lib/polkit-1/polkit-agent-helper-1".into());
    println!("compiling with polkit-agent-helper-1 located at {polkit_location}");
    println!("cargo::rustc-env=POLKIT_AGENT_HELPER_PATH={polkit_location}");
}
