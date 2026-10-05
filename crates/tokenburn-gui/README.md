# tokenburn-gui

Desktop GUI ([Iced](https://iced.rs)) for [TokenBurn](https://github.com/sorinirimies/tokenburn) — local AI token/cost usage for **pi** and **Zed**.

Pick the tool, the bucket (total / hourly / daily / monthly), the window and a theme; the report refreshes in the background and every 60 s.

## Platforms

macOS, Windows and Linux. On Linux **one binary serves X11 and Wayland**: it only links libc/libm and loads the display-server and GPU libraries at runtime (Wayland when `WAYLAND_DISPLAY` is set, X11 otherwise). Rendering falls back to software when there is no usable GPU.

```sh
env -u WAYLAND_DISPLAY tokenburn-gui     # force X11 (XWayland) inside a Wayland session
ICED_BACKEND=tiny-skia tokenburn-gui     # force software rendering (VMs, remote desktops)
tokenburn-gui --daily --tool pi          # choose the initial view
```
