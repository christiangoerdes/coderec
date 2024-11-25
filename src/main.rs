/*
    Copyright 2023 - Raphaël Rigo

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

        http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/
// Includes (many) changes by Valentin Obst.

mod corpus;
mod plotting;

use crate::corpus::{load_corpus, CorpusStats};

use std::cmp::min;
use std::collections::{BTreeMap, HashMap};
use std::convert::From;
use std::ops::Range;

use anyhow::{Context, Result};
use clap::{arg, Arg, ArgAction};
use log::{debug, info};
use rayon::prelude::*;

#[derive(Debug)]
struct KlRes {
    arch: String,
    div: f64,
}

struct RangeFullKlRes {
    kl_bg: Vec<KlRes>,
    kl_tg: Vec<KlRes>,
}

fn calculate_kl(corpus_stats: &[CorpusStats], target: &CorpusStats) -> RangeFullKlRes {
    let mut kl_bg = Vec::<KlRes>::with_capacity(corpus_stats.len());
    let mut kl_tg = Vec::<KlRes>::with_capacity(corpus_stats.len());

    for arch_stats in corpus_stats {
        let r = target.compute_kl(arch_stats);
        kl_bg.push(KlRes {
            arch: arch_stats.arch.clone(),
            div: r.bigrams,
        });
        kl_tg.push(KlRes {
            arch: arch_stats.arch.clone(),
            div: r.trigrams,
        });
    }

    // Sort
    kl_bg.sort_unstable_by(|a, b| a.div.partial_cmp(&b.div).unwrap());
    debug!("Results 2-gram: {:?}", &kl_bg[0..2]);
    kl_tg.sort_unstable_by(|a, b| a.div.partial_cmp(&b.div).unwrap());
    debug!("Results 3-gram: {:?}", &kl_tg[0..2]);

    RangeFullKlRes { kl_bg, kl_tg }
}

struct ProcessedDetectionResult {
    pub win_sz: usize,
    pub max_kl_bg: f64,
    pub min_kl_bg: f64,
    pub max_kl_tg: f64,
    pub min_kl_tg: f64,
    pub range_to_result_bg: HashMap<Range<usize>, RangeResult>,
    pub range_to_result_tg: HashMap<Range<usize>, RangeResult>,
    pub arch_to_idx: HashMap<Arch, usize>,
    pub idx_to_arch: HashMap<usize, Arch>,
    pub kl_arch_to_range_bg: BTreeMap<Arch, Vec<(Range<usize>, f64)>>,
    pub kl_arch_to_range_tg: BTreeMap<Arch, Vec<(Range<usize>, f64)>>,
    pub range_to_final_result: HashMap<Range<usize>, Option<Arch>>,
    pub arch_to_final_ranges: HashMap<Arch, Vec<Range<usize>>>,
}

pub struct RangeResult {
    arch: Arch,
    div: f64,
    range_mean: f64,
    range_var: f64,
}

pub fn final_range_result(res_bg: &RangeResult, res_tg: &RangeResult) -> Option<Arch> {
    let RangeResult {
        arch: bg_arch,
        div: bg_div,
        range_mean: mean_bg,
        range_var: var_bg,
    } = res_bg;
    let std_deviation_bg = var_bg.sqrt();
    let RangeResult {
        arch: tg_arch,
        div: tg_div,
        range_mean: mean_tg,
        range_var: var_tg,
    } = res_tg;
    let std_deviation_tg = var_tg.sqrt();

    if tg_div
        .partial_cmp(&(mean_tg - 2.0 * std_deviation_tg))
        .unwrap()
        == core::cmp::Ordering::Less
    {
        Some(tg_arch.clone())
    } else if bg_div
        .partial_cmp(&(mean_bg - 2.0 * std_deviation_bg))
        .unwrap()
        == core::cmp::Ordering::Less
    {
        Some(bg_arch.clone())
    } else if bg_div
        .partial_cmp(&(mean_bg - 1.0 * std_deviation_bg))
        .unwrap()
        == core::cmp::Ordering::Less
        && tg_div
            .partial_cmp(&(mean_tg - 1.0 * std_deviation_tg))
            .unwrap()
            == core::cmp::Ordering::Less
        && tg_arch == bg_arch
    {
        Some(tg_arch.clone())
    } else if tg_div
        .partial_cmp(&(mean_tg - 1.0 * std_deviation_tg))
        .unwrap()
        == core::cmp::Ordering::Less
        && tg_arch.starts_with("_words")
    {
        Some(tg_arch.clone())
    } else {
        None
    }
}

impl From<(Arch, f64, f64, f64)> for RangeResult {
    fn from(i: (Arch, f64, f64, f64)) -> Self {
        Self {
            arch: i.0,
            div: i.1,
            range_mean: i.2,
            range_var: i.3,
        }
    }
}

pub fn calculate_mean(data: &[f64]) -> f64 {
    data.iter().sum::<f64>() / (data.len() as f64)
}

pub fn calculate_variance(data: &[f64], mean: f64) -> f64 {
    data.iter().map(|x| f64::powi(x - mean, 2)).sum::<f64>() / (data.len() as f64)
}

impl From<DetectionResult> for ProcessedDetectionResult {
    fn from(res_ex: DetectionResult) -> Self {
        // Size of a range.
        let win_sz = res_ex.kl_bg_range_to_arch.keys().next().unwrap().len();

        // Numbering of arches.
        let mut arch_to_idx: HashMap<Arch, usize> = HashMap::new();
        let mut idx_to_arch: HashMap<usize, Arch> = HashMap::new();
        for (arch_idx, (arch, _res)) in res_ex.kl_bg_arch_to_range.iter().enumerate() {
            arch_to_idx.insert(arch.clone(), arch_idx);
            idx_to_arch.insert(arch_idx, arch.clone());
        }

        // Global max and min.
        let mut all_divs_bg: Vec<f64> = res_ex
            .kl_bg_arch_to_range
            .values()
            .flat_map(|arch| arch.iter().map(|(_, div)| *div))
            .collect();
        all_divs_bg.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let max_kl_bg = *all_divs_bg.last().unwrap();
        let min_kl_bg = *all_divs_bg
            .iter()
            .find(|div| (*div).partial_cmp(&0.1).unwrap() != core::cmp::Ordering::Less)
            .unwrap();
        let mut all_divs_tg: Vec<f64> = res_ex
            .kl_tg_arch_to_range
            .values()
            .flat_map(|arch| arch.iter().map(|(_, div)| *div))
            .collect();
        all_divs_tg.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let max_kl_tg = *all_divs_tg.last().unwrap();
        let min_kl_tg = *all_divs_tg
            .iter()
            .find(|div| (*div).partial_cmp(&0.1).unwrap() != core::cmp::Ordering::Less)
            .unwrap();

        // Per-range min (with arch), mean, and variance.
        let range_to_result_bg: HashMap<Range<usize>, RangeResult> = res_ex
            .kl_bg_range_to_arch
            .iter()
            .map(|(range, arches)| {
                let mut arches = arches.clone();
                arches.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                let divs: Vec<_> = arches.iter().map(|(_, div)| *div).collect();

                let mean = calculate_mean(&divs);
                let var = calculate_variance(&divs, mean);

                (
                    range.clone(),
                    (arches[0].0.clone(), arches[0].1, mean, var).into(),
                )
            })
            .collect();
        let range_to_result_tg: HashMap<Range<usize>, RangeResult> = res_ex
            .kl_tg_range_to_arch
            .iter()
            .map(|(range, arches)| {
                let mut arches = arches.clone();
                arches.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                let divs: Vec<_> = arches.iter().map(|(_, div)| *div).collect();

                let mean = calculate_mean(&divs);
                let var = calculate_variance(&divs, mean);

                (
                    range.clone(),
                    (arches[0].0.clone(), arches[0].1, mean, var).into(),
                )
            })
            .collect();

        // Our final verdict.
        let range_to_final_result: HashMap<Range<usize>, Option<String>> = range_to_result_bg
            .iter()
            .map(|(range, res_bg)| {
                let res_tg = range_to_result_tg.get(range).unwrap();

                (range.clone(), final_range_result(res_bg, res_tg))
            })
            .collect();

        let mut arch_to_final_ranges: HashMap<Arch, Vec<Range<usize>>> = HashMap::new();
        for (range, arch_op) in range_to_final_result.iter() {
            if let Some(arch) = arch_op {
                arch_to_final_ranges
                    .entry(arch.clone())
                    .and_modify(|ranges| ranges.push(range.clone()))
                    .or_insert(vec![range.clone()]);
            }
        }

        Self {
            win_sz,
            arch_to_idx,
            idx_to_arch,
            max_kl_bg,
            min_kl_bg,
            max_kl_tg,
            min_kl_tg,
            range_to_result_bg,
            range_to_result_tg,
            kl_arch_to_range_bg: res_ex.kl_bg_arch_to_range,
            kl_arch_to_range_tg: res_ex.kl_tg_arch_to_range,
            range_to_final_result,
            arch_to_final_ranges,
        }
    }
}

type Arch = String;
struct DetectionResult {
    pub kl_bg_arch_to_range: BTreeMap<Arch, Vec<(Range<usize>, f64)>>,
    pub kl_tg_arch_to_range: BTreeMap<Arch, Vec<(Range<usize>, f64)>>,
    pub kl_bg_range_to_arch: HashMap<Range<usize>, Vec<(Arch, f64)>>,
    pub kl_tg_range_to_arch: HashMap<Range<usize>, Vec<(Arch, f64)>>,
}

impl<I: ParallelIterator<Item = (Range<usize>, RangeFullKlRes)>> From<I> for DetectionResult {
    fn from(i: I) -> Self {
        let mut res_ex = Self {
            kl_bg_arch_to_range: BTreeMap::new(),
            kl_tg_arch_to_range: BTreeMap::new(),
            kl_bg_range_to_arch: HashMap::new(),
            kl_tg_range_to_arch: HashMap::new(),
        };
        let res: Vec<_> = i.collect();

        for (range, RangeFullKlRes { kl_bg, kl_tg }) in res {
            for (kl_bg_arch, kl_tg_arch) in kl_bg.into_iter().zip(kl_tg.into_iter()) {
                res_ex
                    .kl_bg_arch_to_range
                    .entry(kl_bg_arch.arch.clone())
                    .and_modify(|e| e.push((range.clone(), kl_bg_arch.div)))
                    .or_insert(vec![(range.clone(), kl_bg_arch.div)]);
                res_ex
                    .kl_tg_arch_to_range
                    .entry(kl_tg_arch.arch.clone())
                    .and_modify(|e| e.push((range.clone(), kl_tg_arch.div)))
                    .or_insert(vec![(range.clone(), kl_tg_arch.div)]);
                res_ex
                    .kl_bg_range_to_arch
                    .entry(range.clone())
                    .and_modify(|e| e.push((kl_bg_arch.arch.clone(), kl_bg_arch.div)))
                    .or_insert(vec![(kl_bg_arch.arch, kl_bg_arch.div)]);
                res_ex
                    .kl_tg_range_to_arch
                    .entry(range.clone())
                    .and_modify(|e| e.push((kl_tg_arch.arch.clone(), kl_tg_arch.div)))
                    .or_insert(vec![(kl_tg_arch.arch.clone(), kl_tg_arch.div)]);
            }
        }

        res_ex
    }
}

fn detect_code(corpus_stats: &[CorpusStats], file_data: &[u8], filename: &str) -> DetectionResult {
    // Heuristic depending on file size, the number is actually half the window
    // size.
    let window = match file_data.len() {
        0x100001..=0x1000000 => 0x1000, // 257 - 4096, 1MiB - 16MiB
        0x20001..=0x100000 => 0x800,    // 65 - 512, 128KiB - 1MiB
        0x8001..=0x20000 => 0x400,      // 33 - 128, 32KiB - 128KiB
        0x1001..=0x8000 => 0x200,       // 9 - 64, 4KiB - 32KiB
        0..=0x1000 => 0x100,            // 1 - 16, 0B - 4KiB
        // From here on we grow the number of windows logarithmically in the
        // file size. Constant factor ensures smooth transition.
        l => (l / (170 * ((l as f64).log2() as usize))) & 0xFFFFF000,
    };

    info!("{}: window_size : 0x{:x} ", filename, window * 2);

    let res_ex: DetectionResult = (0..file_data.len())
        .into_par_iter()
        .step_by(window)
        .map(|start| {
            let end = min(file_data.len(), start + window * 2);

            let win_stats = CorpusStats::new("target".to_string(), &file_data[start..end], 0.0);

            let range_res = calculate_kl(corpus_stats, &win_stats);

            (start..end, range_res)
        })
        .into();

    res_ex
}

fn main() -> Result<()> {
    let app = clap::Command::new("coderec")
        .version(env!("CARGO_PKG_VERSION"))
        .propagate_version(true)
        .author("Valentin Obst <coderec@vpao.io>")
        .about("Identifies machine code in binary files.")
        .arg(arg!(-d - -debug))
        .arg(arg!(-v - -verbose))
        .arg(arg!(--"big-file" "Optimized analysis for files larger than X00MiB."))
        .arg(arg!(--"plot-corpus" "Plot distributions of samples in corpus and exit."))
        .arg(arg!(--"plot-divs" "Plot raw analysis results in addition to region plot."))
        .arg(
            Arg::new("files")
                .action(ArgAction::Append)
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .required_unless_present("plot-corpus"),
        );

    let args = app.get_matches();

    let level = if args.get_flag("debug") {
        log::Level::Debug
    } else if args.get_flag("verbose") {
        log::Level::Info
    } else {
        log::Level::Warn
    };
    simple_logger::init_with_level(level)?;

    let big_file = args.get_flag("big-file");

    let corpus_stats = load_corpus();

    if args.get_flag("plot-corpus") {
        for arch in corpus_stats.iter() {
            arch.plot_tg();
            arch.plot_cond_prob();
        }

        return Ok(());
    }

    info!("Corpus size: {}", corpus_stats.len());

    for file in args.get_many::<String>("files").unwrap() {
        let file_data = std::fs::read(file).with_context(|| format!("Could not open {}", file))?;

        let raw_res = detect_code(&corpus_stats, &file_data, file);
        let processes_res: ProcessedDetectionResult = raw_res.into();

        if args.get_flag("plot-divs") {
            crate::plotting::plot_divs(file, file_data.len(), &processes_res);
        }

        crate::plotting::plot_regions(file, file_data.len(), &file_data, &processes_res, big_file);
    }

    Ok(())
}
