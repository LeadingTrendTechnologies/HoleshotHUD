#pragma once

#include <cstddef>
#include <string>

inline void copyBounded(char* dest, int destSize, const char* src)
{
    if (!dest || destSize <= 0)
    {
        return;
    }
    dest[0] = '\0';
    if (!src)
    {
        return;
    }
    int n = 0;
    while (src[n] && n + 1 < destSize)
    {
        dest[n] = src[n];
        ++n;
    }
    dest[n] = '\0';
}

inline void truncateCopy(char* dest, size_t destSize, const char* src, int maxChars)
{
    if (!dest || destSize == 0)
    {
        return;
    }
    dest[0] = '\0';
    if (!src)
    {
        return;
    }
    int n = 0;
    while (src[n] && n < maxChars && static_cast<size_t>(n) + 1 < destSize)
    {
        dest[n] = src[n];
        ++n;
    }
    dest[n] = '\0';
}

inline std::string copyCString(const char* s, size_t maxLen)
{
    if (!s)
    {
        return {};
    }
    size_t n = 0;
    while (n < maxLen && s[n] != '\0')
    {
        ++n;
    }
    return std::string(s, s + n);
}
