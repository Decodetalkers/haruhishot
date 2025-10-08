# haruhishot

It is a screenshot tool for wlroots based compositors such as sway and river written in Rust, with wayland-rs

![haruhis](./images/haruhi.jpg)

## How to build

dependiences: `wayland` , `wlroots` , `rust` and `meson`

You can just run `cargo run`

If you want to package it , you can use `meson.build`

```bash
  meson setup \
    -Dprefix=/usr \
    -Dbuildtype=release \
    build
  ninja -C build
```

## Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/haruhishot.svg)](https://repology.org/project/haruhishot/versions)

## Thanks to wayshot

## Use example

Pick with Region

```
haruhishot -S --stdout | wl-copy
```

or

```
haruhishot --slurp --stdout | wl-copy
```

Get Lists

```
haruhishot -L
```

or
```
haruhishot --list-outputs
```

Shot one screen

```
haruhishot -O DP-2 --stdout > test.png
```

or

```
haruhishot --output DP-2 --stdout > test.png
```

Get Color

```
haruhishot -C
```

or

```
haruhishot --color
```

## Features

### Notify Message

![notify](./images/notify.png)

## TODO

* I want to add a slint frontend
* ~~Real Fullscreen shot~~
* In the code of wayshot, it seems need to make change if meet some format, but it works well on my computer, so..

## Thanks to the help of developers in Smithay
