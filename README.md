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

Keys: `space` pause / resume, `←/→` wander (Shift ×5), `[ ]` speed, `+ -` zoom, `↑/↓` look up/down, `m` braille · octant · half-block, `r` new seed, `e` save SVG, `i` status line, `?` help, `q` quit.

## Credits

Thank you, Lingdong Huang, for the amazing project: [LingdongHuang/shan-shui-inf: Shan, Shui\* (github.com)](https://github.com/LingdongHuang/shan-shui-inf)

### Original README (keeped as-is)

Procedurally-generated vector-format infinitely-scrolling Chinese landscape for the browser.

Generate your own on https://lingdong-.github.io/shan-shui-inf/ (or [Alternative link](https://shan-shui-inf.glitch.me)).

Some examples:
![Screenshot1](/screenshots/screen001.jpg?raw=true "")
![Screenshot2](/screenshots/screen002.jpg?raw=true "")

{Shan, Shui}\* is inspired by [traditional Chinese landscape scrolls](https://en.wikipedia.org/wiki/Shan_shui) (such as [this](https://en.wikipedia.org/wiki/Dwelling_in_the_Fuchun_Mountains) and [this](https://en.wikipedia.org/wiki/Wang_Ximeng)) and uses noises and mathematical functions to model the mountains and trees from scratch. It is written entirely in javascript and outputs Scalable Vector Graphics (SVG) format.
