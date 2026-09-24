#pragma once

class PluginState;

namespace crash_log
{
    // Install unhandled-exception filter. Pass live PluginState for session snapshot.
    void install(const PluginState* state);

    void breadcrumb(const char* name);
    void flushTrail();

    // Call from RunTelemetry; flushes last-trail.txt every 25 frames.
    void noteTelemetry();
}
