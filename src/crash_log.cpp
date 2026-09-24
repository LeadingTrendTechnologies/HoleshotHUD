#include "crash_log.h"
#include "state.h"

#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <dbghelp.h>
#include <tlhelp32.h>

#include <cstdarg>
#include <cstdio>
#include <cstring>

namespace crash_log
{
namespace
{
    constexpr int kRing = 32;
    constexpr int kNameLen = 24;
    constexpr int kTeleFlushEvery = 25;
    constexpr int kMaxStackFrames = 16;

    struct Crumb
    {
        DWORD tickMs = 0;
        char name[kNameLen]{};
    };

    Crumb g_ring[kRing]{};
    int g_ringWrite = 0;
    int g_ringCount = 0;
    int g_teleSinceFlush = 0;
    DWORD g_tick0 = 0;
    volatile LONG g_inCrash = 0;
    const PluginState* g_state = nullptr;
    LPTOP_LEVEL_EXCEPTION_FILTER g_prevFilter = nullptr;

    using MiniDumpWriteDumpFn = BOOL(WINAPI*)(
        HANDLE,
        DWORD,
        HANDLE,
        MINIDUMP_TYPE,
        PMINIDUMP_EXCEPTION_INFORMATION,
        PMINIDUMP_USER_STREAM_INFORMATION,
        PMINIDUMP_CALLBACK_INFORMATION);
    using SymInitializeFn = BOOL(WINAPI*)(HANDLE, PCSTR, BOOL);
    using StackWalk64Fn = BOOL(WINAPI*)(
        DWORD,
        HANDLE,
        HANDLE,
        LPSTACKFRAME64,
        PVOID,
        PREAD_PROCESS_MEMORY_ROUTINE64,
        PFUNCTION_TABLE_ACCESS_ROUTINE64,
        PGET_MODULE_BASE_ROUTINE64,
        PTRANSLATE_ADDRESS_ROUTINE64);
    using SymFunctionTableAccess64Fn = PVOID(WINAPI*)(HANDLE, DWORD64);
    using SymGetModuleBase64Fn = DWORD64(WINAPI*)(HANDLE, DWORD64);

    struct DbgHelp
    {
        HMODULE mod = nullptr;
        MiniDumpWriteDumpFn miniDump = nullptr;
        SymInitializeFn symInit = nullptr;
        StackWalk64Fn stackWalk = nullptr;
        SymFunctionTableAccess64Fn fnTable = nullptr;
        SymGetModuleBase64Fn modBase = nullptr;
        bool tried = false;
    };

    DbgHelp g_dbg{};

    bool logsDir(wchar_t* out, size_t cap)
    {
        if (!out || cap < 8)
        {
            return false;
        }
        wchar_t local[MAX_PATH]{};
        const DWORD n = GetEnvironmentVariableW(L"LOCALAPPDATA", local, MAX_PATH);
        if (n == 0 || n >= MAX_PATH)
        {
            return false;
        }
        // LOCALAPPDATA\Holeshot HUD\logs
        if (_snwprintf_s(out, cap, _TRUNCATE, L"%s\\Holeshot HUD", local) < 0)
        {
            return false;
        }
        CreateDirectoryW(out, nullptr);
        if (_snwprintf_s(out, cap, _TRUNCATE, L"%s\\Holeshot HUD\\logs", local) < 0)
        {
            return false;
        }
        CreateDirectoryW(out, nullptr);
        return true;
    }

    void ensureDbgHelp()
    {
        if (g_dbg.tried)
        {
            return;
        }
        g_dbg.tried = true;
        g_dbg.mod = LoadLibraryW(L"dbghelp.dll");
        if (!g_dbg.mod)
        {
            return;
        }
        g_dbg.miniDump = reinterpret_cast<MiniDumpWriteDumpFn>(
            GetProcAddress(g_dbg.mod, "MiniDumpWriteDump"));
        g_dbg.symInit = reinterpret_cast<SymInitializeFn>(
            GetProcAddress(g_dbg.mod, "SymInitialize"));
        g_dbg.stackWalk = reinterpret_cast<StackWalk64Fn>(
            GetProcAddress(g_dbg.mod, "StackWalk64"));
        g_dbg.fnTable = reinterpret_cast<SymFunctionTableAccess64Fn>(
            GetProcAddress(g_dbg.mod, "SymFunctionTableAccess64"));
        g_dbg.modBase = reinterpret_cast<SymGetModuleBase64Fn>(
            GetProcAddress(g_dbg.mod, "SymGetModuleBase64"));
    }

    void writeAll(HANDLE file, const char* text)
    {
        if (!file || file == INVALID_HANDLE_VALUE || !text)
        {
            return;
        }
        const DWORD len = static_cast<DWORD>(std::strlen(text));
        DWORD written = 0;
        WriteFile(file, text, len, &written, nullptr);
    }

    void appendf(HANDLE file, char* buf, size_t cap, const char* fmt, ...)
    {
        va_list ap;
        va_start(ap, fmt);
        const int n = _vsnprintf_s(buf, cap, _TRUNCATE, fmt, ap);
        va_end(ap);
        if (n > 0)
        {
            writeAll(file, buf);
        }
    }

    void resolveModule(ULONG_PTR addr, char* pathOut, size_t pathCap, DWORD64* rvaOut)
    {
        if (pathOut && pathCap)
        {
            pathOut[0] = '\0';
        }
        if (rvaOut)
        {
            *rvaOut = 0;
        }
        HMODULE mod = nullptr;
        if (!GetModuleHandleExA(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                reinterpret_cast<LPCSTR>(addr),
                &mod) ||
            !mod)
        {
            return;
        }
        if (pathOut && pathCap)
        {
            GetModuleFileNameA(mod, pathOut, static_cast<DWORD>(pathCap));
        }
        if (rvaOut)
        {
            *rvaOut = static_cast<DWORD64>(addr) - reinterpret_cast<DWORD64>(mod);
        }
    }

    void writeBreadcrumbs(HANDLE file, char* line, size_t lineCap)
    {
        writeAll(file, "breadcrumbs (oldest first):\n");
        const int count = g_ringCount < kRing ? g_ringCount : kRing;
        if (count <= 0)
        {
            writeAll(file, "  (none)\n");
            return;
        }
        const int start = (g_ringWrite - count + kRing) % kRing;
        for (int i = 0; i < count; ++i)
        {
            const Crumb& c = g_ring[(start + i) % kRing];
            appendf(
                file,
                line,
                lineCap,
                "  +%lums  %s\n",
                static_cast<unsigned long>(c.tickMs - g_tick0),
                c.name);
        }
    }

    void writeSession(HANDLE file, char* line, size_t lineCap)
    {
        writeAll(file, "session:\n");
        if (!g_state)
        {
            writeAll(file, "  (no state pointer)\n");
            return;
        }
        const PluginState& s = *g_state;
        appendf(
            file,
            line,
            lineCap,
            "  uptime_s=%.1f\n  track=%s\n  server=%s\n  guid=%s\n  rider=%s\n  riders=%zu\n"
            "  on_track=%d\n  session_kind=%d\n  session_state=%d\n",
            pluginNowSeconds(),
            s.trackName().empty() ? "(none)" : s.trackName().c_str(),
            s.serverName().empty() ? "(none)" : s.serverName().c_str(),
            s.eventGuid().empty() ? "(none)" : s.eventGuid().c_str(),
            s.localName().empty() ? "(none)" : s.localName().c_str(),
            s.entries().size(),
            s.onTrack() ? 1 : 0,
            s.sessionKind(),
            s.sessionState());
    }

    void writeStack(HANDLE file, CONTEXT* ctx, char* line, size_t lineCap)
    {
        writeAll(file, "stack:\n");
        ensureDbgHelp();
        if (!g_dbg.stackWalk || !g_dbg.fnTable || !g_dbg.modBase || !ctx)
        {
            writeAll(file, "  (dbghelp StackWalk64 unavailable)\n");
            return;
        }
        HANDLE proc = GetCurrentProcess();
        HANDLE thread = GetCurrentThread();
        if (g_dbg.symInit)
        {
            g_dbg.symInit(proc, nullptr, TRUE);
        }
        CONTEXT walkCtx = *ctx;
        STACKFRAME64 frame{};
        frame.AddrPC.Offset = walkCtx.Rip;
        frame.AddrPC.Mode = AddrModeFlat;
        frame.AddrFrame.Offset = walkCtx.Rbp;
        frame.AddrFrame.Mode = AddrModeFlat;
        frame.AddrStack.Offset = walkCtx.Rsp;
        frame.AddrStack.Mode = AddrModeFlat;

        for (int i = 0; i < kMaxStackFrames; ++i)
        {
            if (!g_dbg.stackWalk(
                    IMAGE_FILE_MACHINE_AMD64,
                    proc,
                    thread,
                    &frame,
                    &walkCtx,
                    nullptr,
                    g_dbg.fnTable,
                    g_dbg.modBase,
                    nullptr))
            {
                break;
            }
            if (frame.AddrPC.Offset == 0)
            {
                break;
            }
            char modPath[MAX_PATH]{};
            DWORD64 rva = 0;
            resolveModule(static_cast<ULONG_PTR>(frame.AddrPC.Offset), modPath, sizeof(modPath), &rva);
            const char* base = modPath;
            if (const char* slash = std::strrchr(modPath, '\\'))
            {
                base = slash + 1;
            }
            appendf(
                file,
                line,
                lineCap,
                "  #%d  %s+0x%llX\n",
                i,
                modPath[0] ? base : "?",
                static_cast<unsigned long long>(rva));
        }
    }

    bool pathHas(const char* path, const char* needle)
    {
        if (!path || !needle || !needle[0])
        {
            return false;
        }
        const size_t nlen = std::strlen(needle);
        for (const char* p = path; *p; ++p)
        {
            size_t i = 0;
            while (i < nlen)
            {
                const char a = p[i];
                const char b = needle[i];
                if (!a)
                {
                    return false;
                }
                const char al = (a >= 'A' && a <= 'Z') ? static_cast<char>(a - 'A' + 'a') : a;
                const char bl = (b >= 'A' && b <= 'Z') ? static_cast<char>(b - 'A' + 'a') : b;
                if (al != bl)
                {
                    break;
                }
                ++i;
            }
            if (i == nlen)
            {
                return true;
            }
        }
        return false;
    }

    bool isSystemModule(const char* path)
    {
        return pathHas(path, "\\windows\\system32\\") || pathHas(path, "\\windows\\syswow64\\") ||
               pathHas(path, "\\windows\\winsxs\\") || pathHas(path, "\\windows\\systemapps\\");
    }

    // Plugins, overlays, and injectors worth calling out after a crash.
    bool isSuspectModule(const char* path)
    {
        if (!path || !path[0])
        {
            return false;
        }
        if (pathHas(path, "\\plugins\\"))
        {
            return true;
        }
        static const char* kNeedles[] = {
            "frostmod",
            "holeshot",
            "mxbmrp",
            "mxbo",
            "reshade",
            "reshade64",
            "reshade32",
            "gameoverlayrenderer",
            "steamoverlay",
            "rtsshooks",
            "rivatuner",
            "specialk",
            "d3d9.dll", // often ReShade/injector rename beside the exe
            "dxgi.dll", // ReShade/overlay inject beside the exe (not System32)
            "opengl32.dll", // same — local inject copy
            "nvspcap",
            "nvoglv",
            "igd10",
            "bandicam",
            "obs-virtualcam",
            "graphics-hook",
            "discord_hook",
            "discordhook",
            "overwolf",
            "medal-hook",
            "mirillis",
            "plays.tv",
            "xinput", // game plugins\\xinput64.dli and proxies
            "hidpp_forcefeedback",
            "lghub",
            "logitech",
        };
        for (const char* n : kNeedles)
        {
            if (pathHas(path, n))
            {
                // Skip stock System32 copies of dxgi/d3d9/opengl/xinput — not injectors.
                if (isSystemModule(path) &&
                    (pathHas(path, "dxgi.dll") || pathHas(path, "d3d9.dll") ||
                     pathHas(path, "opengl32.dll") || pathHas(path, "xinput")))
                {
                    continue;
                }
                return true;
            }
        }
        return false;
    }

    void writeLoadedModules(HANDLE file, char* line, size_t lineCap, bool fullNonSystem)
    {
        writeAll(file, "loaded_suspects (plugins / overlays / injectors):\n");
        const HANDLE snap = CreateToolhelp32Snapshot(
            TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32,
            GetCurrentProcessId());
        if (snap == INVALID_HANDLE_VALUE)
        {
            writeAll(file, "  (module snapshot failed)\n");
            return;
        }

        MODULEENTRY32 me{};
        me.dwSize = sizeof(me);
        int suspects = 0;
        int others = 0;
        char otherBuf[48][MAX_PATH]{};

        if (Module32First(snap, &me))
        {
            do
            {
                const char* path = me.szExePath;
                if (!path[0])
                {
                    path = me.szModule;
                }
                if (isSuspectModule(path))
                {
                    appendf(
                        file,
                        line,
                        lineCap,
                        "  %s  base=0x%p\n",
                        path,
                        me.modBaseAddr);
                    ++suspects;
                }
                else if (fullNonSystem && !isSystemModule(path) && others < 48)
                {
                    std::strncpy(otherBuf[others], path, MAX_PATH - 1);
                    otherBuf[others][MAX_PATH - 1] = '\0';
                    ++others;
                }
            } while (Module32Next(snap, &me));
        }
        CloseHandle(snap);

        if (suspects == 0)
        {
            writeAll(file, "  (none matched)\n");
        }

        if (fullNonSystem)
        {
            writeAll(file, "loaded_other_non_system:\n");
            if (others == 0)
            {
                writeAll(file, "  (none)\n");
            }
            else
            {
                for (int i = 0; i < others; ++i)
                {
                    appendf(file, line, lineCap, "  %s\n", otherBuf[i]);
                }
                if (others >= 48)
                {
                    writeAll(file, "  ...(truncated)\n");
                }
            }
        }
    }

    void writeMinidump(const wchar_t* dumpPath, EXCEPTION_POINTERS* info)
    {
        ensureDbgHelp();
        if (!g_dbg.miniDump || !dumpPath)
        {
            return;
        }
        const HANDLE file = CreateFileW(
            dumpPath,
            GENERIC_WRITE,
            FILE_SHARE_READ,
            nullptr,
            CREATE_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            nullptr);
        if (file == INVALID_HANDLE_VALUE)
        {
            return;
        }
        MINIDUMP_EXCEPTION_INFORMATION mei{};
        mei.ThreadId = GetCurrentThreadId();
        mei.ExceptionPointers = info;
        mei.ClientPointers = FALSE;
        g_dbg.miniDump(
            GetCurrentProcess(),
            GetCurrentProcessId(),
            file,
            MiniDumpNormal,
            info ? &mei : nullptr,
            nullptr,
            nullptr);
        CloseHandle(file);
    }

    void stampName(wchar_t* stamp, size_t cap)
    {
        SYSTEMTIME st{};
        GetLocalTime(&st);
        _snwprintf_s(
            stamp,
            cap,
            _TRUNCATE,
            L"%04u%02u%02u-%02u%02u%02u",
            st.wYear,
            st.wMonth,
            st.wDay,
            st.wHour,
            st.wMinute,
            st.wSecond);
    }

    LONG WINAPI onUnhandled(EXCEPTION_POINTERS* info)
    {
        if (InterlockedCompareExchange(&g_inCrash, 1, 0) != 0)
        {
            return EXCEPTION_CONTINUE_SEARCH;
        }

        wchar_t dir[MAX_PATH]{};
        wchar_t stamp[32]{};
        wchar_t txtPath[MAX_PATH]{};
        wchar_t dmpPath[MAX_PATH]{};
        if (!logsDir(dir, MAX_PATH))
        {
            return EXCEPTION_CONTINUE_SEARCH;
        }
        stampName(stamp, 32);
        _snwprintf_s(txtPath, MAX_PATH, _TRUNCATE, L"%s\\crash-%s.txt", dir, stamp);
        _snwprintf_s(dmpPath, MAX_PATH, _TRUNCATE, L"%s\\crash-%s.dmp", dir, stamp);

        const HANDLE file = CreateFileW(
            txtPath,
            GENERIC_WRITE,
            FILE_SHARE_READ,
            nullptr,
            CREATE_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            nullptr);
        if (file != INVALID_HANDLE_VALUE)
        {
            char line[512]{};
            writeAll(file, "Holeshot-HUD crash report\n");

            const EXCEPTION_RECORD* er = info ? info->ExceptionRecord : nullptr;
            const CONTEXT* ctx = info ? info->ContextRecord : nullptr;
            const DWORD code = er ? er->ExceptionCode : 0;
            const ULONG_PTR fault =
                er && er->NumberParameters >= 2 ? er->ExceptionInformation[1] : 0;
            const ULONG_PTR pc = ctx ? static_cast<ULONG_PTR>(ctx->Rip) : 0;

            char modPath[MAX_PATH]{};
            DWORD64 rva = 0;
            resolveModule(pc, modPath, sizeof(modPath), &rva);

            appendf(file, line, sizeof(line), "exception_code=0x%08lX\n", static_cast<unsigned long>(code));
            appendf(
                file,
                line,
                sizeof(line),
                "fault_address=0x%p\n",
                reinterpret_cast<void*>(fault));
            appendf(file, line, sizeof(line), "rip=0x%p\n", reinterpret_cast<void*>(pc));
            appendf(
                file,
                line,
                sizeof(line),
                "fault_module=%s\n",
                modPath[0] ? modPath : "(unknown)");
            appendf(file, line, sizeof(line), "fault_rva=0x%llX\n", static_cast<unsigned long long>(rva));

            if (ctx)
            {
                appendf(
                    file,
                    line,
                    sizeof(line),
                    "regs:\n"
                    "  rip=%016llX rsp=%016llX rbp=%016llX\n"
                    "  rax=%016llX rbx=%016llX rcx=%016llX rdx=%016llX\n"
                    "  rsi=%016llX rdi=%016llX r8=%016llX r9=%016llX\n",
                    static_cast<unsigned long long>(ctx->Rip),
                    static_cast<unsigned long long>(ctx->Rsp),
                    static_cast<unsigned long long>(ctx->Rbp),
                    static_cast<unsigned long long>(ctx->Rax),
                    static_cast<unsigned long long>(ctx->Rbx),
                    static_cast<unsigned long long>(ctx->Rcx),
                    static_cast<unsigned long long>(ctx->Rdx),
                    static_cast<unsigned long long>(ctx->Rsi),
                    static_cast<unsigned long long>(ctx->Rdi),
                    static_cast<unsigned long long>(ctx->R8),
                    static_cast<unsigned long long>(ctx->R9));
            }

            writeStack(file, info ? info->ContextRecord : nullptr, line, sizeof(line));
            writeBreadcrumbs(file, line, sizeof(line));
            writeSession(file, line, sizeof(line));
            writeLoadedModules(file, line, sizeof(line), true);
            writeAll(file, "end of report\n");
            CloseHandle(file);
        }

        writeMinidump(dmpPath, info);
        flushTrail();

        return EXCEPTION_CONTINUE_SEARCH;
    }
} // namespace

void install(const PluginState* state)
{
    g_state = state;
    if (!g_prevFilter)
    {
        g_prevFilter = SetUnhandledExceptionFilter(onUnhandled);
    }
}

void breadcrumb(const char* name)
{
    if (!name || !name[0])
    {
        return;
    }
    const DWORD now = GetTickCount();
    if (g_ringCount == 0)
    {
        g_tick0 = now;
    }
    Crumb& c = g_ring[g_ringWrite];
    c.tickMs = now;
    std::strncpy(c.name, name, kNameLen - 1);
    c.name[kNameLen - 1] = '\0';
    g_ringWrite = (g_ringWrite + 1) % kRing;
    if (g_ringCount < kRing)
    {
        ++g_ringCount;
    }
}

void flushTrail()
{
    wchar_t dir[MAX_PATH]{};
    if (!logsDir(dir, MAX_PATH))
    {
        return;
    }
    wchar_t path[MAX_PATH]{};
    _snwprintf_s(path, MAX_PATH, _TRUNCATE, L"%s\\last-trail.txt", dir);
    const HANDLE file = CreateFileW(
        path,
        GENERIC_WRITE,
        FILE_SHARE_READ,
        nullptr,
        CREATE_ALWAYS,
        FILE_ATTRIBUTE_NORMAL,
        nullptr);
    if (file == INVALID_HANDLE_VALUE)
    {
        return;
    }
    char line[256]{};
    writeAll(file, "Holeshot-HUD last-trail\n");
    writeBreadcrumbs(file, line, sizeof(line));
    writeSession(file, line, sizeof(line));
    writeLoadedModules(file, line, sizeof(line), false);
    CloseHandle(file);
    g_teleSinceFlush = 0;
}

void noteTelemetry()
{
    if (++g_teleSinceFlush >= kTeleFlushEvery)
    {
        flushTrail();
    }
}
} // namespace crash_log
