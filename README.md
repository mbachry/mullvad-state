# mullvad-state

A dbus service to provide easy access mullvad VPN state and a
py3status widget.

## Install the daemon

Build with `cargo build --release` and put
`target/release/mullvad-state` to bin path.

Add a systemd service:

```
[Unit]
Description=mullvad state daemon

[Service]
ExecStart=%h/.local/bin/mullvad-state

[Install]
WantedBy=default.target
```

and start with `systemd --user enable --now mullvad-state.service`.

## Install py3status widget

Install Python dbus bindings, eg. `sudo dnf install python3-dbus` on
Fedora.

Copy `py3status/mullvad.py` to `~/.config/py3status/modules/` and add
`order += "mullvad"` to `py3status` configuration. Reload swaybar.
