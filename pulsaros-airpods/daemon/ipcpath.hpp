#pragma once

#include <QByteArray>
#include <QFile>
#include <QString>

namespace OpenPods::Ipc
{
    // QLocalServer and QLocalSocket treat a leading '/' as a full path, so this
    // binds under /run/user/<uid> (mode 0700) instead of world-visible /tmp.
    // Empty return means the session has no XDG_RUNTIME_DIR; callers must fail
    // rather than fall back, or the socket lands back in /tmp.
    inline QString socketPath()
    {
        const QByteArray runtimeDir = qgetenv("XDG_RUNTIME_DIR");
        if (runtimeDir.isEmpty()) {
            return QString();
        }
        return QFile::decodeName(runtimeDir) + QStringLiteral("/librepods.sock");
    }
}
