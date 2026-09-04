#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif

#include "steam_friends.h"

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <thread>
#include <vector>

#include <windows.h>

namespace
{
    using HSteamUser = int32_t;
    using HSteamPipe = int32_t;

    using SteamAPI_GetHSteamUser_Fn = HSteamUser(__cdecl*)();
    using SteamAPI_GetHSteamPipe_Fn = HSteamPipe(__cdecl*)();
    using SteamInternal_FindOrCreateUserInterface_Fn = void*(__cdecl*)(HSteamUser, const char*);
    using SteamClient_Fn = void*(__cdecl*)();
    using ISteamClient_GetISteamUser_Fn = void*(__cdecl*)(void*, HSteamUser, HSteamPipe, const char*);
    using ISteamClient_GetISteamFriends_Fn = void*(__cdecl*)(void*, HSteamUser, HSteamPipe, const char*);
    using ISteamUser_GetSteamID_Fn = uint64_t(__cdecl*)(void*);
    using ISteamFriends_GetFriendCount_Fn = int(__cdecl*)(void*, int);
    using ISteamFriends_GetFriendByIndex_Fn = uint64_t(__cdecl*)(void*, int, int);

    constexpr int kFriendImmediate = 0x04;
    constexpr int kScanMs = 10000;

    const char* kUserVers[] = {"SteamUser023", "SteamUser022", "SteamUser021", "SteamUser020", nullptr};
    const char* kFriendVers[] = {"SteamFriends017", "SteamFriends018", "SteamFriends016", "SteamFriends015", nullptr};

#if defined(_MSC_VER)
    uint64_t sehSteamId(ISteamUser_GetSteamID_Fn fn, void* self)
    {
        __try
        {
            return fn(self);
        }
        __except (EXCEPTION_EXECUTE_HANDLER)
        {
            return 0;
        }
    }

    int sehFriendCount(ISteamFriends_GetFriendCount_Fn fn, void* self, int flags)
    {
        __try
        {
            return fn(self, flags);
        }
        __except (EXCEPTION_EXECUTE_HANDLER)
        {
            return -1;
        }
    }

    uint64_t sehFriendByIndex(ISteamFriends_GetFriendByIndex_Fn fn, void* self, int i, int flags)
    {
        __try
        {
            return fn(self, i, flags);
        }
        __except (EXCEPTION_EXECUTE_HANDLER)
        {
            return 0;
        }
    }
#else
    uint64_t sehSteamId(ISteamUser_GetSteamID_Fn fn, void* self)
    {
        return fn(self);
    }
    int sehFriendCount(ISteamFriends_GetFriendCount_Fn fn, void* self, int flags)
    {
        return fn(self, flags);
    }
    uint64_t sehFriendByIndex(ISteamFriends_GetFriendByIndex_Fn fn, void* self, int i, int flags)
    {
        return fn(self, i, flags);
    }
#endif

    struct Hook
    {
        void* user = nullptr;
        void* friends = nullptr;
        ISteamUser_GetSteamID_Fn getId = nullptr;
        ISteamFriends_GetFriendCount_Fn getCount = nullptr;
        ISteamFriends_GetFriendByIndex_Fn getByIndex = nullptr;
        bool ok = false;
    };

    bool acquire(Hook& h)
    {
        HMODULE dll = GetModuleHandleW(L"steam_api64.dll");
        if (!dll)
        {
            return false;
        }

        h.getId = reinterpret_cast<ISteamUser_GetSteamID_Fn>(
            GetProcAddress(dll, "SteamAPI_ISteamUser_GetSteamID"));
        h.getCount = reinterpret_cast<ISteamFriends_GetFriendCount_Fn>(
            GetProcAddress(dll, "SteamAPI_ISteamFriends_GetFriendCount"));
        h.getByIndex = reinterpret_cast<ISteamFriends_GetFriendByIndex_Fn>(
            GetProcAddress(dll, "SteamAPI_ISteamFriends_GetFriendByIndex"));
        if (!h.getId || !h.getCount || !h.getByIndex)
        {
            return false;
        }

        auto findIface = reinterpret_cast<SteamInternal_FindOrCreateUserInterface_Fn>(
            GetProcAddress(dll, "SteamInternal_FindOrCreateUserInterface"));
        auto getUser = reinterpret_cast<SteamAPI_GetHSteamUser_Fn>(
            GetProcAddress(dll, "SteamAPI_GetHSteamUser"));
        auto getPipe = reinterpret_cast<SteamAPI_GetHSteamPipe_Fn>(
            GetProcAddress(dll, "SteamAPI_GetHSteamPipe"));

        HSteamUser user = getUser ? getUser() : 0;
        HSteamPipe pipe = getPipe ? getPipe() : 0;

        if (findIface && user != 0)
        {
            for (int i = 0; kUserVers[i]; ++i)
            {
                void* p = findIface(user, kUserVers[i]);
                if (p && sehSteamId(h.getId, p) != 0)
                {
                    h.user = p;
                    break;
                }
            }
            for (int i = 0; kFriendVers[i]; ++i)
            {
                void* p = findIface(user, kFriendVers[i]);
                if (p && sehFriendCount(h.getCount, p, kFriendImmediate) >= 0)
                {
                    h.friends = p;
                    break;
                }
            }
        }

        if ((!h.user || !h.friends) && user != 0 && pipe != 0)
        {
            auto steamClient = reinterpret_cast<SteamClient_Fn>(GetProcAddress(dll, "SteamClient"));
            auto getIUser = reinterpret_cast<ISteamClient_GetISteamUser_Fn>(
                GetProcAddress(dll, "SteamAPI_ISteamClient_GetISteamUser"));
            auto getIFriends = reinterpret_cast<ISteamClient_GetISteamFriends_Fn>(
                GetProcAddress(dll, "SteamAPI_ISteamClient_GetISteamFriends"));
            void* client = steamClient ? steamClient() : nullptr;
            if (client && getIUser && getIFriends)
            {
                if (!h.user)
                {
                    for (int i = 0; kUserVers[i]; ++i)
                    {
                        void* p = getIUser(client, user, pipe, kUserVers[i]);
                        if (p && sehSteamId(h.getId, p) != 0)
                        {
                            h.user = p;
                            break;
                        }
                    }
                }
                if (!h.friends)
                {
                    for (int i = 0; kFriendVers[i]; ++i)
                    {
                        void* p = getIFriends(client, user, pipe, kFriendVers[i]);
                        if (p && sehFriendCount(h.getCount, p, kFriendImmediate) >= 0)
                        {
                            h.friends = p;
                            break;
                        }
                    }
                }
            }
        }

        h.ok = h.user && h.friends;
        return h.ok;
    }

    struct Store
    {
        std::mutex mu;
        std::condition_variable cv;
        std::thread worker;
        std::atomic<bool> stop{false};
        bool started = false;
        uint64_t localId = 0;
        std::vector<uint64_t> friends;
        Hook hook;
    };

    Store g;

    void scan(Hook& hook)
    {
        if (!hook.ok)
        {
            if (!acquire(hook))
            {
                return;
            }
        }

        const uint64_t me = sehSteamId(hook.getId, hook.user);
        const int n = sehFriendCount(hook.getCount, hook.friends, kFriendImmediate);
        if (me == 0 || n < 0)
        {
            hook.ok = false;
            return;
        }

        std::vector<uint64_t> ids;
        ids.reserve(static_cast<size_t>(n));
        for (int i = 0; i < n; ++i)
        {
            const uint64_t id = sehFriendByIndex(hook.getByIndex, hook.friends, i, kFriendImmediate);
            if (id != 0 && id != me)
            {
                ids.push_back(id);
            }
        }

        std::lock_guard<std::mutex> lock(g.mu);
        g.localId = me;
        g.friends.swap(ids);
    }

    void workerLoop()
    {
        try
        {
            while (!g.stop.load(std::memory_order_relaxed))
            {
                scan(g.hook);
                std::unique_lock<std::mutex> lock(g.mu);
                g.cv.wait_for(lock, std::chrono::milliseconds(kScanMs), [] {
                    return g.stop.load(std::memory_order_relaxed);
                });
            }
        }
        catch (...)
        {
        }
    }
}

void steamFriendsStart()
{
    if (g.started)
    {
        return;
    }
    g.stop.store(false, std::memory_order_relaxed);
    g.started = true;
    g.worker = std::thread(workerLoop);
}

void steamFriendsStop()
{
    if (!g.started)
    {
        return;
    }
    g.stop.store(true, std::memory_order_relaxed);
    g.cv.notify_all();
    if (g.worker.joinable())
    {
        g.worker.join();
    }
    std::lock_guard<std::mutex> lock(g.mu);
    g.localId = 0;
    g.friends.clear();
    g.hook = Hook{};
    g.started = false;
}

void steamFriendsCopy(uint64_t* localId, int32_t* count, uint64_t* ids, int maxIds)
{
    std::lock_guard<std::mutex> lock(g.mu);
    if (localId)
    {
        *localId = g.localId;
    }
    int n = static_cast<int>(g.friends.size());
    if (n > maxIds)
    {
        n = maxIds;
    }
    if (count)
    {
        *count = n;
    }
    if (ids)
    {
        for (int i = 0; i < n; ++i)
        {
            ids[i] = g.friends[static_cast<size_t>(i)];
        }
    }
}
