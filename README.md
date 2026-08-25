# Battle Rats

A 2D side-scrolling tower defense inspired by *The Battle Cats*, written in Rust
using the [macroquad](https://github.com/not-fl3/macroquad) game framework.
Every visual — units, bases, buttons — is a drawn shape (circle, square,
triangle, diamond) with colors and health bars, so there are **no external
image or audio assets** to manage.

## How to play

- You defend the base on the **left**; the enemy defends the base on the **right**.
- Money trickles in automatically over time.
- Click the buttons at the bottom of the screen to deploy units:
  - **Ratling** (circle) — cheap, fast, balanced melee.
  - **Tank Rat** (square) — huge HP, slow, great at soaking hits.
  - **Spear Rat** (triangle) — long attack range, low HP.
  - **Bomb Rat** (diamond) — big hits, slow attack speed.
- Deployed units auto-walk toward the enemy and fight whatever they run into
  (an enemy unit, or the enemy base if the lane is clear).
- Enemies (Grubs, Roaches, Beetles, and occasional Bosses) spawn on their own
  and get stronger and more frequent the longer the match goes.
- Destroy the enemy base to win. If your base HP hits 0, you lose.
- Press **SPACE** after the match ends to play again.

## Running it

You'll need the Rust toolchain (https://rustup.rs). On Linux you'll also need
X11/OpenGL dev headers if they're not already installed, e.g. on
Debian/Ubuntu:

```bash
sudo apt-get install libx11-dev libxi-dev libgl1-mesa-dev libxcursor-dev libxrandr-dev libxinerama-dev
```

Then, from this folder:

```bash
cargo run --release
```

A window will open with the game running.

## Project layout

- `src/main.rs` — the entire game (unit stats, spawning, movement/combat AI,
  UI, and all shape-based rendering). It's organized top-to-bottom with
  comments, so it's a good starting point for adding new unit types, enemy
  types, or mechanics like multiple lanes.
- `Cargo.toml` — dependencies (just `macroquad`).

## Ideas for extending it

- Add more unit/enemy types, or a "critical" enemy that only certain units
  can hit effectively.
- Add a second lane, or flying enemies that skip melee units.
- Add unlockable units, persistent currency between runs, or a stage/level
  select screen.
- Swap the placeholder shapes for sprite art once you have assets — the
  `Unit` struct's `shape`/`color` fields are the only things you'd need to
  replace with a texture reference.
