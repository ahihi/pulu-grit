# pulu-grit

a collection of distortion algorithms in a VST3 and/or CLAP plugin.

made in the process of exploring nih-plug on my [livestream](https://youtube.com/live/pxIDK3PoAfU) :)

## algorithms included

- hard clip
- [SuperDirt](https://github.com/musikinformatik/SuperDirt/) Shape
- [Barry's Satan Maximizer](https://github.com/swh/lv2/tree/master/plugins/satan_maximiser-swh.lv2)
- [Dirt compressor](https://github.com/tidalcycles/Dirt/blob/071fd88b3e004a11215afd11b718e92d3ab44d18/audio.c#L770-L777)

## building

```shell
cargo xtask bundle pulu-grit --release
```
