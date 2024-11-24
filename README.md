# coderec

Find machine code in binary files or memory dumps. Example:

Cisco IOS firmware image:

![](https://blog.eb9f.de/media/coderec/C800-UNI-159-3.M2_w81920_regions.png)

Cisco boot ROM memory dump:

![](https://blog.eb9f.de/media/coderec/ffc31000_ffd2b000.dump_w4096_regions.png)
![](https://blog.eb9f.de/media/coderec/bfc00000_bfc90000.dump_w4096_regions.png)

## Installation

Including blobs in version control is a Bad Idea. Thus you will have to
download the current corpus and unpack it before you can build the executable.
Here's a one-liner for that:

```
curl --proto '=https' --tlsv1.2 -sSf https://valentinobst.de/95c746fc63c16fc1474ed0cbbcead47a0d46383fd3296cbbef86db5ed4a362cf/cpu_rec_corpus.7z -o cpu_rec_corpus.7z && 7z x cpu_rec_corpus.7z && rm cpu_rec_corpus.7z
```

Then you can install this like any other `cargo`-based Rust project:

```
cargo install --locked --path .
```

## About

The underlying approach to machine code detection and corpus are taken from
[`cpu_rec`](https://github.com/airbus-seclab/cpu_rec/). This codebase is a
hard-fork of [`cpu_rec_rs`](https://github.com/trou/cpu_rec_rs).

Why use this and not `cpu_rec_rs`? Some reasons:

- utilizes all your cores (makes it soooooooo muuuuuuch faster on big files...),
- produces beautiful plots,
- more ergonomic to use as it embeds the corpus (put the binary on your PATH and It Just Works),
- different heuristic for filtering false positives,
- different heuristic for window sizing on large files (as we utilize all cores
  it is acceptable to use smaller windows on large files -> better results),
- better detection of string and high-entropy regions,

See our [blog post](https://blog.eb9f.de/2024/11/24/coderec.html) for more information.

Note: as the approach is based on statistics, false positives are definitely
possible. You should cross check with other sources and validate the results
with a disassembler.
