# piano-rs

![Rust Toolchain](https://img.shields.io/badge/rust-stable-brightgreen.svg)
[![Build Status](https://travis-ci.org/ritiek/piano-rs.svg?branch=master)](https://travis-ci.org/ritiek/piano-rs)

A multiplayer piano using UDP sockets that can be played using computer keyboard, in the terminal.

## Screenshots

[Video clip](https://peertube.social/videos/watch/cb98f9b5-5c5b-417b-bde4-94f17533910c)

<img src="https://i.imgur.com/DOx0wWf.png" width="900">

## Compiling

### Nix

If you have flakes enabled, you can try out the game using:
```
$ nix run github:ritiek/piano-rs
```

And development environment can be setup through:
```
$ git clone https://github.com/ritiek/piano-rs
$ cd piano-rs
$ nix develop
```

You can also build and execute the game using:
```
$ nix build
$ nix run
```

### Other Linux based

You'll need to have Rust compiler and its package manager, Cargo installed to compile piano-rs.
If you don't have them already, head over to https://rustup.rs/ to run the installer.

You can then compile piano-rs with:

```
$ git clone https://github.com/ritiek/piano-rs
$ cd piano-rs
$ cargo build --release
```

You might face the following:

```
error: failed to run custom build command for `alsa-sys v0.1.1`
```

In this case, compiling again after installing `libasound2-dev` should solve the problem:
```
$ sudo apt-get install libasound2-dev
```

### macOS

The game also builds and runs on macOS (including Apple Silicon). The crossterm 0.11
sub-crates are pinned to exact versions in `Cargo.toml`, so a plain install works
without any extra flags:

```
$ cargo install --path .
```

Note: the note sounds are embedded into the binary at compile time, so piano-rs works
from any directory without the `assets/` folder on disk. Pass `-a` to load note sounds
from a custom directory instead.

## Usage

Once it compiles, run the binary with:
```
$ cargo run --release
```

You can also call the binary directly located in `./target/release/piano-rs`.

Additional options to the compiled binary can be passed with cargo or nix using the `--` delimiter:

```
$ cargo run --release -- --help

Play piano in the terminal using PC (computer) keyboard.

USAGE:
    piano-rs [OPTIONS]

FLAGS:
    -c, --central      Vertically and horizontally center the piano in the terminal (Default: off)
    -h, --help         Prints help information
    -k, --show-keys    Display the keyboard letter on each key (Default: off)
    -V, --version      Prints version information

OPTIONS:
    -a, --assets <ASSETS>               Path to assets directory (Default: embedded in binary) [env: ASSETS=]
        --host-address <ADDRESS>        Set the host's IP Address and Port to connect to (Default: 127.0.0.1:9999)
    -m, --mark-duration <DURATION>      Duration to show piano mark for, in ms (Default: 500)
    -n, --note-duration <DURATION>      Duration to play each note for, where 0 means till the end of note (Default: 0)
    -p, --play-file <FILEPATH>          Play notes from .yml file (Default: None)
    -t, --playback-tempo <AMOUNT>       Set playback speed when playing from file (Default: 1.0)
        --receiver-address <ADDRESS>    Set the IP Address and Port to which the receiver socket will bind to (Default:
                                        127.0.0.1:9999, loopback only; pass 0.0.0.0:9999 to accept multiplayer
                                        connections)
    -r, --record-file <FILEPATH>        Record notes to .yml file (Default: None)
        --sender-address <ADDRESS>      Set the IP Address and Port to which the sender socket will bind to. A port of 0
                                        implies to bind on a random unused port (Default: 0.0.0.0:0)
    -s, --sequence <AMOUNT>             Frequency sequence from 0 to 5 to begin with (Default: 2)
    -k, --show-keys                     Display the keyboard letter on each key (Default: off)
    -v, --volume <AMOUNT>               Set initial volume for notes (Default: 1.0)
```

- The piano is drawn in the top-left corner of the terminal by default. Pass `-c, --central`
  to center it instead: the instrument sits in the middle of the terminal with equal margins
  above and below (and on both sides when the terminal is wider than the keyboard). The piano
  also reacts to terminal resizes: the size is watched while the game runs, so it re-centers
  and redraws itself whenever the window changes.

- You can press the keys on your computer keyboard to play the piano notes. The note keys
  follow the layout used by [Multiplayer Piano](https://multiplayerpiano.com): the bottom row
  (`z x c v b n m , . /`) plays the lower white keys, the home row (`a s f g j k l '`) the
  lower black keys, the top row (`q w e r t y u i o p [ ]`) the upper white keys and
  `1 2 4 5 7 8 9` the upper black keys.

- New to the key layout? Pass `-k, --show-keys` to print the keyboard letter on each piano key.
  The labels follow the note mapping: they shift with <kbd>←</kbd> / <kbd>→</kbd>, and when you
  play a key with <kbd>Shift</kbd> (one octave up) or <kbd>Ctrl</kbd>/<kbd>Alt</kbd> (one octave
  down) held, the labels move with the notes and return to the base position once you play
  without the modifier. (Note: a terminal only reports a modifier when it is held together with
  another key, so the labels update as you play, not on the bare press/release of the modifier.)

- Increase or decrease the note frequency with <kbd>←</kbd> and <kbd>→</kbd> respectively
  (or hold <kbd>Ctrl</kbd> or <kbd>Alt</kbd> for one octave down, <kbd>Shift</kbd> for one
  octave up, while playing).

- Press <kbd>Space</kbd> to toggle the sustain pedal, drawn on the right side below the
  keyboard like a real piano's sustain pedal (it lights up while sustain is active).
  <kbd>Backspace</kbd> does the same as a sustain lock. While the pedal is down, notes ring
  out to their natural end instead of stopping after the configured note duration. With
  `-k, --show-keys`, the pedal shows its `SPACE` key label.

- A status row below the pedal shows the current volume, note length and octave, updating as
  you press <kbd>+</kbd>/<kbd>-</kbd>, <kbd>↑</kbd>/<kbd>↓</kbd> and <kbd>←</kbd>/<kbd>→</kbd>.

- With `-k, --show-keys`, a key hint panel is drawn at the top-right corner
  of the terminal: `Shift+Key` octave up, `Alt+Key` octave down, `Arrows` change octave,
  `Space` sustain and `Backspace` sustain lock.

- You can also record your piano session by passing the command-line argument `-r <path/to/save/notes.yml>`
  and play them later on with `-p <path/to/save/notes.yml>`.

Press the <kbd>Esc</kbd> key to exit the game.

## Multiplayer

piano-rs is multiplayer! It can also be enjoyed with friends by sharing the same piano session. Here's how to setup:

By default (solo mode) the receiver socket binds to `127.0.0.1:9999` — loopback only, so playing by
yourself exposes no server and triggers no firewall prompt. To play together, **every machine that
should receive notes must explicitly open its receiver** with `--receiver-address 0.0.0.0:9999` (or
its own LAN address):

On the 1st machine, launch piano-rs with the receiver open:
```
$ cargo run --release -- --receiver-address=0.0.0.0:9999
```
or
```
$ ./target/release/piano-rs --receiver-address=0.0.0.0:9999
```

On the 2nd machine, open its own receiver as well (so the 1st machine can send notes back) and pass
the 1st machine's address to connect to its session:
```
$ cargo run --release -- --receiver-address=0.0.0.0:9999 --host-address=192.168.1.3:9999
```
or
```
$ ./target/release/piano-rs --receiver-address=0.0.0.0:9999 --host-address=192.168.1.3:9999
```

Here, 192.168.1.3 is the IP address of the 1st machine. Both machines must be able to reach each
other on UDP port 9999 (or whichever port you pick).

The 2nd machine should now be connected and will share the same piano-rs session as the host machine.
Any keys you hit, should be marked with a different color indicator (each player gets a color assigned
by the order they joined).

Similar to the way you connected the 2nd machine, you can connect any number of machines to share
the same piano-rs session!

--------------------

**NOTE:** These multiplayer features do not make use of tokio-rs runtime and instead use `std::net::UdpSocket`
for communication, which comes included with the Rust standard library. The major limitation of relying on
`std::net::UdpSocket` is that the network requests are handled sequentially on the basis of first come,
first serve. This would be a problem if hundreds of players are connected to the same piano-rs session and
are hitting the keys at the same time. Obviously, we could acheive much better performance if we were to
handle network requests asynchronously with [tokio-rs](https://github.com/tokio-rs/tokio) and
[futures](https://docs.rs/futures/0.1.29/futures/). Unfortunately, these awesome libraries
have a bit of learning curve which I don't have the time to go through at the moment! It will be awesome if
someone would like to help here make a transition to asynchronously handle network requests.

The cool devs at tokio-rs have also been trying to lower the learning curve by introducing `async` and `await`
keywords, similar to [Python](https://docs.python.org/3/library/asyncio.html). However, these keywords at the
moment are only available under the recent alpha release of tokio-rs for Rust nightly. See the relevant
[blog post](https://tokio.rs/blog/2019-08-alphas/).

## Running tests

```
$ cargo test
```

## Resources

- piano-rs uses the same note sounds and key bindings as [multiplayerpiano](http://multiplayerpiano.com).
  In fact, the note sound files you see in the [assets](https://github.com/ritiek/piano-rs/tree/master/assets)
  sub-directory are downloaded from multiplayerpiano itself.
  If you're a moderator on their website and got a problem with this, let me know and I'll remove and
  stop using the sound files in this repository.

- You can use this [paste](https://pastebin.com/CX1ew0uB) to learn to play some popular songs. If you're
  interested, I've *transcribed* a few synthesia YouTube videos [in this gist](https://gist.github.com/ritiek/28be91b64ef82f0ff8599c1037e1e05e),
  so they can be played with piano-rs.

## License

`The MIT License`
