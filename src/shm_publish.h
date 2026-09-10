#pragma once

#include "config.h"
#include "shm/mxbo_shm.h"
#include "state.h"

#include <vector>

void tessellateTrack(const PluginState& state, std::vector<MxboShmPoint>& pts);
void fillSnapshot(MxboShmSnapshot& local,
                  const PluginState& state,
                  const PluginConfig& config,
                  const MxboShmPoint* poly,
                  int polyCount,
                  uint64_t tickQpc);
void seqlockStore(MxboShmSnapshot& dst, const MxboShmSnapshot& local);
