#pragma once

#include <QDebug>
#include <QLoggingCategory>
#include <unistd.h>

Q_DECLARE_LOGGING_CATEGORY(openpods)

// ANSI color helpers. The escape sequences make terminal output readable
// but appear as literal `\033[32m` garbage when stderr is redirected
// (journald, `2>&1 | tee`, systemd-managed service). Gate on
// isatty(STDERR_FILENO); cached once at first use.
namespace openpods_log_detail {
inline bool ttyOnce()
{
    static const bool t = ::isatty(STDERR_FILENO);
    return t;
}
inline const char *clr(const char *code) { return ttyOnce() ? code : ""; }
} // namespace openpods_log_detail

#define LOG_INFO(msg)  qCInfo(openpods)     << openpods_log_detail::clr("\033[32m") << msg << openpods_log_detail::clr("\033[0m")
#define LOG_WARN(msg)  qCWarning(openpods)  << openpods_log_detail::clr("\033[33m") << msg << openpods_log_detail::clr("\033[0m")
#define LOG_ERROR(msg) qCCritical(openpods) << openpods_log_detail::clr("\033[31m") << msg << openpods_log_detail::clr("\033[0m")
#define LOG_DEBUG(msg) qCDebug(openpods)    << openpods_log_detail::clr("\033[34m") << msg << openpods_log_detail::clr("\033[0m")
