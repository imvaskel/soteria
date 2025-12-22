<div align="center">

# Soteria

Soteria is a Polkit authentication agent written in GTK designed to be used with any desktop environment.

<img alt="Example authentication popup" src=".github/example_popup.png" width=50% height=50% ></image>

[Installation](#installation) •
[Why?](#why) •
[Usage](#usage)

</div>

## Installation

> [!NOTE]
> Some users using non-desktop environments (sway, etc) have reported that ``XDG_SESSION_ID`` is not being properly imported.
> XDG session info is required for the agent to register itself to polkit.
> To fix this, you must import the proper environment variables (assuming systemd is managing the user session):
> ```
> dbus-update-activation-environment --systemd DISPLAY WAYLAND_DISPLAY XDG_CURRENT_DESKTOP
> ```
> For more info, see NixOS/nixpkgs#373290.

### Arch Linux

Soteria is available on the [AUR](https://aur.archlinux.org/packages/soteria-git) as `soteria-git`
. You can install it using an AUR helper:

```bash
# Using yay
yay -S soteria-git
# or using paru
paru -S soteria-git
```
or manually: 
```bash
git clone https://aur.archlinux.org/soteria-git.git
cd soteria-git
makepkg -si
```
This should place Soteria into `/usr/lib/soteria-polkit/soteria`

### NixOS

Soteria is available as `soteria`. There is a also NixOS module to enable it under ``security.soteria.enable``.

### Manual Installation
    
#### Requirements
Soteria requires Rust >= 1.85 (edition 2024), GTK4 development headers (`libgtk-4-dev` / `gtk4-devel`), Polkit development headers (`libpolkit-agent-1-dev`), and `gettext` for compiling translations.

> [!NOTE]
> If your `polkit-agent-helper-1` executable is in a non-standard location (i.e. not `/usr/lib/polkit/polkit-agent-helper-1`), 
> you should set up a configuration file at `~/.config/soteria/config.toml` (or `/etc/soteria/config.toml`) with:
> ```toml
> helper_path = "/path/to/your/helper"
> ```

Run the following commands to build and install Soteria:

```bash
git clone https://github.com/imvaskel/soteria
cd soteria

# Install binary
cargo install --locked --path .

# Install translations (locally)
mkdir -p ~/.local/share/locales
for file in po/*.po; do \
    lang=${file%.*}; \
    mkdir -p ~/.local/share/locales/${lang#po/}/LC_MESSAGES; \
    msgfmt $file -o ~/.local/share/locales/${lang#po/}/LC_MESSAGES/soteria.mo; \
done

# Run with local translations
SOTERIA_LOCALEDIR=~/.local/share/locales soteria
```

> [!NOTE]
> By default, Soteria looks for translations in `/usr/share/locale`. Use `SOTERIA_LOCALEDIR` to override this path, as shown above.

This should place Soteria into ~/.cargo/bin and you can run it from there.

## Usage

Simply have your desktop run the `soteria` binary to have it register as your authentication agent. Once run, anytime an application requests polkit authentication, it should popup and prompt you to authenticate.

For Hyprland, this would look like:

```conf
exec-once = /path/to/soteria
```

You may also like:

```conf
windowrulev2=pin,class:gay.vaskel.soteria
```

This makes sure that Soteria stays pinned to your current workspace.

Other desktop environments should be similiar.

## Why?

When looking for a polkit authentication agent, I noticed that most were either extremely old, using a framework that I didn't like, or completely unstylable.
Additionally, most were hard to edit as they just called out to polkit's `libpolkit-agent` to do all the work. Because of this, I decided to put the work in to figure out how authentication agents worked.

It should be noted that this project does still call out to libpolkit-agent, but only via the polkit agent helper. This is because polkit
uses root sending a dbus response to the polkit daemon to confirm authentication as the identity. I find it non-beneficial to put in
the work to maintain the security implications of a setuid binary.

## Debugging

If you would like to debug why something went wrong, just run `RUST_LOG=debug soteria` and this will start it with debug logging, which should help you identify what's going wrong.
