/*
    Copyright 2024 - Valentin Obst <coderec@vpao.io>

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
//! Command line JSON output.

use crate::{Arch, ProcessedDetectionResult};

use std::convert::From;
use std::ops::Range;

use itertools::Itertools;
use serde::Serialize;

/// Information that is printed to stdout for each analyzed file.
#[derive(Serialize)]
pub struct CliJsonOutput {
    /// Name of the analyzed file.
    file: String,
    /// Consolidated detection results.
    range_results: Vec<(Range<usize>, usize, Arch)>,
}

impl From<(&str, &ProcessedDetectionResult)> for CliJsonOutput {
    fn from((file, res): (&str, &ProcessedDetectionResult)) -> Self {
        let mut range_to_final_result: Vec<_> = res.range_to_final_result.iter().collect();
        range_to_final_result
            .sort_unstable_by(|(range_a, _), (range_b, _)| range_a.start.cmp(&range_b.start));
        let runs = range_to_final_result
            .iter()
            .chunk_by(|(_, arch_op)| (*arch_op).clone());

        CliJsonOutput {
            file: file.to_owned(),
            range_results: runs
                .into_iter()
                .filter_map(|(arch_op, mut ranges)| {
                    let first_range = ranges.next().unwrap().0.clone();
                    let last_range = match ranges.last() {
                        Some((range, _)) => (*range).clone(),
                        None => first_range.clone(),
                    };

                    arch_op.map(|arch| {
                        (
                            first_range.start..last_range.end,
                            last_range.end - first_range.start,
                            arch,
                        )
                    })
                })
                .collect(),
        }
    }
}
