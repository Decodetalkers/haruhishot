# haruhishot

![haruhis](./images/haruhi.jpg)
## Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/haruhishot.svg)](https://repology.org/project/haruhishot/versions)

## Thanks to wayshot

## Use example

Fullscreen (No, just the first screen, just because I am not familar with image.rs)

```
haruhishot | wl-copy
```

or

```
haruhishot > suzumiya.png
```

Pick with Region

```
haruhishot -S (slurp -d) --stdout | wl-copy
```

or

```
haruhishot --slurp (slurp -d) --stdout | wl-copy
```

Get Lists

```
haruhishot -L
```

or
```
haruhishot --list
```

Shot one screen

```
haruhishot -O DP-2 --stdout > test.png
```

or

```
haruhishot --output DP-2 --stdout > test.png
```

## TODO

* I want to add a slint fontend
* Real Fullscreen shot
* In the code of wayshot, it seems need to make change if meet some format, but it works well on my computer, so..

## Thanks to the help of developers in Simthay
