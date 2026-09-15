"""Print the faulting module from a Windows minidump (no extra packages)."""
import argparse
import struct
import sys
from pathlib import Path

ExceptionStream = 6
ModuleListStream = 4
Memory64ListStream = 9
MemoryListStream = 5


def rva(data: bytes, off: int, fmt: str):
    return struct.unpack_from(fmt, data, off)


def u32(data, off):
    return rva(data, off, "<I")[0]


def u64(data, off):
    return rva(data, off, "<Q")[0]


def utf16z(data: bytes, off: int) -> str:
    if off < 0 or off + 4 > len(data):
        return "?"
    n = u32(data, off)
    if n < 0 or off + 4 + n > len(data):
        return "?"
    raw = data[off + 4 : off + 4 + n]
    return raw.decode("utf-16-le", errors="replace").rstrip("\x00")


def parse(path: Path) -> int:
    data = path.read_bytes()
    if data[:4] != b"MDMP":
        print(f"not a minidump: {path}", file=sys.stderr)
        return 2
    nstreams, dir_rva = rva(data, 8, "<II")
    streams = {}
    for i in range(nstreams):
        st, sz, rv = rva(data, dir_rva + i * 12, "<III")
        streams[st] = (sz, rv)

    print(f"file {path.name}  {path.stat().st_size} bytes")
    if ExceptionStream not in streams:
        print("no exception stream")
        return 1
    _, ev = streams[ExceptionStream]
    tid = u32(data, ev)
    code = u32(data, ev + 8)
    addr = u64(data, ev + 8 + 16)
    print(f"thread {tid}  exception 0x{code:08x}  address 0x{addr:016x}")

    mods = []
    if ModuleListStream in streams:
        _, mv = streams[ModuleListStream]
        count = u32(data, mv)
        rec = 108
        for i in range(count):
            o = mv + 4 + i * rec
            base = u64(data, o)
            size = u32(data, o + 8)
            name = utf16z(data, u32(data, o + 20))
            mods.append((base, size, name))

    hit = None
    for base, size, name in mods:
        if base <= addr < base + size:
            hit = (base, size, name)
            break
    if hit:
        print(f"fault module {Path(hit[2]).name}  +0x{addr - hit[0]:x}  ({hit[2]})")
    else:
        print("fault address not in any listed module")

    print("modules:")
    for base, size, name in mods:
        mark = "  <==" if hit and name == hit[2] else ""
        print(f"  {Path(name).name:28}  0x{base:016x}  {size:9}{mark}")
    return 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dumps", nargs="+")
    args = ap.parse_args()
    rc = 0
    for raw in args.dumps:
        rc = max(rc, parse(Path(raw)))
        print()
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
