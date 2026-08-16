#pragma once

#include <cstdint>

// PiBoSo plugin colors are packed ABGR (A in the high byte, R in the low byte).
constexpr unsigned long abgr(std::uint8_t a, std::uint8_t b, std::uint8_t g, std::uint8_t r)
{
    return (static_cast<unsigned long>(a) << 24) |
           (static_cast<unsigned long>(b) << 16) |
           (static_cast<unsigned long>(g) << 8) |
           static_cast<unsigned long>(r);
}

namespace Palette
{
    constexpr unsigned long kPanelBg     = abgr(200, 10, 10, 10);
    constexpr unsigned long kPanelBorder = abgr(140, 78, 78, 82);
    constexpr unsigned long kHeaderBg    = abgr(230, 16, 16, 16);
    constexpr unsigned long kHeaderLine  = abgr(255, 48, 148, 255);
    constexpr unsigned long kRowAlt      = abgr(28, 22, 22, 24);
    constexpr unsigned long kLocalRow    = abgr(64, 28, 72, 160);
    constexpr unsigned long kLocalPip    = abgr(240, 48, 148, 255);
    constexpr unsigned long kText        = abgr(255, 228, 228, 230);
    constexpr unsigned long kTextDim     = abgr(255, 132, 132, 138);
    constexpr unsigned long kAccent      = abgr(255, 48, 148, 255);
    constexpr unsigned long kAhead       = abgr(255, 96, 196, 92);
    constexpr unsigned long kBehind      = abgr(255, 96, 96, 232);
    constexpr unsigned long kTrack       = abgr(255, 232, 232, 236);
    constexpr unsigned long kTrackEdge   = abgr(220, 18, 18, 20);
    constexpr unsigned long kMapFill     = abgr(168, 8, 8, 10);
    constexpr unsigned long kStartFinish = abgr(255, 245, 245, 245);
    constexpr unsigned long kRider       = abgr(255, 255, 88, 168);
    constexpr unsigned long kRiderLocal  = abgr(255, 48, 148, 255);
    constexpr unsigned long kRiderCrash  = abgr(255, 64, 64, 220);
    constexpr unsigned long kDotRing     = abgr(240, 8, 8, 8);
}
