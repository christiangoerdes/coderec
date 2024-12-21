# coderec

Find machine code in binary files or memory dumps. Example:

Cisco IOS firmware image:

![](https://blog.eb9f.de/media/coderec/C800-UNI-159-3.M2_w81920_regions.png)

Cisco boot ROM memory dump:

![](https://blog.eb9f.de/media/coderec/ffc31000_ffd2b000.dump_w4096_regions.png)

## Installation

Including blobs in version control is a Bad Idea. Thus you will have to
download the current corpus and unpack it before you can build the executable.
Here's a one-liner for that:

```
curl --proto '=https' --tlsv1.2 -sSf https://valentinobst.de/a13f15d91f0f8846d748e42e7a881f783eb8f922861a63d9dfb74824d21337039dd8216f0373c3e5820c5e32de8f0a1880ec55456ff0da39f17d32f567d62b84/cpu_rec_corpus.tar.gz -o cpu_rec_corpus.tar.gz && tar xf cpu_rec_corpus.tar.gz && rm cpu_rec_corpus.tar.gz
```

Then you can install this like any other `cargo`-based Rust project:

```
cargo install --locked --path .
```

### Packaging

Users of Arch-based distros can install `coderec` via the
[AUR](https://aur.archlinux.org/packages/coderec).

## How to Read the Plots

There are two kinds of plots: byte plots and region plots. In a byte plot, each
point corresponds to a byte in the file; The X-value is the byte's offset, and
the Y-value is its value. Coloring is used to indicate the detection result for
the region that the byte belongs to. Here is an example of a byte plot:

![](https://valentinobst.de/31c929c36d54d2670a97f8485f09f99c43266e6b5ac51f2b322178d03c2c5f00/bfc00000_bfc90000.dump_w4096_regions.png)

For larger files, byte plots become less useful. For such files you can use the
region plots. Those include the detection result for each region of the target
file as two colored bars at the corresponding offset. The length of the bars
coming from the top and bottom indicates the quality of the tri- and bigram
detection, respectively. Coloring of the bars for one region is used to indicate
if the bi- and trigram detection agreed. Here is an example of a region plot:

![](https://valentinobst.de/e97aabb102c6fc5b241a3eef5772511c7e4089ad5dd12bc859075e442a47fe95/c2800nm-adventerprisek9_sna-mz.124-22_core_02_cropped_w73728_regions.png)

By default, byte plots are produced; The `--big-file` flag switches to region
plots.

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
