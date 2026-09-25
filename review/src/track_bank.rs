//! Persistent per-track ideal line bank (smooth merge of clean Motos laps).

#![allow(dead_code)] // merge path is wired from the sqlite feature

use mxbo_hud::location_tape::{ChannelBin, BINS};

pub const IDEAL_BINS: usize = BINS;

/// Min contiguous faster bins before geometry switches onto the new lap.
const MIN_RUN: usize = 6;
/// Edge lerp width at run boundaries.
const BLEND: usize = 4;
/// Reject geometry that would jump farther than this (metres) vs neighbors.
const JUMP_M: f32 = 7.0;
/// Max lateral metres off track centerline before a sample is rejected.
const ON_TRACK_M: f32 = 12.0;
/// Clamp stored ideal onto the dirt ribbon (tighter than reject gate).
const SNAP_M: f32 = 7.0;
/// Only break ideal densify for huge teleports (Metres²).
const IDEAL_BREAK_M2: f32 = 80.0 * 80.0;
/// Fraction of a run that must pass on-track to accept the run.
const RUN_ON_TRACK_FRAC: f32 = 0.7;
/// New segment must beat bank by this fraction or by MIN_GAIN_MS.
const HYST_FRAC: f32 = 0.03;
const MIN_GAIN_MS: i32 = 25;
/// Spatial smooth half-window (samples each side).
const SMOOTH_R: usize = 2;

#[derive(Clone, Debug)]
pub struct TrackBankRow {
    pub track: String,
    pub best_ms: i32,
    pub ideal_ms: i32,
    pub merged_laps: i32,
    pub updated: i64,
}

#[derive(Clone, Debug)]
pub struct TrackBankDetail {
    pub track: String,
    pub best_ms: i32,
    pub ideal_ms: i32,
    pub merged_laps: i32,
    pub updated: i64,
    pub poly: Vec<(f32, f32)>,
    pub sf_meters: f32,
    pub best_line: Vec<(f32, f32, f32)>,
    pub ideal_line: Vec<(f32, f32, f32)>,
}

#[derive(Clone, Debug)]
pub struct IdealBank {
    pub best_ms: i32,
    pub best_line: Vec<(f32, f32, f32)>,
    pub best_bins: [ChannelBin; IDEAL_BINS],
    pub ideal_ms: i32,
    pub ideal_xyz: [(f32, f32, f32); IDEAL_BINS],
    pub ideal_seg_ms: [i32; IDEAL_BINS],
    pub poly: Vec<(f32, f32)>,
    pub sf_meters: f32,
    pub merged_laps: i32,
    pub updated: i64,
}

impl IdealBank {
    pub fn empty() -> Self {
        Self {
            best_ms: 0,
            best_line: Vec::new(),
            best_bins: [ChannelBin::default(); IDEAL_BINS],
            ideal_ms: 0,
            ideal_xyz: [(f32::NAN, f32::NAN, f32::NAN); IDEAL_BINS],
            ideal_seg_ms: [0; IDEAL_BINS],
            poly: Vec::new(),
            sf_meters: -1.0,
            merged_laps: 0,
            updated: 0,
        }
    }

    pub fn has_ideal(&self) -> bool {
        self.ideal_seg_ms.iter().any(|&s| s > 0)
            && self
                .ideal_xyz
                .iter()
                .any(|(x, _, z)| x.is_finite() && z.is_finite())
    }
}

pub fn segment_costs(bins: &[ChannelBin; IDEAL_BINS]) -> [i32; IDEAL_BINS] {
    let mut out = [0; IDEAL_BINS];
    let mut prev = 0;
    for i in 0..IDEAL_BINS {
        let ms = bins[i].ms;
        if ms <= 0 {
            out[i] = 0;
            continue;
        }
        let seg = if i == 0 || prev <= 0 {
            ms
        } else if ms > prev {
            ms - prev
        } else {
            0
        };
        out[i] = seg;
        prev = ms;
    }
    out
}

fn finite_xyz(p: (f32, f32, f32)) -> bool {
    p.0.is_finite() && p.1.is_finite() && p.2.is_finite()
}

fn xz_dist2(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dz = a.2 - b.2;
    dx * dx + dz * dz
}

fn lerp_xyz(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    let t = t.clamp(0.0, 1.0);
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

/// Continuous finite runs of a line (NaN gaps break runs).
fn line_runs(line: &[(f32, f32, f32)]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < line.len() {
        while i < line.len() && !finite_xyz(line[i]) {
            i += 1;
        }
        if i >= line.len() {
            break;
        }
        let start = i;
        while i < line.len() && finite_xyz(line[i]) {
            i += 1;
        }
        if i - start >= 2 {
            runs.push((start, i));
        }
    }
    runs
}

fn run_arc_len(line: &[(f32, f32, f32)], start: usize, end: usize) -> f32 {
    let mut total = 0.0_f32;
    for i in start + 1..end {
        total += xz_dist2(line[i - 1], line[i]).sqrt();
    }
    total
}

/// Sample at fraction `t` of total finite arc length without bridging NaN gaps.
/// Returns None if `t` lands inside a hitch hole (unmapped stretch).
pub fn sample_line_at(line: &[(f32, f32, f32)], t: f32) -> Option<(f32, f32, f32)> {
    let runs = line_runs(line);
    if runs.is_empty() {
        return line.iter().copied().find(|p| finite_xyz(*p));
    }
    let lens: Vec<f32> = runs
        .iter()
        .map(|&(a, b)| run_arc_len(line, a, b))
        .collect();
    let total: f32 = lens.iter().sum();
    if total <= 1e-3 {
        return Some(line[runs[0].0]);
    }
    let target = t.clamp(0.0, 1.0) * total;
    let mut acc = 0.0_f32;
    for (ri, &(start, end)) in runs.iter().enumerate() {
        let len = lens[ri];
        if acc + len + 1e-4 < target {
            acc += len;
            continue;
        }
        let local = (target - acc).clamp(0.0, len);
        let mut along = 0.0_f32;
        for i in start + 1..end {
            let seg = xz_dist2(line[i - 1], line[i]).sqrt();
            if along + seg + 1e-4 >= local {
                let u = if seg > 1e-4 {
                    ((local - along) / seg).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                return Some(lerp_xyz(line[i - 1], line[i], u));
            }
            along += seg;
        }
        return Some(line[end - 1]);
    }
    let (_, end) = runs[runs.len() - 1];
    Some(line[end - 1])
}

fn nearest_on_poly(poly: &[(f32, f32)], x: f32, z: f32) -> Option<(f32, f32, f32)> {
    if poly.len() < 2 {
        return None;
    }
    let mut best_d2 = f32::MAX;
    let mut best = (x, 0.0_f32, z);
    let n = poly.len();
    for i in 0..n {
        let (ax, az) = poly[i];
        let (bx, bz) = poly[(i + 1) % n];
        let abx = bx - ax;
        let abz = bz - az;
        let ab2 = abx * abx + abz * abz;
        let t = if ab2 > 1e-6 {
            ((x - ax) * abx + (z - az) * abz) / ab2
        } else {
            0.0
        }
        .clamp(0.0, 1.0);
        let px = ax + abx * t;
        let pz = az + abz * t;
        let d2 = (x - px) * (x - px) + (z - pz) * (z - pz);
        if d2 < best_d2 {
            best_d2 = d2;
            best = (px, 0.0, pz);
        }
    }
    Some(best)
}

fn lateral_d2_to_poly(poly: &[(f32, f32)], p: (f32, f32, f32)) -> f32 {
    let Some(n) = nearest_on_poly(poly, p.0, p.2) else {
        return 0.0;
    };
    xz_dist2(p, n)
}

fn on_track(poly: &[(f32, f32)], p: (f32, f32, f32)) -> bool {
    if poly.len() < 2 {
        return true;
    }
    lateral_d2_to_poly(poly, p) <= ON_TRACK_M * ON_TRACK_M
}

fn snap_to_poly(poly: &[(f32, f32)], p: (f32, f32, f32)) -> (f32, f32, f32) {
    if poly.len() < 2 {
        return p;
    }
    let Some(n) = nearest_on_poly(poly, p.0, p.2) else {
        return p;
    };
    let dx = p.0 - n.0;
    let dz = p.2 - n.2;
    let d = (dx * dx + dz * dz).sqrt();
    if d <= SNAP_M {
        return p;
    }
    let s = SNAP_M / d;
    (n.0 + dx * s, p.1, n.2 + dz * s)
}

fn faster_enough(new_seg: i32, bank_seg: i32) -> bool {
    if new_seg <= 0 {
        return false;
    }
    if bank_seg <= 0 {
        return true;
    }
    if new_seg >= bank_seg {
        return false;
    }
    let gain = bank_seg - new_seg;
    gain >= MIN_GAIN_MS || (gain as f32) >= bank_seg as f32 * HYST_FRAC
}

/// Moving-average smooth on finite samples; keeps NaN holes.
pub fn smooth_ideal_xyz(xyz: &mut [(f32, f32, f32); IDEAL_BINS]) {
    let src = *xyz;
    for i in 0..IDEAL_BINS {
        if !finite_xyz(src[i]) {
            continue;
        }
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sz = 0.0;
        let mut n = 0.0;
        let lo = i.saturating_sub(SMOOTH_R);
        let hi = (i + SMOOTH_R).min(IDEAL_BINS - 1);
        for j in lo..=hi {
            if !finite_xyz(src[j]) {
                continue;
            }
            if xz_dist2(src[i], src[j]) > (JUMP_M * 2.0) * (JUMP_M * 2.0) {
                continue;
            }
            sx += src[j].0;
            sy += src[j].1;
            sz += src[j].2;
            n += 1.0;
        }
        if n >= 1.0 {
            xyz[i] = (sx / n, sy / n, sz / n);
        }
    }
}

fn snap_ideal_to_poly(xyz: &mut [(f32, f32, f32); IDEAL_BINS], poly: &[(f32, f32)]) {
    if poly.len() < 2 {
        return;
    }
    for p in xyz.iter_mut() {
        if finite_xyz(*p) {
            *p = snap_to_poly(poly, *p);
        }
    }
}

/// Fill NaN bins by lerping between nearest finite neighbors (wraps for a closed lap).
pub fn fill_ideal_holes(xyz: &mut [(f32, f32, f32); IDEAL_BINS], poly: &[(f32, f32)]) {
    let src = *xyz;
    for i in 0..IDEAL_BINS {
        if finite_xyz(src[i]) {
            continue;
        }
        let mut prev = None;
        for k in 1..IDEAL_BINS {
            let j = (i + IDEAL_BINS - k) % IDEAL_BINS;
            if finite_xyz(src[j]) {
                prev = Some((k, src[j]));
                break;
            }
        }
        let mut next = None;
        for k in 1..IDEAL_BINS {
            let j = (i + k) % IDEAL_BINS;
            if finite_xyz(src[j]) {
                next = Some((k, src[j]));
                break;
            }
        }
        let Some((bp, p)) = prev else {
            continue;
        };
        let Some((bn, n)) = next else {
            xyz[i] = p;
            continue;
        };
        let t = bp as f32 / (bp + bn) as f32;
        xyz[i] = lerp_xyz(p, n, t);
    }
    snap_ideal_to_poly(xyz, poly);
}

/// Densify for draw: ordered finite samples; only break on huge teleports.
pub fn ideal_line_from_bank(bank: &IdealBank) -> Vec<(f32, f32, f32)> {
    let mut out = Vec::with_capacity(IDEAL_BINS + 8);
    let mut prev: Option<(f32, f32, f32)> = None;
    for p in &bank.ideal_xyz {
        if !finite_xyz(*p) {
            continue;
        }
        if let Some(q) = prev {
            if xz_dist2(q, *p) >= IDEAL_BREAK_M2 {
                out.push((f32::NAN, 0.0, f32::NAN));
            }
        }
        out.push(*p);
        prev = Some(*p);
    }
    out
}

/// Merge a clean you-lap into the bank. Returns true if anything changed.
pub fn merge_lap(
    bank: &mut IdealBank,
    lap_ms: i32,
    bins: &[ChannelBin; IDEAL_BINS],
    line: &[(f32, f32, f32)],
    poly: &[(f32, f32)],
    sf_meters: f32,
    now: i64,
) -> bool {
    if lap_ms <= 0 || line.len() < 8 {
        return false;
    }
    let new_seg = segment_costs(bins);
    if !new_seg.iter().any(|&s| s > 0) {
        return false;
    }

    let mut new_xyz = [(f32::NAN, f32::NAN, f32::NAN); IDEAL_BINS];
    let mut on_ok = [false; IDEAL_BINS];
    for i in 0..IDEAL_BINS {
        if new_seg[i] <= 0 {
            continue;
        }
        let t = (i as f32 + 0.5) / IDEAL_BINS as f32;
        let Some(p) = sample_line_at(line, t) else {
            continue;
        };
        if !on_track(poly, p) {
            continue;
        }
        new_xyz[i] = p;
        on_ok[i] = true;
    }

    let empty = !bank.has_ideal();
    let mut take = [false; IDEAL_BINS];
    if empty {
        for i in 0..IDEAL_BINS {
            take[i] = on_ok[i];
        }
    } else {
        let mut i = 0;
        while i < IDEAL_BINS {
            if !faster_enough(new_seg[i], bank.ideal_seg_ms[i]) || !on_ok[i] {
                i += 1;
                continue;
            }
            let start = i;
            while i < IDEAL_BINS
                && faster_enough(new_seg[i], bank.ideal_seg_ms[i])
                && on_ok[i]
            {
                i += 1;
            }
            let len = i - start;
            if len < MIN_RUN {
                continue;
            }
            let ok_count = (start..i).filter(|&j| on_ok[j]).count();
            if (ok_count as f32) < len as f32 * RUN_ON_TRACK_FRAC {
                continue;
            }
            let ok_start = start == 0
                || !finite_xyz(bank.ideal_xyz[start.saturating_sub(1)])
                || xz_dist2(new_xyz[start], bank.ideal_xyz[start.saturating_sub(1)])
                    <= JUMP_M * JUMP_M;
            let end = i - 1;
            let ok_end = end + 1 >= IDEAL_BINS
                || !finite_xyz(bank.ideal_xyz[end + 1])
                || xz_dist2(new_xyz[end], bank.ideal_xyz[end + 1]) <= JUMP_M * JUMP_M;
            if ok_start && ok_end {
                for j in start..i {
                    take[j] = true;
                }
            }
        }
    }

    if !take.iter().any(|&t| t) && !(lap_ms > 0 && (bank.best_ms == 0 || lap_ms < bank.best_ms)) {
        // Still allow best-only update when geometry run rejected.
        if lap_ms > 0 && (bank.best_ms == 0 || lap_ms < bank.best_ms) && line_on_track_enough(line, poly)
        {
            bank.best_ms = lap_ms;
            bank.best_line = line.to_vec();
            bank.best_bins = *bins;
            bank.updated = now;
            if should_replace_poly(bank.poly.len(), poly.len()) {
                bank.poly = poly.to_vec();
            }
            if sf_meters.is_finite() && sf_meters > 0.0 {
                bank.sf_meters = sf_meters;
            }
            let _ = fold_best_into_ideal(bank);
            return true;
        }
        return false;
    }

    let mut changed = false;

    for i in 0..IDEAL_BINS {
        if !take[i] {
            continue;
        }
        if new_seg[i] > 0 && (bank.ideal_seg_ms[i] <= 0 || new_seg[i] < bank.ideal_seg_ms[i]) {
            bank.ideal_seg_ms[i] = new_seg[i];
            changed = true;
        }
    }

    let mut next_xyz = bank.ideal_xyz;
    for i in 0..IDEAL_BINS {
        if !take[i] || !finite_xyz(new_xyz[i]) {
            continue;
        }
        let edge = run_edge_weight(&take, i);
        if edge < 1.0 && finite_xyz(bank.ideal_xyz[i]) {
            next_xyz[i] = lerp_xyz(bank.ideal_xyz[i], new_xyz[i], edge);
        } else {
            next_xyz[i] = new_xyz[i];
        }
        changed = true;
    }
    if changed {
        smooth_ideal_xyz(&mut next_xyz);
        let poly_ref = if poly.len() >= 2 { poly } else { &bank.poly };
        snap_ideal_to_poly(&mut next_xyz, poly_ref);
        fill_ideal_holes(&mut next_xyz, poly_ref);
        bank.ideal_xyz = next_xyz;
        bank.ideal_ms = bank.ideal_seg_ms.iter().filter(|&&s| s > 0).sum();
        bank.merged_laps = bank.merged_laps.saturating_add(1);
        bank.updated = now;
    }

    if lap_ms > 0
        && (bank.best_ms == 0 || lap_ms < bank.best_ms)
        && line_on_track_enough(line, poly)
    {
        bank.best_ms = lap_ms;
        bank.best_line = line.to_vec();
        bank.best_bins = *bins;
        bank.updated = now;
        changed = true;
    }

    if should_replace_poly(bank.poly.len(), poly.len()) {
        bank.poly = poly.to_vec();
        changed = true;
    }
    if sf_meters.is_finite() && sf_meters > 0.0 {
        bank.sf_meters = sf_meters;
    }

    if bank.best_ms > 0 && fold_best_into_ideal(bank) {
        changed = true;
    }

    changed
}

/// PB is a floor: fold best lap segments into ideal so Ideal is never slower than Best.
fn fold_best_into_ideal(bank: &mut IdealBank) -> bool {
    if bank.best_ms <= 0 {
        return false;
    }
    let best_seg = segment_costs(&bank.best_bins);
    let mut seg_changed = false;
    let mut xyz_changed = false;
    let mut next_xyz = bank.ideal_xyz;
    for i in 0..IDEAL_BINS {
        let b = best_seg[i];
        if b <= 0 {
            continue;
        }
        if bank.ideal_seg_ms[i] <= 0 || b < bank.ideal_seg_ms[i] {
            bank.ideal_seg_ms[i] = b;
            seg_changed = true;
            let t = (i as f32 + 0.5) / IDEAL_BINS as f32;
            if let Some(p) = sample_line_at(&bank.best_line, t) {
                if bank.poly.len() < 2 || on_track(&bank.poly, p) {
                    next_xyz[i] = p;
                    xyz_changed = true;
                }
            }
        }
    }
    if xyz_changed {
        smooth_ideal_xyz(&mut next_xyz);
        snap_ideal_to_poly(&mut next_xyz, &bank.poly);
        fill_ideal_holes(&mut next_xyz, &bank.poly);
        bank.ideal_xyz = next_xyz;
    }
    let mut ideal_ms: i32 = bank.ideal_seg_ms.iter().filter(|&&s| s > 0).sum();
    ideal_ms = ideal_ms.min(bank.best_ms);
    if ideal_ms != bank.ideal_ms || seg_changed || xyz_changed {
        bank.ideal_ms = ideal_ms;
        return true;
    }
    false
}

fn line_on_track_enough(line: &[(f32, f32, f32)], poly: &[(f32, f32)]) -> bool {
    if poly.len() < 2 {
        return true;
    }
    let mut ok = 0usize;
    let mut n = 0usize;
    for p in line.iter().copied().filter(|p| finite_xyz(*p)) {
        n += 1;
        if on_track(poly, p) {
            ok += 1;
        }
    }
    n > 0 && (ok as f32) >= n as f32 * RUN_ON_TRACK_FRAC
}

fn run_edge_weight(take: &[bool; IDEAL_BINS], i: usize) -> f32 {
    if BLEND == 0 || !take[i] {
        return 1.0;
    }
    let mut start = i;
    while start > 0 && take[start - 1] {
        start -= 1;
    }
    let mut end = i;
    while end + 1 < IDEAL_BINS && take[end + 1] {
        end += 1;
    }
    let from_start = i - start;
    let from_end = end - i;
    let mut w = 1.0_f32;
    if from_start < BLEND {
        w = w.min((from_start + 1) as f32 / (BLEND as f32 + 1.0));
    }
    if from_end < BLEND {
        w = w.min((from_end + 1) as f32 / (BLEND as f32 + 1.0));
    }
    w
}

fn should_replace_poly(have: usize, next: usize) -> bool {
    have < 2 || next > have
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bins_from_segs(segs: &[i32; IDEAL_BINS]) -> [ChannelBin; IDEAL_BINS] {
        let mut bins = [ChannelBin::default(); IDEAL_BINS];
        let mut acc = 0;
        for i in 0..IDEAL_BINS {
            if segs[i] > 0 {
                acc += segs[i];
                bins[i].ms = acc;
                bins[i].speed = 40.0;
            }
        }
        bins
    }

    fn oval_line(offset: f32, n: usize) -> Vec<(f32, f32, f32)> {
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let a = (i as f32 / n as f32) * std::f32::consts::TAU;
            v.push((
                a.cos() * (40.0 + offset),
                1.0,
                a.sin() * (28.0 + offset * 0.5),
            ));
        }
        v
    }

    fn poly() -> Vec<(f32, f32)> {
        oval_line(0.0, 64)
            .into_iter()
            .map(|(x, _, z)| (x, z))
            .collect()
    }

    #[test]
    fn complementary_halves_make_faster_ideal() {
        let mut bank = IdealBank::empty();
        let mut segs_a = [80; IDEAL_BINS];
        let mut segs_b = [80; IDEAL_BINS];
        for i in 0..IDEAL_BINS / 2 {
            segs_a[i] = 40;
            segs_b[i] = 100;
        }
        for i in IDEAL_BINS / 2..IDEAL_BINS {
            segs_a[i] = 100;
            segs_b[i] = 40;
        }
        let line_a = oval_line(2.0, 400);
        let line_b = oval_line(2.2, 400);
        let poly = poly();
        assert!(merge_lap(
            &mut bank,
            segs_a.iter().sum(),
            &bins_from_segs(&segs_a),
            &line_a,
            &poly,
            12.0,
            1,
        ));
        let best_a = bank.best_ms;
        assert!(merge_lap(
            &mut bank,
            segs_b.iter().sum(),
            &bins_from_segs(&segs_b),
            &line_b,
            &poly,
            12.0,
            2,
        ));
        assert!(bank.ideal_ms > 0);
        assert!(bank.ideal_ms < best_a);
        let line = ideal_line_from_bank(&bank);
        let finite: Vec<_> = line.iter().copied().filter(|p| finite_xyz(*p)).collect();
        assert!(finite.len() > IDEAL_BINS / 2);
        for w in finite.windows(2) {
            let d2 = xz_dist2(w[0], w[1]);
            assert!(
                d2 < (JUMP_M * 3.0) * (JUMP_M * 3.0) || d2 >= IDEAL_BREAK_M2,
                "choppy jump {}",
                d2.sqrt()
            );
        }
        for p in &finite {
            assert!(on_track(&poly, *p), "ideal off track");
        }
    }

    #[test]
    fn one_bin_spike_does_not_kink() {
        let mut bank = IdealBank::empty();
        let segs = [50; IDEAL_BINS];
        let line = oval_line(1.0, 300);
        let poly = poly();
        assert!(merge_lap(
            &mut bank,
            segs.iter().sum(),
            &bins_from_segs(&segs),
            &line,
            &poly,
            10.0,
            1,
        ));
        let before = bank.ideal_xyz;
        let mut spike = segs;
        spike[40] = 10;
        let mut far = line.clone();
        let mid = far.len() / 4;
        if let Some(p) = far.get_mut(mid) {
            p.0 += 30.0;
        }
        let _ = merge_lap(
            &mut bank,
            spike.iter().sum::<i32>() + 1000,
            &bins_from_segs(&spike),
            &far,
            &poly,
            10.0,
            2,
        );
        let d2 = xz_dist2(before[40], bank.ideal_xyz[40]);
        assert!(d2 < 1.0 || !finite_xyz(before[40]), "single spike moved geometry");
    }

    #[test]
    fn hitch_gap_does_not_bridge_off_track() {
        let mut bank = IdealBank::empty();
        let segs = [50; IDEAL_BINS];
        let mut line = oval_line(1.5, 300);
        let mid = line.len() / 2;
        line[mid] = (f32::NAN, 0.0, f32::NAN);
        // yank a point after the gap far off track
        if let Some(p) = line.get_mut(mid + 5) {
            *p = (0.0, 1.0, 0.0);
        }
        let poly = poly();
        assert!(merge_lap(
            &mut bank,
            segs.iter().sum(),
            &bins_from_segs(&segs),
            &line,
            &poly,
            10.0,
            1,
        ));
        for p in bank.ideal_xyz.iter().copied().filter(|p| finite_xyz(*p)) {
            assert!(on_track(&poly, p), "hitch pulled ideal off track");
        }
    }

    #[test]
    fn fill_ideal_holes_closes_nan_gap() {
        let poly = poly();
        let mut xyz = [(f32::NAN, f32::NAN, f32::NAN); IDEAL_BINS];
        xyz[10] = (40.0, 1.0, 0.0);
        xyz[20] = (0.0, 1.0, 28.0);
        fill_ideal_holes(&mut xyz, &poly);
        assert!(finite_xyz(xyz[15]), "mid gap should fill");
        assert!(on_track(&poly, xyz[15]));
    }

    #[test]
    fn fold_best_keeps_ideal_at_most_best() {
        let mut bank = IdealBank::empty();
        let poly = poly();
        let slow = [100; IDEAL_BINS];
        assert!(merge_lap(
            &mut bank,
            slow.iter().sum(),
            &bins_from_segs(&slow),
            &oval_line(1.0, 300),
            &poly,
            10.0,
            1,
        ));
        assert!(bank.ideal_ms > 0);
        // PB landed as best without beating ideal segments (stale bank).
        let fast = [60; IDEAL_BINS];
        bank.best_ms = fast.iter().sum();
        bank.best_bins = bins_from_segs(&fast);
        bank.best_line = oval_line(1.0, 300);
        assert!(
            bank.ideal_ms > bank.best_ms,
            "precondition: ideal slower than injected PB"
        );
        assert!(fold_best_into_ideal(&mut bank));
        assert!(
            bank.ideal_ms <= bank.best_ms,
            "ideal {} must not exceed best {}",
            bank.ideal_ms,
            bank.best_ms
        );
        assert_eq!(bank.ideal_ms, bank.best_ms);
    }

    #[test]
    fn ideal_line_from_bank_stays_continuous_under_break() {
        let mut bank = IdealBank::empty();
        for i in 0..IDEAL_BINS {
            let a = (i as f32 / IDEAL_BINS as f32) * std::f32::consts::TAU;
            bank.ideal_xyz[i] = (40.0 * a.cos(), 1.0, 28.0 * a.sin());
        }
        // Two neighbors ~15 m apart — under IDEAL_BREAK_M (80 m).
        bank.ideal_xyz[40] = (40.0, 1.0, 0.0);
        bank.ideal_xyz[41] = (25.0, 1.0, 0.0);
        let line = ideal_line_from_bank(&bank);
        assert!(
            line.iter().all(|p| finite_xyz(*p)),
            "jumps under break threshold must not insert NaN cuts"
        );
        assert_eq!(line.len(), IDEAL_BINS);
    }

    #[test]
    fn sample_line_at_ends() {
        let line = vec![(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)];
        let a = sample_line_at(&line, 0.0).unwrap();
        let b = sample_line_at(&line, 1.0).unwrap();
        let m = sample_line_at(&line, 0.5).unwrap();
        assert!((a.0 - 0.0).abs() < 0.01);
        assert!((b.0 - 10.0).abs() < 0.01);
        assert!((m.0 - 5.0).abs() < 0.01);
    }
}
