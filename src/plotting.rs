/*
    Copyright 2024 - Valentin Obst

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

use crate::{CorpusStats, ProcessedDetectionResult, RangeResult};

use itertools::Itertools;
use log::info;
use plotters::coord::combinators::IntoLogRange;
use plotters::prelude::full_palette::{GREY, ORANGE};
use plotters::prelude::*;

const RESOLUTION_3D: (u32, u32) = (3000, 3000);
const MARGIN_3D: u32 = 100;
const LABEL_AREA_3D: u32 = 200;
const CAPTION_STYLE_3D: (&str, u32, FontStyle, &RGBColor) =
    ("Calibri", 80, FontStyle::Normal, &BLACK);
const LABEL_STYLE_3D: (&str, u32, FontStyle, &RGBColor) =
    ("Calibri", 30, FontStyle::Normal, &BLACK);
const CAPTION_STYLE_2D: (&str, u32, FontStyle, &RGBColor) =
    ("sans-serif", 80, FontStyle::Normal, &BLACK);
const LABEL_STYLE_2D: (&str, u32, FontStyle, &RGBColor) =
    ("Calibri", 12, FontStyle::Normal, &BLACK);

impl CorpusStats {
    pub fn plot_tg(&self) {
        let plot_name = format!("{}_tg.svg", self.arch);

        let drawing_area = SVGBackend::new(&plot_name, RESOLUTION_3D).into_drawing_area();
        drawing_area.fill(&WHITE).unwrap();

        let mut chart_builder = ChartBuilder::on(&drawing_area);
        chart_builder
            .margin(MARGIN_3D)
            .set_all_label_area_size(LABEL_AREA_3D)
            .caption(
                format!("{}, trigrams", self.arch),
                CAPTION_STYLE_3D.into_text_style(&drawing_area),
            );

        let mut chart_context = chart_builder
            .build_cartesian_3d(0..256, 0..256, 0..256)
            .unwrap();

        let binding = |coord: (i32, i32, i32, f64), size, _style| {
            let style = match coord.3 {
                0.0..0.000001 => GREY,
                0.000001..0.000005 => ORANGE,
                0.000005..0.000010 => RED,
                0.000010..0.000015 => GREEN,
                _ => BLUE,
            };
            EmptyElement::at((coord.0, coord.1, coord.2)) + Circle::new((0, 0), size, style)
        };
        let tg_ser = PointSeries::of_element(
            (0u8..=255u8)
                .cartesian_product(0u8..=255u8)
                .cartesian_product(0..255u8)
                .filter_map(|tg| {
                    let tg = (tg.0 .0, tg.0 .1, tg.1);
                    self.trigrams_freq
                        .get(&tg)
                        .map(|tg_freq| (tg.0 as i32, tg.1 as i32, tg.2 as i32, *tg_freq))
                }),
            5,
            BLUE,
            &binding,
        );
        chart_context.draw_series(tg_ser).unwrap();

        chart_context
            .configure_axes()
            .tick_size(15)
            .x_max_light_lines(10)
            .y_max_light_lines(10)
            .z_max_light_lines(10)
            .label_style(LABEL_STYLE_3D.into_text_style(&drawing_area))
            .x_labels(20)
            .y_labels(20)
            .z_labels(20)
            .draw()
            .unwrap();
    }

    pub fn plot_cond_prob(&self) {
        let plot_name = format!("{}_cond_prob.svg", self.arch);
        let drawing_area = SVGBackend::new(&plot_name, RESOLUTION_3D).into_drawing_area();
        drawing_area.fill(&WHITE).unwrap();

        let mut chart_builder = ChartBuilder::on(&drawing_area);
        chart_builder
            .margin(100)
            .set_all_label_area_size(200)
            .caption(
                format!("{}, 2 byte cond. prob.", self.arch),
                ("Calibri", 80, FontStyle::Normal, &BLACK).into_text_style(&drawing_area),
            );

        let cond_prob_ser = PointSeries::of_element(
            (0u8..=255u8).cartesian_product(0u8..=255u8).map(|bg| {
                if let Some(bg_freq) = self.bigrams_freq.get(&bg) {
                    let cond_prob = bg_freq / self.ungrams_freq.get(&bg.0).unwrap();

                    Circle::new((bg.0 as i32, cond_prob, bg.1 as i32), 3, BLUE)
                } else if self.ungrams_freq.contains_key(&bg.0) {
                    Circle::new((bg.0 as i32, 0.0, bg.1 as i32), 2, ORANGE)
                } else {
                    Circle::new((bg.0 as i32, 0.0, bg.1 as i32), 2, BLACK)
                }
            }),
            5,
            BLUE,
            &|c, _s, _st| c,
        );

        let mut chart_context = chart_builder
            .build_cartesian_3d(0..256, (0.0..1.0).log_scale(), 0..256)
            .unwrap();
        chart_context.draw_series(cond_prob_ser).unwrap();
        chart_context
            .configure_axes()
            .tick_size(15)
            .x_max_light_lines(10)
            .y_max_light_lines(20)
            .z_max_light_lines(10)
            .label_style(("Calibri", 30, FontStyle::Normal, &BLACK).into_text_style(&drawing_area))
            .x_labels(20)
            .y_labels(40)
            .z_labels(20)
            .draw()
            .unwrap();
    }
}

fn arch_idx_to_color(arch_idx: usize) -> RGBAColor {
    RGBAColor::from(RGBColor(
        arch_idx.wrapping_mul(1337) as u8,
        arch_idx.wrapping_mul(9976) as u8,
        arch_idx.wrapping_mul(13) as u8,
    ))
}

pub fn plot_regions(
    file_name: &str,
    file_len: usize,
    file_bytes: &[u8],
    det_res: &ProcessedDetectionResult,
    big_file: bool,
) {
    let win_sz = det_res.win_sz;
    let arch_to_idx = &det_res.arch_to_idx;
    let arch_to_best_map = &det_res.arch_to_final_ranges;

    let file_name = file_name.split("/").last().unwrap();
    let plot_name = format!("{}_w{}_regions.bmp", file_name, win_sz);

    let root = BitMapBackend::new(&plot_name, (5000, 500)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&root)
        .caption(format!("{}, regions", file_name), CAPTION_STYLE_2D)
        .margin(5)
        .top_x_label_area_size(40)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .right_y_label_area_size(40)
        .build_cartesian_2d(0..file_len, 0..256)
        .unwrap();

    let binding = |coord: (usize, i32), size, style| {
        EmptyElement::at(coord) + Circle::new((0, 0), size, style)
    };
    for (arch, ranges) in arch_to_best_map.iter() {
        let arch_idx = *arch_to_idx.get(arch).unwrap();
        let style = arch_idx_to_color(arch_idx);

        if !big_file {
            let arch_ranges_bytes_ser = PointSeries::of_element(
                ranges
                    .iter()
                    .flat_map(|range| range.clone())
                    .map(|offset| (offset, file_bytes[offset] as i32)),
                2,
                style,
                &binding,
            );
            chart
                .draw_series(arch_ranges_bytes_ser)
                .unwrap()
                .label(arch)
                .legend(move |(x, y)| Rectangle::new([(x - 10, y + 10), (x, y)], style.filled()));
        } else {
            chart
                .draw_series(ranges.iter().flat_map(|range| {
                    // Encode information about the absolute divergence of the
                    // closest arch in bi- and trigrams. Also highlight cases
                    // where bi- and trigrams disagreed.
                    const MAX_DIV_BEST_BG: f64 = 10.0;
                    const MAX_DIV_BEST_TG: f64 = 10.0;

                    let style_bg = if arch == &det_res.range_to_result_bg.get(range).unwrap().arch {
                        style
                    } else {
                        RGBAColor::from(GREY)
                    };
                    let style_tg = if arch == &det_res.range_to_result_tg.get(range).unwrap().arch {
                        style
                    } else {
                        RGBAColor::from(GREY)
                    };

                    let mut range_res_bg = (12.8
                        * (MAX_DIV_BEST_BG
                            - det_res.range_to_result_bg.get(range).unwrap().div.floor()))
                        as i32;
                    let mut range_res_tg = 256 - (12.8
                        * (MAX_DIV_BEST_TG
                            - det_res.range_to_result_tg.get(range).unwrap().div.floor()))
                        as i32;

                    if range_res_bg < 0 {
                        range_res_bg = 1;
                    }
                    if range_res_tg < 0 {
                        range_res_tg = 254;
                    }

                    [
                        Rectangle::new(
                            [(range.start, 0), (range.end, range_res_bg)],
                            style_bg.filled(),
                        ),
                        Rectangle::new(
                            [(range.start, range_res_tg), (range.end, 255)],
                            style_tg.filled(),
                        ),
                    ].into_iter()
                }))
                .unwrap()
                .label(arch)
                .legend(move |(x, y)| Rectangle::new([(x - 10, y + 10), (x, y)], style.filled()));
        }
    }
    if !big_file {
        let arch_ranges_bytes_ser = PointSeries::of_element(
            det_res
                .range_to_final_result
                .iter()
                .filter_map(|(range, arch_op)| match arch_op {
                    None => Some(range),
                    _ => None,
                })
                .flat_map(|range| range.clone())
                .map(|offset| (offset, file_bytes[offset] as i32)),
            2,
            GREY,
            &binding,
        );
        chart
            .draw_series(arch_ranges_bytes_ser)
            .unwrap()
            .label("unknown")
            .legend(move |(x, y)| Rectangle::new([(x - 10, y + 10), (x, y)], GREY.filled()));
    } else {
        chart
            .draw_series(
                det_res
                    .range_to_final_result
                    .iter()
                    .filter_map(|(range, arch_op)| match arch_op {
                        None => Some(range),
                        _ => None,
                    })
                    .map(|range| {
                        Rectangle::new([(range.start, 0), (range.end, 255)], GREY.filled())
                    }),
            )
            .unwrap()
            .label("unknown")
            .legend(move |(x, y)| Rectangle::new([(x - 10, y + 10), (x, y)], GREY.filled()));
    }

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .margin(20)
        .legend_area_size(5)
        .border_style(BLUE)
        .background_style(BLUE.mix(0.1))
        .label_font(LABEL_STYLE_2D)
        .draw()
        .unwrap();
    chart
        .configure_mesh()
        .x_labels(100)
        .y_labels(24)
        .max_light_lines(4)
        .x_label_formatter(&|offset| format!("{:x}", { *offset }))
        .y_label_formatter(&|offset| format!("{:x}", *offset as usize))
        .label_style(LABEL_STYLE_2D)
        .draw()
        .unwrap();

    root.present().unwrap();
}

pub fn plot_divs(file_name: &str, file_len: usize, det_res: &ProcessedDetectionResult) {
    let win_sz = det_res.win_sz;
    let max_kl_bg = det_res.max_kl_bg;
    let min_kl_bg = det_res.min_kl_bg;
    let max_kl_tg = det_res.max_kl_tg;
    let min_kl_tg = det_res.min_kl_tg;
    let range_to_result_bg = &det_res.range_to_result_bg;
    let range_to_result_tg = &det_res.range_to_result_tg;
    let arch_to_idx = &det_res.arch_to_idx;
    let idx_to_arch = &det_res.idx_to_arch;

    let file_name = file_name.split("/").last().unwrap();
    let plot_name_bg = format!("{}_w{}_bg.svg", file_name, win_sz);
    let plot_name_tg = format!("{}_w{}_tg.svg", file_name, win_sz);

    info!("Generating: {}, {}", plot_name_bg, plot_name_tg);

    let drawing_area_bg = SVGBackend::new(&plot_name_bg, RESOLUTION_3D).into_drawing_area();
    drawing_area_bg.fill(&WHITE).unwrap();
    let drawing_area_tg = SVGBackend::new(&plot_name_tg, RESOLUTION_3D).into_drawing_area();
    drawing_area_tg.fill(&WHITE).unwrap();

    let mut chart_builder_bg = ChartBuilder::on(&drawing_area_bg);
    chart_builder_bg
        .margin(100)
        .set_all_label_area_size(200)
        .caption(
            format!("{}, w{}, bigrams", file_name, win_sz),
            ("Calibri", 80, FontStyle::Normal, &BLACK).into_text_style(&drawing_area_bg),
        );
    let mut chart_builder_tg = ChartBuilder::on(&drawing_area_tg);
    chart_builder_tg
        .margin(100)
        .set_all_label_area_size(200)
        .caption(
            format!("{}, w{}, trigrams", file_name, win_sz),
            ("Calibri", 80, FontStyle::Normal, &BLACK).into_text_style(&drawing_area_bg),
        );

    let mut chart_context_bg = chart_builder_bg
        .build_cartesian_3d(
            0..det_res.kl_arch_to_range_bg.len(),
            (min_kl_bg..max_kl_bg).log_scale(),
            0.0..(file_len as f64),
        )
        .unwrap();
    let mut chart_context_tg = chart_builder_tg
        .build_cartesian_3d(
            0..det_res.kl_arch_to_range_tg.len(),
            (min_kl_tg..max_kl_tg).log_scale(),
            0.0..(file_len as f64),
        )
        .unwrap();

    /*
    chart_context_bg.with_projection(|mut p| {
            p.pitch = -0.5;
            p.into_matrix() // build the projection matrix
        });
    chart_context_tg.with_projection(|mut p| {
            p.pitch = -0.5;
            p.into_matrix() // build the projection matrix
        });
    */

    for ((arch_bg, res_bg), (arch_tg, res_tg)) in det_res
        .kl_arch_to_range_bg
        .iter()
        .zip(det_res.kl_arch_to_range_tg.iter())
    {
        let arch_idx_bg = *arch_to_idx.get(arch_bg).unwrap();
        let color_bg = arch_idx_to_color(arch_idx_bg);

        let arch_divs_ser_bg = LineSeries::new(
            res_bg.iter().map(|(range, div)| {
                (
                    arch_idx_bg,
                    *div,
                    (range.end as f64 + range.start as f64) / 2.0,
                )
            }),
            color_bg,
        );
        chart_context_bg
            .draw_series(arch_divs_ser_bg)
            .unwrap()
            .label(arch_bg.clone());

        let arch_idx_tg = *arch_to_idx.get(arch_tg).unwrap();
        let color_tg = arch_idx_to_color(arch_idx_tg);

        let arch_divs_ser_tg = LineSeries::new(
            res_tg.iter().map(|(range, div)| {
                (
                    arch_idx_tg,
                    *div,
                    (range.end as f64 + range.start as f64) / 2.0,
                )
            }),
            color_tg,
        );
        chart_context_tg
            .draw_series(arch_divs_ser_tg)
            .unwrap()
            .label(arch_tg.clone());
    }
    let binding = |coord: (usize, f64, f64), size, style| {
        EmptyElement::at(coord)
            + Circle::new((0, 0), size, style)
            + Text::new(
                if ((coord.2 as usize).next_multiple_of(win_sz) / win_sz) % 0x4 == 0 {
                    idx_to_arch.get(&coord.0).unwrap().to_string()
                } else {
                    String::from("")
                },
                (0, 15),
                ("sans-serif", 15),
            )
    };
    let best_in_range_ser_bg = PointSeries::of_element(
        range_to_result_bg
            .iter()
            .map(|(range, RangeResult { arch, div, .. })| {
                (
                    *arch_to_idx.get(arch).unwrap(),
                    *div,
                    (range.end as f64 + range.start as f64) / 2.0,
                )
            }),
        5,
        RED,
        &binding,
    );
    chart_context_bg.draw_series(best_in_range_ser_bg).unwrap();
    let best_in_range_ser_tg = PointSeries::of_element(
        range_to_result_tg
            .iter()
            .map(|(range, RangeResult { arch, div, .. })| {
                (
                    *arch_to_idx.get(arch).unwrap(),
                    *div,
                    (range.end as f64 + range.start as f64) / 2.0,
                )
            }),
        5,
        RED,
        &binding,
    );
    chart_context_tg.draw_series(best_in_range_ser_tg).unwrap();

    chart_context_bg
        .configure_axes()
        .z_formatter(&|offset| format!("{:x}", *offset as usize))
        .x_formatter(&|arch_idx| idx_to_arch.get(arch_idx).unwrap().to_owned())
        .tick_size(15)
        .x_max_light_lines(10)
        .y_max_light_lines(20)
        .z_max_light_lines(10)
        .label_style(LABEL_STYLE_3D.into_text_style(&drawing_area_bg))
        .x_labels(20)
        .y_labels(40)
        .z_labels(20)
        .draw()
        .unwrap();

    chart_context_tg
        .configure_axes()
        .z_formatter(&|offset| format!("{:x}", *offset as usize))
        .x_formatter(&|arch_idx| idx_to_arch.get(arch_idx).unwrap().to_owned())
        .tick_size(15)
        .x_max_light_lines(10)
        .y_max_light_lines(20)
        .z_max_light_lines(10)
        .label_style(LABEL_STYLE_3D.into_text_style(&drawing_area_bg))
        .x_labels(20)
        .y_labels(40)
        .z_labels(20)
        .draw()
        .unwrap();
}
