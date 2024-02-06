# batteryrc
A service for running certain commands when a computer connects/disconnects from battery.

On Linux, the config is probably stored at `~/.config/batteryrc/.batteryrc`, create the file if it doesn't exist.
MacOS and Windows will probably work, but they haven't been tested.

## Dummy config
```
@ac
echo Plugged in
echo Lets go!
@battery
echo No longer plugged in
echo Oh no!
```
## Notes
I, personally, added the line `exec batteryrc` to my `sway` config, but this program could run as a service with `systemd` or the like.

## Install without compiling
Download the latest autobuild in the `Releases` section of the page, and move/rename it to a directory which is in your `PATH`, probably `/usr/local/bin/`.

## Manually build and install
1. Make sure you have the Rust compiler
2. `git clone https://github.com/Jaycadox/batteryrc && cd batteryrc && cargo build -r && cargo install --path .`
