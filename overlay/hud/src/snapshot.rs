pub const MAGIC: u32 = 0x4F42584D;
pub const VERSION: u32 = 11;
pub const MAX_POLY: usize = 1024;
pub const MAX_RIDERS: usize = 64;
pub const MAX_STANDINGS: usize = 40;
pub const MAX_SECTORS: usize = 3;
pub const NAME: usize = 32;
pub const TRACK_NAME: usize = 64;
pub const GUID: usize = 100;
pub const SERVER_NAME: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Point {
    pub x: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Rider {
    pub race_num: i32,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub track_pos: f32,
    pub crashed: i32,
    pub name: [u8; NAME],
}

impl Default for Rider {
    fn default() -> Self {
        Self {
            race_num: 0,
            x: 0.0,
            z: 0.0,
            yaw: 0.0,
            track_pos: 0.0,
            crashed: 0,
            name: [0; NAME],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Standing {
    pub race_num: i32,
    pub position: i32,
    pub state: i32,
    pub best_lap_ms: i32,
    pub num_laps: i32,
    pub gap_ms: i32,
    pub gap_laps: i32,
    pub pit: i32,
    pub penalty_ms: i32,
    pub crashed: i32,
    pub name: [u8; NAME],
    pub bike: [u8; NAME],
    pub last_lap_ms: i32,
    pub category: [u8; NAME],
}

impl Default for Standing {
    fn default() -> Self {
        Self {
            race_num: 0,
            position: 0,
            state: 0,
            best_lap_ms: 0,
            num_laps: 0,
            gap_ms: 0,
            gap_laps: 0,
            pit: 0,
            penalty_ms: 0,
            crashed: 0,
            name: [0; NAME],
            bike: [0; NAME],
            last_lap_ms: 0,
            category: [0; NAME],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Snapshot {
    pub magic: u32,
    pub version: u32,
    pub seq: u32,
    pub size: u32,
    pub tick_qpc: u64,
    pub local_race_num: i32,
    pub focus_race_num: i32,
    pub has_telemetry: i32,
    pub local_crashed: i32,
    pub local_x: f32,
    pub local_z: f32,
    pub local_vel_x: f32,
    pub local_vel_z: f32,
    pub local_yaw: f32,
    pub local_speed: f32,
    pub local_track_pos: f32,
    pub track_name: [u8; TRACK_NAME],
    pub track_length: f32,
    pub sf_meters: f32,
    pub poly_count: i32,
    pub poly: [Point; MAX_POLY],
    pub rider_count: i32,
    pub riders: [Rider; MAX_RIDERS],
    pub standing_count: i32,
    pub standings: [Standing; MAX_STANDINGS],
    pub map: Rect,
    pub standings_rect: Rect,
    pub relative: Rect,
    pub show_map: i32,
    pub show_standings: i32,
    pub show_relative: i32,
    pub standings_rows: i32,
    pub relative_count: i32,
    pub local_gear: i32,
    pub local_rpm: i32,
    pub engine_temp: f32,
    pub air_temp: f32,
    pub last_lap_ms: i32,
    pub current_lap_ms: i32,
    pub current_lap: i32,
    pub session_laps: i32,
    pub on_track: i32,
    pub max_rpm: i32,
    pub shift_rpm: i32,
    pub session_time_ms: i32,
    pub session_length: i32,
    pub best_lap_ms: i32,
    pub sector_count: i32,
    pub sector_last: i32,
    pub sector_cur: [i32; MAX_SECTORS],
    pub sector_last_lap: [i32; MAX_SECTORS],
    pub sector_best: [i32; MAX_SECTORS],
    pub sector_delta: [i32; MAX_SECTORS],
    pub sector_delta_valid: i32,
    pub session_kind: i32,
    pub session_state: i32,
    pub fuel: f32,
    pub max_fuel: f32,
    pub guid: [u8; GUID],
    pub server_name: [u8; SERVER_NAME],
    pub server_type: i32,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            magic: 0,
            version: 0,
            seq: 0,
            size: 0,
            tick_qpc: 0,
            local_race_num: -1,
            focus_race_num: -1,
            has_telemetry: 0,
            local_crashed: 0,
            local_x: 0.0,
            local_z: 0.0,
            local_vel_x: 0.0,
            local_vel_z: 0.0,
            local_yaw: 0.0,
            local_speed: 0.0,
            local_track_pos: 0.0,
            track_name: [0; TRACK_NAME],
            track_length: 0.0,
            sf_meters: 0.0,
            poly_count: 0,
            poly: [Point { x: 0.0, z: 0.0 }; MAX_POLY],
            rider_count: 0,
            riders: [Rider::default(); MAX_RIDERS],
            standing_count: 0,
            standings: [Standing::default(); MAX_STANDINGS],
            map: Rect {
                x: 0.775,
                y: 0.62,
                w: 0.21,
                h: 0.34,
            },
            standings_rect: Rect {
                x: 0.012,
                y: 0.03,
                w: 0.20,
                h: 0.42,
            },
            relative: Rect {
                x: 0.012,
                y: 0.62,
                w: 0.20,
                h: 0.33,
            },
            show_map: 1,
            show_standings: 1,
            show_relative: 1,
            standings_rows: 12,
            relative_count: 3,
            local_gear: 0,
            local_rpm: 0,
            engine_temp: 0.0,
            air_temp: 0.0,
            last_lap_ms: 0,
            current_lap_ms: 0,
            current_lap: 0,
            session_laps: 0,
            on_track: 0,
            max_rpm: 0,
            shift_rpm: 0,
            session_time_ms: 0,
            session_length: 0,
            best_lap_ms: 0,
            sector_count: MAX_SECTORS as i32,
            sector_last: -1,
            sector_cur: [0; MAX_SECTORS],
            sector_last_lap: [0; MAX_SECTORS],
            sector_best: [0; MAX_SECTORS],
            sector_delta: [0; MAX_SECTORS],
            sector_delta_valid: 0,
            session_kind: -1,
            session_state: -1,
            fuel: 0.0,
            max_fuel: 0.0,
            guid: [0; GUID],
            server_name: [0; SERVER_NAME],
            server_type: 0,
        }
    }
}

impl Snapshot {
    /// Short name of the bike you are on (standings row for `local_race_num`).
    pub fn local_bike(&self) -> String {
        let num = self.local_race_num;
        if num < 0 {
            return String::new();
        }
        let n = self.standing_count.clamp(0, MAX_STANDINGS as i32) as usize;
        self.standings[..n]
            .iter()
            .find(|r| r.race_num == num)
            .map(|r| cstr(&r.bike))
            .filter(|b| !b.is_empty())
            .unwrap_or_default()
    }
}

pub fn cstr(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let bytes = &buf[..end];
    let raw = if let Ok(s) = std::str::from_utf8(bytes) {
        s.to_string()
    } else {
        // MX Bikes names/bikes often arrive as Windows-1252, not UTF-8.
        bytes.iter().map(|&b| cp1252_char(b)).collect()
    };
    clean_display(&raw)
}

fn cp1252_char(b: u8) -> char {
    match b {
        0x80 => '€',
        0x82 => '‚',
        0x83 => 'ƒ',
        0x84 => '„',
        0x85 => '…',
        0x86 => '†',
        0x87 => '‡',
        0x88 => 'ˆ',
        0x89 => '‰',
        0x8A => 'Š',
        0x8B => '‹',
        0x8C => 'Œ',
        0x8E => 'Ž',
        0x91 => '‘',
        0x92 => '’',
        0x93 => '“',
        0x94 => '”',
        0x95 => '•',
        0x96 => '–',
        0x97 => '—',
        0x98 => '˜',
        0x99 => '™',
        0x9A => 'š',
        0x9B => '›',
        0x9C => 'œ',
        0x9E => 'ž',
        0x9F => 'Ÿ',
        _ => b as char,
    }
}

/// Drop controls / replacement / trademark glyphs the HUD fonts usually lack.
fn clean_display(s: &str) -> String {
    s.chars()
        .filter(|c| {
            *c == ' '
                || *c == '\t'
                || (!c.is_control()
                    && *c != '\u{FFFD}'
                    && *c != '™'
                    && *c != '®'
                    && *c != '©')
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn write_name(dest: &mut [u8], src: &str) {
    dest.fill(0);
    let bytes = src.as_bytes();
    let n = bytes.len().min(dest.len().saturating_sub(1));
    dest[..n].copy_from_slice(&bytes[..n]);
}

impl Snapshot {
    /// Race / replay payload is present. Menus with an empty snapshot are not.
    pub fn has_session_data(&self) -> bool {
        self.on_track != 0
            || self.has_telemetry != 0
            || self.standing_count > 0
            || self.rider_count > 0
    }

    /// Human dump of SHM scalars plus occupied riders / standings / poly samples.
    pub fn dump_text(&self) -> String {
        use std::fmt::Write;
        let mut o = String::new();
        let _ = writeln!(o, "=== MxboShmSnapshot v{} ===", self.version);
        if self.version > 0 && self.version < VERSION {
            let _ = writeln!(
                o,
                "plugin SHM v{} — session_kind/state are not in this plugin. Quit MX Bikes, run build.bat, start the game again.",
                self.version
            );
        }
        let _ = writeln!(
            o,
            "magic={:#x} seq={} size={} rust_size={} tick_qpc={}",
            self.magic,
            self.seq,
            self.size,
            std::mem::size_of::<Self>(),
            self.tick_qpc
        );
        let _ = writeln!(
            o,
            "local_race_num={} focus_race_num={} has_telemetry={} local_crashed={}",
            self.local_race_num, self.focus_race_num, self.has_telemetry, self.local_crashed
        );
        let _ = writeln!(
            o,
            "local_xz=({:.3},{:.3}) vel=({:.3},{:.3}) yaw={:.3} speed={:.2} track_pos={:.4}",
            self.local_x,
            self.local_z,
            self.local_vel_x,
            self.local_vel_z,
            self.local_yaw,
            self.local_speed,
            self.local_track_pos
        );
        let _ = writeln!(
            o,
            "track={:?} length={:.1} sf_meters={:.1}",
            cstr(&self.track_name),
            self.track_length,
            self.sf_meters
        );
        let _ = writeln!(
            o,
            "show map={} standings={} relative={} standings_rows={} relative_count={}",
            self.show_map, self.show_standings, self.show_relative, self.standings_rows, self.relative_count
        );
        let _ = writeln!(
            o,
            "rects map=({:.3},{:.3},{:.3},{:.3}) standings=({:.3},{:.3},{:.3},{:.3}) relative=({:.3},{:.3},{:.3},{:.3})",
            self.map.x, self.map.y, self.map.w, self.map.h,
            self.standings_rect.x, self.standings_rect.y, self.standings_rect.w, self.standings_rect.h,
            self.relative.x, self.relative.y, self.relative.w, self.relative.h
        );
        let _ = writeln!(
            o,
            "gear={} rpm={} max_rpm={} shift_rpm={} engine_temp={:.1} air_temp={:.1} fuel={:.2}/{:.2}",
            self.local_gear, self.local_rpm, self.max_rpm, self.shift_rpm, self.engine_temp, self.air_temp, self.fuel, self.max_fuel
        );
        let _ = writeln!(
            o,
            "laps current={} last_ms={} current_ms={} best_ms={} session_laps={} on_track={}",
            self.current_lap,
            self.last_lap_ms,
            self.current_lap_ms,
            self.best_lap_ms,
            self.session_laps,
            self.on_track
        );
        let _ = writeln!(
            o,
            "session_kind={} session_state={} session_time_ms={} session_length={}",
            self.session_kind, self.session_state, self.session_time_ms, self.session_length
        );
        let _ = writeln!(
            o,
            "guid={:?} server_name={:?} server_type={}",
            cstr(&self.guid),
            cstr(&self.server_name),
            self.server_type
        );
        if self.version > 0 && self.version < VERSION {
            let _ = writeln!(
                o,
                "(newer SHM fields stay at defaults until Holeshot-HUD.dlo is rebuilt and MX Bikes is restarted)"
            );
        }
        let _ = writeln!(
            o,
            "sectors count={} last={} cur={:?} last_lap={:?} best={:?} delta={:?} valid={}",
            self.sector_count,
            self.sector_last,
            self.sector_cur,
            self.sector_last_lap,
            self.sector_best,
            self.sector_delta,
            self.sector_delta_valid
        );

        let pn = self.poly_count.clamp(0, MAX_POLY as i32) as usize;
        let _ = writeln!(o, "poly_count={pn}");
        let poly_end = if pn <= 6 { pn } else { 3 };
        for i in 0..poly_end {
            let p = &self.poly[i];
            let _ = writeln!(o, "  poly[{i}]=({:.2},{:.2})", p.x, p.z);
        }
        if pn > 6 {
            let _ = writeln!(o, "  ...");
            for i in pn - 2..pn {
                let p = &self.poly[i];
                let _ = writeln!(o, "  poly[{i}]=({:.2},{:.2})", p.x, p.z);
            }
        }

        let rn = self.rider_count.clamp(0, MAX_RIDERS as i32) as usize;
        let _ = writeln!(o, "rider_count={rn}");
        for (i, r) in self.riders.iter().enumerate().take(rn) {
            let _ = writeln!(
                o,
                "  rider[{i}] #{} {:?} xz=({:.2},{:.2}) yaw={:.2} pos={:.4} crashed={}",
                r.race_num,
                cstr(&r.name),
                r.x,
                r.z,
                r.yaw,
                r.track_pos,
                r.crashed
            );
        }

        let sn = self.standing_count.clamp(0, MAX_STANDINGS as i32) as usize;
        let _ = writeln!(o, "standing_count={sn}");
        for (i, st) in self.standings.iter().enumerate().take(sn) {
            let _ = writeln!(
                o,
                "  stand[{i}] P{} #{} {:?} {} {} laps={} last={} best={} gap={} gap_laps={} pit={} pen={} crashed={} state={}",
                st.position,
                st.race_num,
                cstr(&st.name),
                cstr(&st.bike),
                cstr(&st.category),
                st.num_laps,
                st.last_lap_ms,
                st.best_lap_ms,
                st.gap_ms,
                st.gap_laps,
                st.pit,
                st.penalty_ms,
                st.crashed,
                st.state
            );
        }
        o
    }
}

#[cfg(test)]
#[path = "tests/snapshot.rs"]
mod tests;
