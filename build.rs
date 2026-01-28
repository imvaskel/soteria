fn main() {
    let polkit_agent_helper_path = std::env::var("POLKIT_AGENT_HELPER_PATH")
        .unwrap_or("/usr/lib/polkit-1/polkit-agent-helper-1".into());
    let polkit_agent_socket_path = std::env::var("POLKIT_AGENT_SOCKET_PATH")
        .unwrap_or("/run/polkit/agent-helper.socket".into());
    let soteria_default_locale_dir =
        std::env::var("SOTERIA_DEFAULT_LOCALE_DIR").unwrap_or("/usr/share/locale".into());

    println!("compiling with polkit-agent-helper-1 located at {polkit_agent_helper_path}");
    println!("cargo::rustc-env=POLKIT_AGENT_HELPER_PATH={polkit_agent_helper_path}");
    println!("cargo::rustc-env=POLKIT_AGENT_SOCKET_PATH={polkit_agent_socket_path}");
    println!("cargo::rustc-env=SOTERIA_DEFAULT_LOCALE_DIR={soteria_default_locale_dir}")
}
