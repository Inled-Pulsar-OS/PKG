// Pure-helper tests for OpenPods::Ipc::socketPath. The whole reason the
// socket left /tmp is that this returns an absolute path under the
// runtime dir, so that property is what these assert.

#include "ipcpath.hpp"

#include <QtTest/QtTest>

class TestIpcPath : public QObject
{
    Q_OBJECT

private slots:
    void unsetRuntimeDir_returnsEmpty()
    {
        qunsetenv("XDG_RUNTIME_DIR");
        QVERIFY(OpenPods::Ipc::socketPath().isEmpty());
    }

    void setRuntimeDir_returnsAbsolutePathUnderIt()
    {
        qputenv("XDG_RUNTIME_DIR", "/run/user/1000");
        const QString path = OpenPods::Ipc::socketPath();
        QCOMPARE(path, QStringLiteral("/run/user/1000/librepods.sock"));
        // A leading slash is what makes QLocalServer bind here instead of /tmp.
        QVERIFY(path.startsWith(QLatin1Char('/')));
    }

    void cleanup()
    {
        qunsetenv("XDG_RUNTIME_DIR");
    }
};

QTEST_GUILESS_MAIN(TestIpcPath)
#include "tst_ipcpath.moc"
