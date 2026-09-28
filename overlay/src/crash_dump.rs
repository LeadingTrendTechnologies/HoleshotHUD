//! Newest `crash-*.dmp` under `%LOCALAPPDATA%\Holeshot HUD\logs`, decoded at RIP.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use iced_x86::{Code, Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};

const EXCEPTION_STREAM: u32 = 6;
const MODULE_LIST_STREAM: u32 = 4;
const MEMORY_LIST_STREAM: u32 = 5;
const MEMORY64_LIST_STREAM: u32 = 9;
const CONTEXT_RIP: usize = 248;
const INSTRUCTIONS: usize = 12;
const BYTES_BEFORE: u64 = 16;
const DECODE_WINDOW: u64 = 180;

#[derive(Clone, Debug)]
pub(crate) struct CrashReport {
    pub empty: bool,
    pub name: String,
    pub summary: String,
    pub before_hex: String,
    pub missing_code: bool,
    pub unreadable: bool,
    pub instructions: Vec<String>,
}

impl CrashReport {
    fn empty() -> Self {
        Self {
            empty: true,
            name: String::new(),
            summary: String::new(),
            before_hex: String::new(),
            missing_code: false,
            unreadable: false,
            instructions: Vec::new(),
        }
    }

    /// Full report for the clipboard. Not the ellipsized lines drawn on screen.
    pub(crate) fn plain_text(&self) -> String {
        let mut lines = Vec::new();
        if !self.name.is_empty() {
            lines.push(self.name.clone());
        }
        if !self.summary.is_empty() {
            lines.push(self.summary.clone());
        }
        if self.unreadable {
            return lines.join("\n");
        }
        if self.missing_code {
            lines.push("Code bytes are not in this dump.".into());
            return lines.join("\n");
        }
        if !self.before_hex.is_empty() {
            lines.push(format!("before  {}", self.before_hex));
        }
        lines.extend(self.instructions.iter().cloned());
        lines.join("\n")
    }
}

struct Cache {
    path: PathBuf,
    modified: Option<SystemTime>,
    len: u64,
    txt_modified: Option<SystemTime>,
    txt_len: u64,
    copied: bool,
    report: CrashReport,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

pub(crate) fn latest_report() -> CrashReport {
    let Some(dir) = logs_dir() else {
        return CrashReport::empty();
    };
    let Some(path) = newest_crash_dump(&dir) else {
        return CrashReport::empty();
    };
    let meta = fs::metadata(&path).ok();
    let modified = meta.as_ref().and_then(|m| m.modified().ok());
    let len = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let txt_path = path.with_extension("txt");
    let txt_meta = fs::metadata(&txt_path).ok();
    let txt_modified = txt_meta.as_ref().and_then(|m| m.modified().ok());
    let txt_len = txt_meta.as_ref().map(|m| m.len()).unwrap_or(0);
    if let Ok(guard) = CACHE.lock() {
        if let Some(hit) = guard.as_ref() {
            if hit.path == path
                && hit.modified == modified
                && hit.len == len
                && hit.txt_modified == txt_modified
                && hit.txt_len == txt_len
            {
                return hit.report.clone();
            }
        }
    }
    let data = fs::read(&path).unwrap_or_default();
    let txt = fs::read_to_string(&txt_path).ok();
    let mut report = decode_crash(&data, txt.as_deref());
    report.name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("crash.dmp")
        .to_string();
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some(Cache {
            path,
            modified,
            len,
            txt_modified,
            txt_len,
            copied: false,
            report: report.clone(),
        });
    }
    report
}

/// Copy the newest report. `latest_copied` stays true until that dump file changes.
pub(crate) fn copy_latest() -> bool {
    let report = latest_report();
    if report.empty {
        return false;
    }
    if crate::feedback::set_clipboard_text(&report.plain_text()).is_err() {
        return false;
    }
    if let Ok(mut guard) = CACHE.lock() {
        if let Some(hit) = guard.as_mut() {
            hit.copied = true;
        }
    }
    true
}

pub(crate) fn latest_copied() -> bool {
    CACHE
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|hit| hit.copied))
        .unwrap_or(false)
}

fn logs_dir() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(local).join("Holeshot HUD").join("logs"))
}

pub(crate) fn newest_crash_dump(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(String, PathBuf)> = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("crash-") || !name.ends_with(".dmp") {
            continue;
        }
        let take = best.as_ref().map(|(n, _)| name > *n).unwrap_or(true);
        if take {
            best = Some((name, entry.path()));
        }
    }
    best.map(|(_, path)| path)
}

pub(crate) fn decode_crash(data: &[u8], txt: Option<&str>) -> CrashReport {
    let Some(streams) = directory(data) else {
        return unreadable();
    };
    let Some(fault) = fault_from_dump(data, &streams) else {
        return unreadable();
    };
    let summary = summary_line(txt, &fault);
    let Some((before, at_rip)) = code_at_rip(data, &streams, fault.rip) else {
        return CrashReport {
            empty: false,
            name: String::new(),
            summary,
            before_hex: String::new(),
            missing_code: true,
            unreadable: false,
            instructions: Vec::new(),
        };
    };
    CrashReport {
        empty: false,
        name: String::new(),
        summary,
        before_hex: hex_bytes(&before),
        missing_code: false,
        unreadable: false,
        instructions: disassemble(&at_rip, fault.rip),
    }
}

fn unreadable() -> CrashReport {
    CrashReport {
        empty: false,
        name: String::new(),
        summary: "Could not read this dump.".into(),
        before_hex: String::new(),
        missing_code: false,
        unreadable: true,
        instructions: Vec::new(),
    }
}

struct Stream {
    kind: u32,
    size: u32,
    rva: u32,
}

struct Fault {
    code: u32,
    rip: u64,
    module: String,
    rva: u64,
    has_module: bool,
}

fn directory(data: &[u8]) -> Option<Vec<Stream>> {
    if data.len() < 32 || &data[..4] != b"MDMP" {
        return None;
    }
    let count = u32_at(data, 8)? as usize;
    if count > 256 {
        return None;
    }
    let dir = u32_at(data, 12)? as usize;
    let mut streams = Vec::with_capacity(count);
    for i in 0..count {
        let off = dir + i * 12;
        streams.push(Stream {
            kind: u32_at(data, off)?,
            size: u32_at(data, off + 4)?,
            rva: u32_at(data, off + 8)?,
        });
    }
    Some(streams)
}

fn stream_bytes<'a>(data: &'a [u8], streams: &[Stream], kind: u32) -> Option<&'a [u8]> {
    let stream = streams.iter().find(|s| s.kind == kind)?;
    let start = stream.rva as usize;
    let end = start.checked_add(stream.size as usize)?;
    data.get(start..end)
}

fn fault_from_dump(data: &[u8], streams: &[Stream]) -> Option<Fault> {
    let ex = stream_bytes(data, streams, EXCEPTION_STREAM)?;
    let code = u32_at(ex, 8)?;
    let address = u64_at(ex, 24)?;
    let rip = context_rip(data, ex).unwrap_or(address);
    let (module, rva, has_module) = module_for(data, streams, rip);
    Some(Fault {
        code,
        rip,
        module,
        rva,
        has_module,
    })
}

fn context_rip(data: &[u8], exception: &[u8]) -> Option<u64> {
    let size = u32_at(exception, 160)? as usize;
    let rva = u32_at(exception, 164)? as usize;
    if size < CONTEXT_RIP + 8 {
        return None;
    }
    u64_at(data, rva + CONTEXT_RIP)
}

fn module_for(data: &[u8], streams: &[Stream], rip: u64) -> (String, u64, bool) {
    let Some(list) = stream_bytes(data, streams, MODULE_LIST_STREAM) else {
        return (String::new(), 0, false);
    };
    let Some(count) = u32_at(list, 0) else {
        return (String::new(), 0, false);
    };
    let count = count as usize;
    if count > 4096 {
        return (String::new(), 0, false);
    }
    for i in 0..count {
        let off = 4 + i * 108;
        let Some(base) = u64_at(list, off) else {
            break;
        };
        let Some(size) = u32_at(list, off + 8) else {
            break;
        };
        let end = base.saturating_add(size as u64);
        if rip < base || rip >= end {
            continue;
        }
        let name_rva = u32_at(list, off + 20).unwrap_or(0) as usize;
        let name = module_name(data, name_rva);
        return (name, rip - base, true);
    }
    (String::new(), 0, false)
}

fn module_name(data: &[u8], rva: usize) -> String {
    let Some(nbytes) = u32_at(data, rva) else {
        return String::new();
    };
    let start = rva + 4;
    let end = start.saturating_add(nbytes as usize).min(data.len());
    if end < start || end - start < 2 {
        return String::new();
    }
    let raw = &data[start..end];
    let mut units = Vec::with_capacity(raw.len() / 2);
    for chunk in raw.chunks_exact(2) {
        let unit = u16::from_le_bytes([chunk[0], chunk[1]]);
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    let text = String::from_utf16_lossy(&units);
    Path::new(&text)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(text.as_str())
        .to_string()
}

fn summary_line(txt: Option<&str>, fault: &Fault) -> String {
    let code = txt
        .and_then(|t| txt_field(t, "exception_code"))
        .unwrap_or_else(|| format!("0x{:08X}", fault.code));
    let module = txt
        .and_then(|t| txt_field(t, "fault_module"))
        .map(|p| {
            Path::new(&p)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(p.as_str())
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .or_else(|| fault.has_module.then(|| fault.module.clone()));
    let rva = txt
        .and_then(|t| txt_field(t, "fault_rva"))
        .or_else(|| fault.has_module.then(|| format!("0x{:X}", fault.rva)));
    let rip = txt
        .and_then(|t| txt_field(t, "rip"))
        .unwrap_or_else(|| format!("0x{:016X}", fault.rip));
    let mut parts = vec![code];
    match (module, rva) {
        (Some(module), Some(rva)) => parts.push(format!("{module}+{rva}")),
        (Some(module), None) => parts.push(module),
        _ => {}
    }
    parts.push(format!("rip {rip}"));
    parts.join("  ")
}

fn txt_field(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(&prefix) else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() || rest == "(unknown)" {
            return None;
        }
        return Some(rest.to_string());
    }
    None
}

fn code_at_rip(data: &[u8], streams: &[Stream], rip: u64) -> Option<(Vec<u8>, Vec<u8>)> {
    if let Some(list) = stream_bytes(data, streams, MEMORY_LIST_STREAM) {
        if let Some(window) = memory_list_window(data, list, rip) {
            return Some(window);
        }
    }
    if let Some(list) = stream_bytes(data, streams, MEMORY64_LIST_STREAM) {
        if let Some(window) = memory64_window(data, list, rip) {
            return Some(window);
        }
    }
    None
}

fn memory_list_window(data: &[u8], list: &[u8], rip: u64) -> Option<(Vec<u8>, Vec<u8>)> {
    let count = u32_at(list, 0)? as usize;
    if count > 100_000 {
        return None;
    }
    for i in 0..count {
        let off = 4 + i * 16;
        let start = u64_at(list, off)?;
        let size = u32_at(list, off + 8)? as u64;
        let rva = u32_at(list, off + 12)? as u64;
        if size == 0 {
            continue;
        }
        let end = start.saturating_add(size);
        if rip >= start && rip < end {
            return slice_window(data, rva, start, end, rip);
        }
    }
    None
}

fn memory64_window(data: &[u8], list: &[u8], rip: u64) -> Option<(Vec<u8>, Vec<u8>)> {
    let count = u64_at(list, 0)?;
    if count > 100_000 {
        return None;
    }
    let mut cursor = u64_at(list, 8)?;
    for i in 0..count as usize {
        let off = 16 + i * 16;
        let start = u64_at(list, off)?;
        let size = u64_at(list, off + 8)?;
        let end = start.saturating_add(size);
        if size > 0 && rip >= start && rip < end {
            return slice_window(data, cursor, start, end, rip);
        }
        cursor = cursor.saturating_add(size);
    }
    None
}

fn slice_window(
    data: &[u8],
    file_base: u64,
    start: u64,
    end: u64,
    rip: u64,
) -> Option<(Vec<u8>, Vec<u8>)> {
    let from = rip.saturating_sub(BYTES_BEFORE).max(start);
    let to = (rip + DECODE_WINDOW).min(end);
    if to <= rip {
        return None;
    }
    let file_from = file_base + (from - start);
    let file_to = file_base + (to - start);
    let bytes = data.get(file_from as usize..file_to as usize)?;
    let before_len = (rip - from) as usize;
    if before_len > bytes.len() {
        return None;
    }
    let (before, at_rip) = bytes.split_at(before_len);
    if at_rip.is_empty() {
        return None;
    }
    Some((before.to_vec(), at_rip.to_vec()))
}

fn disassemble(bytes: &[u8], rip: u64) -> Vec<String> {
    let mut decoder = Decoder::with_ip(64, bytes, rip, DecoderOptions::NONE);
    let mut formatter = NasmFormatter::new();
    let mut instruction = Instruction::default();
    let mut text = String::new();
    let mut lines = Vec::new();
    for _ in 0..INSTRUCTIONS {
        if !decoder.can_decode() {
            break;
        }
        let ip = decoder.ip();
        decoder.decode_out(&mut instruction);
        if instruction.code() == Code::INVALID || instruction.len() == 0 {
            break;
        }
        text.clear();
        formatter.format(&instruction, &mut text);
        lines.push(format!("0x{ip:016X}  {text}"));
    }
    if lines.is_empty() && !bytes.is_empty() {
        let n = bytes.len().min(16);
        lines.push(format!("0x{rip:016X}  db {}", hex_bytes(&bytes[..n])));
    }
    lines
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn u32_at(data: &[u8], off: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(off..off + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn u64_at(data: &[u8], off: usize) -> Option<u64> {
    let bytes: [u8; 8] = data.get(off..off + 8)?.try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u32(buf: &mut [u8], off: usize, v: u32) {
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn put_u64(buf: &mut [u8], off: usize, v: u64) {
        buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }

    fn append_u32(buf: &mut Vec<u8>, v: u32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    /// `code` lives at `range_start`. `rip` is the faulting instruction address.
    fn synthetic_dump(range_start: u64, code: &[u8], rip: u64, memory64: bool) -> Vec<u8> {
        let mut buf = vec![0u8; 32 + 36];
        buf[0..4].copy_from_slice(b"MDMP");
        put_u32(&mut buf, 8, 3);
        put_u32(&mut buf, 12, 32);

        let ex_rva = buf.len() as u32;
        let ex_size = 168u32;
        buf.resize(buf.len() + ex_size as usize, 0);
        put_u32(&mut buf, ex_rva as usize + 8, 0xC000_0005);
        put_u64(&mut buf, ex_rva as usize + 24, rip);

        let mod_rva = buf.len() as u32;
        let mod_size = 4 + 108;
        buf.resize(buf.len() + mod_size, 0);
        put_u32(&mut buf, mod_rva as usize, 1);
        let rec = mod_rva as usize + 4;
        put_u64(&mut buf, rec, 0x1400_0000);
        put_u32(&mut buf, rec + 8, 0x20_0000);

        let mem_rva = buf.len() as u32;
        if memory64 {
            buf.resize(buf.len() + 32, 0);
            put_u64(&mut buf, mem_rva as usize, 1);
            put_u64(&mut buf, mem_rva as usize + 16, range_start);
            put_u64(&mut buf, mem_rva as usize + 24, code.len() as u64);
        } else {
            buf.resize(buf.len() + 20, 0);
            put_u32(&mut buf, mem_rva as usize, 1);
            put_u64(&mut buf, mem_rva as usize + 4, range_start);
            put_u32(&mut buf, mem_rva as usize + 12, code.len() as u32);
        }

        let ctx_rva = buf.len() as u32;
        buf.resize(buf.len() + 1232, 0);
        put_u64(&mut buf, ctx_rva as usize + CONTEXT_RIP, rip);
        put_u32(&mut buf, ex_rva as usize + 160, 1232);
        put_u32(&mut buf, ex_rva as usize + 164, ctx_rva);

        let name_rva = buf.len() as u32;
        let name = "mxbikes.exe";
        let wide: Vec<u16> = name.encode_utf16().collect();
        append_u32(&mut buf, (wide.len() * 2) as u32);
        for unit in wide {
            buf.extend_from_slice(&unit.to_le_bytes());
        }
        buf.extend_from_slice(&0u16.to_le_bytes());
        put_u32(&mut buf, rec + 20, name_rva);

        let code_rva = buf.len() as u32;
        buf.extend_from_slice(code);
        if memory64 {
            put_u64(&mut buf, mem_rva as usize + 8, code_rva as u64);
        } else {
            put_u32(&mut buf, mem_rva as usize + 16, code_rva);
        }

        let mem_kind = if memory64 {
            MEMORY64_LIST_STREAM
        } else {
            MEMORY_LIST_STREAM
        };
        let mem_size = if memory64 { 32 } else { 20 };
        let entries = [
            (EXCEPTION_STREAM, ex_size, ex_rva),
            (MODULE_LIST_STREAM, mod_size as u32, mod_rva),
            (mem_kind, mem_size, mem_rva),
        ];
        for (i, (kind, size, rva)) in entries.into_iter().enumerate() {
            let off = 32 + i * 12;
            put_u32(&mut buf, off, kind);
            put_u32(&mut buf, off + 4, size);
            put_u32(&mut buf, off + 8, rva);
        }
        buf
    }

    #[test]
    fn disassembles_rip_from_code_segment() {
        let code = [0xCC, 0xCC, 0xCC, 0xCC, 0x48, 0x89, 0xC8, 0x90, 0xC3];
        let dump = synthetic_dump(0x1400_0FFC, &code, 0x1400_1000, false);
        let report = decode_crash(&dump, None);
        assert!(!report.missing_code);
        assert!(report.before_hex.contains("CC CC CC CC"));
        assert!(report.summary.contains("0xC0000005"));
        assert!(report.summary.contains("mxbikes.exe+0x1000"));
        let joined = report.instructions.join("\n");
        assert!(joined.contains("mov"));
        assert!(joined.contains("nop"));
        assert!(joined.contains("ret"));
        assert!(report.instructions[0].starts_with("0x0000000014001000"));
        let text = report.plain_text();
        assert!(text.contains("before  CC CC CC CC"));
        assert!(text.contains("mov"));
        assert!(!text.contains('…'));
    }

    #[test]
    fn memory64_code_segment_disassembles() {
        let code = [0x90, 0xC3];
        let dump = synthetic_dump(0x1400_1000, &code, 0x1400_1000, true);
        let report = decode_crash(&dump, None);
        assert!(!report.missing_code);
        assert!(report.before_hex.is_empty());
        let joined = report.instructions.join("\n");
        assert!(joined.contains("nop"));
        assert!(joined.contains("ret"));
    }

    #[test]
    fn missing_code_bytes_says_so() {
        let dump = synthetic_dump(0x1400_2000, &[0x90], 0x1400_1000, false);
        let report = decode_crash(&dump, None);
        assert!(report.missing_code);
        assert!(report.instructions.is_empty());
        assert!(report.summary.contains("mxbikes.exe"));
        assert!(report.plain_text().contains("Code bytes are not in this dump."));
    }

    #[test]
    fn txt_summary_is_preferred() {
        let dump = synthetic_dump(
            0x1400_0FFC,
            &[0xCC, 0xCC, 0xCC, 0xCC, 0x90],
            0x1400_1000,
            false,
        );
        let txt = "\
exception_code=0xC0000005
fault_module=D:\\Games\\mxbikes.exe
fault_rva=0x1000
rip=0x0000000140001000
";
        let report = decode_crash(&dump, Some(txt));
        assert_eq!(
            report.summary,
            "0xC0000005  mxbikes.exe+0x1000  rip 0x0000000140001000"
        );
    }

    #[test]
    fn newest_crash_name_wins() {
        let dir = std::env::temp_dir().join(format!("mxbo-crash-dump-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("crash-20200101-000000.dmp"), b"MDMP").unwrap();
        fs::write(dir.join("crash-20200102-010203.dmp"), b"MDMP").unwrap();
        fs::write(dir.join("notes.txt"), b"nope").unwrap();
        let found = newest_crash_dump(&dir).unwrap();
        assert_eq!(
            found.file_name().unwrap().to_str().unwrap(),
            "crash-20200102-010203.dmp"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn garbage_is_unreadable() {
        let report = decode_crash(b"not a dump", None);
        assert!(report.unreadable);
        assert_eq!(report.summary, "Could not read this dump.");
        assert_eq!(report.plain_text(), "Could not read this dump.");
    }
}
