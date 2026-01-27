fn main() {
    let polkit_agent_helper_path = std::env::var("POLKIT_AGENT_HELPER_PATH")
        .unwrap_or("/usr/lib/polkit-1/polkit-agent-helper-1".into());
    let polkit_agent_socket_path = std::env::var("POLKIT_AGENT_SOCKET_PATH")
        .unwrap_or("/run/polkit/agent-helper.socket".into());

    println!("compiling with polkit-agent-helper-1 located at {polkit_agent_helper_path}");
    println!("cargo::rustc-env=POLKIT_AGENT_HELPER_PATH={polkit_agent_helper_path}");
    println!("cargo::rustc-env=POLKIT_AGENT_SOCKET_PATH={polkit_agent_socket_path}");
}
