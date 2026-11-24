#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif

#include "shm_writer.h"
#include "shm_publish.h"
#include "shm/mxbo_shm.h"
#include "config.h"
#include "state.h"

#include <windows.h>

#include <vector>

namespace
{

// Current user and SYSTEM. The overlay runs as that user, so it can still map the view.
struct MappingSecurity
{
    SECURITY_ATTRIBUTES attributes{};
    SECURITY_DESCRIPTOR descriptor{};
    std::vector<unsigned char> acl;
    bool ready = false;
};

bool buildMappingSecurity(MappingSecurity& security)
{
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token))
    {
        return false;
    }
    DWORD needed = 0;
    GetTokenInformation(token, TokenUser, nullptr, 0, &needed);
    std::vector<unsigned char> userInfo(needed == 0 ? 1 : needed);
    if (!GetTokenInformation(
            token,
            TokenUser,
            userInfo.data(),
            static_cast<DWORD>(userInfo.size()),
            &needed))
    {
        CloseHandle(token);
        return false;
    }
    CloseHandle(token);
    PSID userSid = reinterpret_cast<TOKEN_USER*>(userInfo.data())->User.Sid;

    SID_IDENTIFIER_AUTHORITY authority = SECURITY_NT_AUTHORITY;
    PSID systemSid = nullptr;
    if (!AllocateAndInitializeSid(
            &authority,
            1,
            SECURITY_LOCAL_SYSTEM_RID,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            &systemSid))
    {
        return false;
    }

    const DWORD aclBytes = sizeof(ACL)
        + 2 * (sizeof(ACCESS_ALLOWED_ACE) - sizeof(DWORD))
        + GetLengthSid(userSid)
        + GetLengthSid(systemSid);
    security.acl.resize(aclBytes);
    auto* acl = reinterpret_cast<PACL>(security.acl.data());
    const bool ok = InitializeAcl(acl, aclBytes, ACL_REVISION)
        && AddAccessAllowedAce(acl, ACL_REVISION, FILE_MAP_ALL_ACCESS, userSid)
        && AddAccessAllowedAce(acl, ACL_REVISION, FILE_MAP_ALL_ACCESS, systemSid)
        && InitializeSecurityDescriptor(&security.descriptor, SECURITY_DESCRIPTOR_REVISION)
        && SetSecurityDescriptorDacl(&security.descriptor, TRUE, acl, FALSE);
    FreeSid(systemSid);
    if (!ok)
    {
        security.acl.clear();
        return false;
    }
    security.attributes.nLength = sizeof(security.attributes);
    security.attributes.lpSecurityDescriptor = &security.descriptor;
    security.attributes.bInheritHandle = FALSE;
    security.ready = true;
    return true;
}

} // namespace

bool ShmWriter::open()
{
    close();
    MappingSecurity security;
    const bool haveSecurity = buildMappingSecurity(security);
    SECURITY_ATTRIBUTES* attributes = haveSecurity ? &security.attributes : nullptr;
    const DWORD bytes = static_cast<DWORD>(sizeof(MxboShmSnapshot));
    SetLastError(0);
    m_map = CreateFileMappingW(
        INVALID_HANDLE_VALUE,
        attributes,
        PAGE_READWRITE,
        0,
        bytes,
        MXBO_SHM_NAME);
    if (!m_map && attributes)
    {
        attributes = nullptr;
        SetLastError(0);
        m_map = CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            nullptr,
            PAGE_READWRITE,
            0,
            bytes,
            MXBO_SHM_NAME);
    }
    if (!m_map)
    {
        return false;
    }
    const bool existed = GetLastError() == ERROR_ALREADY_EXISTS;
    // An existing smaller section (old plugin) must not be memset/memcpy past its
    // end — that corrupts the game process and can GS-fail in mxbikes.exe.
    if (existed)
    {
        MEMORY_BASIC_INFORMATION info{};
        m_view = MapViewOfFile(m_map, FILE_MAP_ALL_ACCESS, 0, 0, 0);
        if (!m_view || !VirtualQuery(m_view, &info, sizeof(info)) || info.RegionSize < bytes)
        {
            close();
            return false;
        }
        UnmapViewOfFile(m_view);
        m_view = nullptr;
    }
    m_view = MapViewOfFile(m_map, FILE_MAP_ALL_ACCESS, 0, 0, bytes);
    if (!m_view)
    {
        CloseHandle(m_map);
        m_map = nullptr;
        return false;
    }
    MEMORY_BASIC_INFORMATION mapped{};
    if (!VirtualQuery(m_view, &mapped, sizeof(mapped)) || mapped.RegionSize < bytes)
    {
        close();
        return false;
    }
    std::memset(m_view, 0, bytes);
    auto* snap = static_cast<MxboShmSnapshot*>(m_view);
    snap->magic = MXBO_SHM_MAGIC;
    snap->version = MXBO_SHM_VERSION;
    snap->size = static_cast<uint32_t>(sizeof(MxboShmSnapshot));
    snap->seq = 0;

    m_cmdMap = CreateFileMappingW(
        INVALID_HANDLE_VALUE,
        attributes,
        PAGE_READWRITE,
        0,
        static_cast<DWORD>(sizeof(MxboShmCmd)),
        MXBO_CMD_NAME);
    if (m_cmdMap)
    {
        m_cmdView = MapViewOfFile(m_cmdMap, FILE_MAP_ALL_ACCESS, 0, 0, sizeof(MxboShmCmd));
        if (m_cmdView)
        {
            auto* cmd = static_cast<MxboShmCmd*>(m_cmdView);
            if (cmd->magic != MXBO_CMD_MAGIC)
            {
                std::memset(cmd, 0, sizeof(MxboShmCmd));
                cmd->magic = MXBO_CMD_MAGIC;
            }
        }
        else
        {
            CloseHandle(m_cmdMap);
            m_cmdMap = nullptr;
        }
    }
    return true;
}

void ShmWriter::close()
{
    if (m_view)
    {
        UnmapViewOfFile(m_view);
        m_view = nullptr;
    }
    if (m_map)
    {
        CloseHandle(m_map);
        m_map = nullptr;
    }
    if (m_cmdView)
    {
        UnmapViewOfFile(m_cmdView);
        m_cmdView = nullptr;
    }
    if (m_cmdMap)
    {
        CloseHandle(m_cmdMap);
        m_cmdMap = nullptr;
    }
    m_lastSpectateQpc = 0;
    m_poly.clear();
    m_polyCount = 0;
    m_polyRev = 0xFFFFFFFFu;
    m_polyTrail = static_cast<size_t>(-1);
    m_drawCount = 0;
}

void ShmWriter::fillPolyCache(const PluginState& state)
{
    const bool lined = !state.centerline().empty();
    const uint32_t rev = state.mapRev();
    const size_t trail_n = lined ? 0 : state.trail().size();
    if (m_polyRev == rev && m_polyTrail == trail_n)
    {
        return;
    }
    tessellateTrack(state, m_poly);
    m_polyCount = static_cast<int32_t>(m_poly.size());
    m_polyRev = rev;
    m_polyTrail = trail_n;
}

void ShmWriter::publish(const PluginState& state, const PluginConfig& config, bool drawFrame)
{
    if (!m_view)
    {
        return;
    }

    MxboShmSnapshot local{};
    LARGE_INTEGER qpc{};
    QueryPerformanceCounter(&qpc);
    fillPolyCache(state);
    fillSnapshot(local, state, config, m_poly.data(), m_polyCount, static_cast<uint64_t>(qpc.QuadPart));
    if (drawFrame)
    {
        ++m_drawCount;
    }
    local.drawCount = m_drawCount;
    seqlockStore(*static_cast<MxboShmSnapshot*>(m_view), local);
    decaySpectating();
}

void ShmWriter::noteSpectating()
{
    LARGE_INTEGER qpc{};
    QueryPerformanceCounter(&qpc);
    m_lastSpectateQpc = qpc.QuadPart;
    if (m_cmdView)
    {
        static_cast<MxboShmCmd*>(m_cmdView)->spectating = 1;
    }
}

int ShmWriter::takeSpectateRequest()
{
    if (!m_cmdView)
    {
        return 0;
    }
    auto* cmd = static_cast<MxboShmCmd*>(m_cmdView);
    const LONG want = InterlockedExchange(reinterpret_cast<LONG*>(&cmd->spectateRaceNum), 0);
    return static_cast<int>(want);
}

void ShmWriter::decaySpectating()
{
    if (!m_cmdView)
    {
        return;
    }
    auto* cmd = static_cast<MxboShmCmd*>(m_cmdView);
    if (m_lastSpectateQpc == 0)
    {
        cmd->spectating = 0;
        return;
    }
    LARGE_INTEGER now{};
    LARGE_INTEGER freq{};
    QueryPerformanceCounter(&now);
    QueryPerformanceFrequency(&freq);
    const double age = freq.QuadPart > 0
        ? static_cast<double>(now.QuadPart - m_lastSpectateQpc) / static_cast<double>(freq.QuadPart)
        : 1.0;
    if (age > 0.25)
    {
        cmd->spectating = 0;
    }
}
