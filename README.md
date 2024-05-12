<div align="center">

# Soteria

Soteria is a Polkit authentication agent written in GTK designed to be used with any desktop environment.

[Installation](#installation) •
[Why?](#why) •
[Configuration](#configuration) •
[Usage](#usage)

</div>

## Installation

Soteria requires GTK >= 4.10. For Arch based distros, you will need
`gtk4`, Debian based distros will need `libgtk-4-dev`, and Fedora
based distros will need `gtk4-devel`.

Additionally, you will need `polkit` and `libpolkit-agent` installed.
(`libpolkit-agent` should be shipped with `polkit`)

Soteria will also need Rust. It was developed on Rust `1.78.0` however,
lower versions of Rust should still work.

Run the following commmand to build and install Soteria:

```bash
        cargo install --locked --git https://github.com/imvaskel/soteria
```

This should place Soteria into ~/.cargo/bin and you can run it from there.

## Usage

Simply have your desktop run the `soteria` binary to have it register as your authentication agent. Once run, anytime an application requests polkit authentication, it should popup and prompt you to authenticate.

For Hyprland, this would look like:

```conf
exec-once = /path/to/soteria
```

Other desktop environments should be similiar.

## Why?

When looking for a polkit authentication agent, I noticed that most were either extremely old, using a framework that I didn't like, or completely unstylable.
Additionally, most were hard to edit as they just called out to polkit's `libpolkit-agent` to do all the work. Because of this, I decieded to put the work in to figure out how authentication agents worked.

## Debugging

If you would like to debug why something went wrong, just run `RUST_LOG=debug soteria` and this will start it with debug logging, which should help you identify what's going wrong.
