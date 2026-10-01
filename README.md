# five-months

Waiting for 4 and a half of billions of years, to watch humanity rise again from ashes and dusks. The Cycle of Life will never stop.

A gift, for our beloved Godmother of Destruction.

Hope you like this late birthday present.

## In the terminal

`tui/` is a Rust + [Ratatui](https://ratatui.rs) port that paints the endless scroll in braille dots, right in your terminal (truecolor recommended).

```sh
make run                 # unroll the scroll (seed "naught")
make run SEED=anything   # same seed, same landscape, every time
make birthday            # what it looks like on the 11th of August
make png X=3000          # one frame to a PNG, without a terminal
make                     # every other target
```

Keys: `space` pause / resume, `←/→` wander (Shift ×5), `[ ]` speed, `+ -` zoom, `↑/↓` look up/down, `m` braille · octant · half-block, `w` ink light · normal · bold, `r` new seed, `e` save SVG, `i` status line, `?` help, `q` quit.

Flags: `--speed 30` (world units per second), `--ink bold|normal|light`, `--marker braille|octant|half-block`, `--palette auto|truecolor|256`, `--no-splash`. See `five-months --help`.

### For the smoothest scroll

Every one-dot step of the scroll redraws most of the screen, so the terminal emulator matters more than the CPU:

- **Best:** GPU-rendered terminals with truecolor and synchronized output, e.g. [kitty](https://sw.kovidgoyal.net/kitty/), [WezTerm](https://wezfurlong.org/wezterm/), [Ghostty](https://ghostty.org/), [Alacritty](https://alacritty.org/) or [foot](https://codeberg.org/dnkl/foot) (Wayland).
- **Works, less smooth:** GNOME Terminal / Ptyxis (Ubuntu's default), Konsole, the VS Code terminal. If it stutters, try `--speed 20`, a smaller window, or a larger font (fewer cells to redraw).
- **Fonts:** pick one whose braille glyphs fill the cell, e.g. DejaVu Sans Mono, JetBrains Mono, Iosevka or any Nerd Font; `--marker half-block` needs no special glyphs at all.

## Credits

Thank you, Lingdong Huang, for the amazing project: [LingdongHuang/shan-shui-inf: Shan, Shui\* (github.com)](https://github.com/LingdongHuang/shan-shui-inf)

### Original README (keeped as-is)

Procedurally-generated vector-format infinitely-scrolling Chinese landscape for the browser.

Generate your own on https://lingdong-.github.io/shan-shui-inf/ (or [Alternative link](https://shan-shui-inf.glitch.me)).

Some examples:
![Screenshot1](/screenshots/screen001.jpg?raw=true "")
![Screenshot2](/screenshots/screen002.jpg?raw=true "")

{Shan, Shui}\* is inspired by [traditional Chinese landscape scrolls](https://en.wikipedia.org/wiki/Shan_shui) (such as [this](https://en.wikipedia.org/wiki/Dwelling_in_the_Fuchun_Mountains) and [this](https://en.wikipedia.org/wiki/Wang_Ximeng)) and uses noises and mathematical functions to model the mountains and trees from scratch. It is written entirely in javascript and outputs Scalable Vector Graphics (SVG) format.
