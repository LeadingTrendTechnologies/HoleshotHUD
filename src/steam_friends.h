#pragma once

#include <cstdint>

// Live Steam friend graph from the game's already-loaded steam_api64.dll.
// Do not call SteamAPI_Init. Do not log Steam IDs.

void steamFriendsStart();
void steamFriendsStop();
void steamFriendsCopy(uint64_t* localId, int32_t* count, uint64_t* ids, int maxIds);
