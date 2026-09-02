use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{HANDLE, NO_ERROR};
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY, MIB_TCPROW_OWNER_PID,
    MIB_TCP_STATE_ESTAB, TCP_TABLE_OWNER_PID_CONNECTIONS,
};
use windows::Win32::Networking::WinSock::AF_INET;

const FALLBACK: u32 = u32::from_ne_bytes([1, 1, 1, 1]);
const ICMP_TIMEOUT_MS: u32 = 600;

/// ICMP RTT in ms. `-1` until a reply lands.
pub struct PingWatch {
    ms: Arc<AtomicI32>,
    mx_pid: Arc<AtomicU32>,
    live: Arc<AtomicBool>,
}

impl PingWatch {
    pub fn new() -> Self {
        let ms = Arc::new(AtomicI32::new(-1));
        let mx_pid = Arc::new(AtomicU32::new(0));
        let live = Arc::new(AtomicBool::new(false));
        let ms_t = ms.clone();
        let pid_t = mx_pid.clone();
        let live_t = live.clone();
        let _ = thread::Builder::new()
            .name("holeshot-ping".into())
            .spawn(move || ping_loop(ms_t, pid_t, live_t));
        Self { ms, mx_pid, live }
    }

    pub fn set_live(&self, on: bool) {
        self.live.store(on, Ordering::Relaxed);
    }

    pub fn set_mx(&self, pid: Option<u32>) {
        self.mx_pid.store(pid.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn ms(&self) -> i32 {
        self.ms.load(Ordering::Relaxed)
    }
}

fn ping_loop(ms: Arc<AtomicI32>, mx_pid: Arc<AtomicU32>, live: Arc<AtomicBool>) {
    let Ok(icmp) = (unsafe { IcmpCreateFile() }) else {
        return;
    };
    if icmp.is_invalid() {
        return;
    }
    loop {
        if !live.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(200));
            continue;
        }
        let dest = mx_remote(mx_pid.load(Ordering::Relaxed)).unwrap_or(FALLBACK);
        if let Some(rtt) = icmp_ms(icmp, dest) {
            ms.store(rtt.min(9999), Ordering::Relaxed);
        }
        thread::sleep(Duration::from_secs(2));
    }
}

fn icmp_ms(icmp: HANDLE, dest: u32) -> Option<i32> {
    let req = [b'h', b's'];
    let mut reply = vec![0u8; size_of::<ICMP_ECHO_REPLY>() + req.len() + 8];
    let n = unsafe {
        IcmpSendEcho(
            icmp,
            dest,
            req.as_ptr().cast(),
            req.len() as u16,
            None,
            reply.as_mut_ptr().cast(),
            reply.len() as u32,
            ICMP_TIMEOUT_MS,
        )
    };
    if n == 0 {
        return None;
    }
    let echo = unsafe { &*(reply.as_ptr() as *const ICMP_ECHO_REPLY) };
    if echo.Status != 0 {
        return None;
    }
    Some(echo.RoundTripTime as i32)
}

fn mx_remote(pid: u32) -> Option<u32> {
    if pid == 0 {
        return None;
    }
    let ips = tcp_remotes(pid);
    pick_remote(&ips)
}

fn tcp_remotes(pid: u32) -> Vec<u32> {
    unsafe {
        let mut size = 0u32;
        let _ = GetExtendedTcpTable(
            None,
            &mut size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_CONNECTIONS,
            0,
        );
        if size == 0 {
            return Vec::new();
        }
        let mut buf = vec![0u8; size as usize];
        let st = GetExtendedTcpTable(
            Some(buf.as_mut_ptr().cast()),
            &mut size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_CONNECTIONS,
            0,
        );
        if st != NO_ERROR.0 {
            return Vec::new();
        }
        if buf.len() < 4 {
            return Vec::new();
        }
        let n = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        let row = size_of::<MIB_TCPROW_OWNER_PID>();
        let mut out = Vec::new();
        for i in 0..n {
            let off = 4 + i * row;
            if off + row > buf.len() {
                break;
            }
            let rec = &*(buf.as_ptr().add(off) as *const MIB_TCPROW_OWNER_PID);
            if rec.dwOwningPid != pid || rec.dwState != MIB_TCP_STATE_ESTAB.0 as u32 {
                continue;
            }
            if rec.dwRemoteAddr != 0 {
                out.push(rec.dwRemoteAddr);
            }
        }
        out
    }
}

/// Public IPv4 first, then any non-loopback. `ip` is Windows network-order.
pub(crate) fn pick_remote(ips: &[u32]) -> Option<u32> {
    ips.iter()
        .copied()
        .find(|ip| is_public(*ip))
        .or_else(|| ips.iter().copied().find(|ip| !is_unusable(*ip)))
}

fn octets(ip: u32) -> [u8; 4] {
    ip.to_ne_bytes()
}

fn is_unusable(ip: u32) -> bool {
    let [a, b, ..] = octets(ip);
    a == 0 || a == 127 || (a == 169 && b == 254) || a >= 224
}

fn is_public(ip: u32) -> bool {
    if is_unusable(ip) {
        return false;
    }
    let [a, b, ..] = octets(ip);
    if a == 10 {
        return false;
    }
    if a == 192 && b == 168 {
        return false;
    }
    if a == 172 && (16..32).contains(&b) {
        return false;
    }
    true
}

#[cfg(test)]
#[path = "tests/ping.rs"]
mod tests;
