// Regression test for logger.h ANSI-escape suppression.
//
// Pre-fix, the LOG_* macros always emitted ANSI color codes. When stderr
// was redirected (journald, file, pipe), those bytes landed as literal
// `\033[32m` in the log stream. The fix gates color on isatty.
//
// We install a custom QtMessageHandler so the test runs in-process and
// inspects the rendered message strings. The test process is not a tty
// when launched under ctest → ANSI codes must be absent.

#include <QTest>
#include <QString>
#include <QtGlobal>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(openpods, "openpods.test", QtDebugMsg)

#include "../logger.h"

namespace {
QStringList g_captured;

void captureHandler(QtMsgType, const QMessageLogContext &, const QString &msg)
{
    g_captured.append(msg);
}
} // namespace

class TestLogger : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        // Force the debug-level path on so LOG_DEBUG flows through.
        QLoggingCategory::setFilterRules("openpods.*=true");
        qInstallMessageHandler(captureHandler);
    }

    void cleanupTestCase()
    {
        qInstallMessageHandler(nullptr);
    }

    void init()
    {
        g_captured.clear();
    }

    void infoSurfacesNoAnsi()
    {
        LOG_INFO("hello");
        QCOMPARE(g_captured.size(), 1);
        QVERIFY2(!g_captured.first().contains(QLatin1String("\x1b[")),
                 qPrintable("ANSI escape leaked: " + g_captured.first()));
        QVERIFY(g_captured.first().contains("hello"));
    }

    void warnSurfacesNoAnsi()
    {
        LOG_WARN("warn-msg");
        QCOMPARE(g_captured.size(), 1);
        QVERIFY(!g_captured.first().contains(QLatin1String("\x1b[")));
        QVERIFY(g_captured.first().contains("warn-msg"));
    }

    void errorSurfacesNoAnsi()
    {
        LOG_ERROR("err-msg");
        QCOMPARE(g_captured.size(), 1);
        QVERIFY(!g_captured.first().contains(QLatin1String("\x1b[")));
        QVERIFY(g_captured.first().contains("err-msg"));
    }

    void debugSurfacesNoAnsi()
    {
        LOG_DEBUG("dbg-msg");
        QCOMPARE(g_captured.size(), 1);
        QVERIFY(!g_captured.first().contains(QLatin1String("\x1b[")));
        QVERIFY(g_captured.first().contains("dbg-msg"));
    }
};

QTEST_GUILESS_MAIN(TestLogger)
#include "tst_logger.moc"
