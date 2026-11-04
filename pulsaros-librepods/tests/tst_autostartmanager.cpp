// AutoStartManager tests.
//
// Covers:
//   - default state is "disabled" (no .desktop)
//   - setAutoStartEnabled(true) creates the file
//   - second setAutoStartEnabled(true) is a no-op (no extra signal)
//   - setAutoStartEnabled(false) removes the file
//   - the written desktop file contains the expected Exec --hide line
//   - createAutoStartEntry's atomic-rename path leaves no <file>.tmp
//   - a symlink planted at the old <file>.tmp path takes no write
//
// QStandardPaths::setTestModeEnabled(true) redirects ConfigLocation to
// ~/.qttest so the test never touches the real user autostart dir.

#include <QTest>
#include <QSignalSpy>
#include <QStandardPaths>
#include <QFile>
#include <QFileInfo>
#include <QDir>
#include <QCoreApplication>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(openpods, "openpods.test", QtWarningMsg)

#include "../autostartmanager.hpp"

class TestAutoStartManager : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        QStandardPaths::setTestModeEnabled(true);
        QCoreApplication::setApplicationName("openpods-test");
        m_path = QStandardPaths::writableLocation(QStandardPaths::ConfigLocation)
            + "/autostart/openpods-test.desktop";
    }

    void init()
    {
        QFile::remove(m_path);
        QFile::remove(m_path + ".tmp");
    }

    void cleanup()
    {
        QFile::remove(m_path);
        QFile::remove(m_path + ".tmp");
    }

    void disabledByDefault()
    {
        AutoStartManager m;
        QVERIFY(!m.autoStartEnabled());
        QVERIFY(!QFile::exists(m_path));
    }

    void enableCreatesDesktopFile()
    {
        AutoStartManager m;
        QSignalSpy spy(&m, &AutoStartManager::autoStartEnabledChanged);

        m.setAutoStartEnabled(true);
        QVERIFY(m.autoStartEnabled());
        QVERIFY(QFile::exists(m_path));
        QCOMPARE(spy.count(), 1);
        QCOMPARE(spy.first().first().toBool(), true);
    }

    void enableTwiceIsIdempotent()
    {
        AutoStartManager m;
        QSignalSpy spy(&m, &AutoStartManager::autoStartEnabledChanged);

        m.setAutoStartEnabled(true);
        m.setAutoStartEnabled(true);
        QCOMPARE(spy.count(), 1); // not 2
        QVERIFY(QFile::exists(m_path));
    }

    void disableRemovesDesktopFile()
    {
        AutoStartManager m;
        QSignalSpy spy(&m, &AutoStartManager::autoStartEnabledChanged);

        m.setAutoStartEnabled(true);
        m.setAutoStartEnabled(false);

        QCOMPARE(spy.count(), 2);
        QCOMPARE(spy.at(1).first().toBool(), false);
        QVERIFY(!QFile::exists(m_path));
        QVERIFY(!m.autoStartEnabled());
    }

    void disableTwiceIsIdempotent()
    {
        AutoStartManager m;
        QSignalSpy spy(&m, &AutoStartManager::autoStartEnabledChanged);

        m.setAutoStartEnabled(false);
        m.setAutoStartEnabled(false);
        QCOMPARE(spy.count(), 0);
    }

    void desktopFileHasHideExec()
    {
        AutoStartManager m;
        m.setAutoStartEnabled(true);

        QFile f(m_path);
        QVERIFY(f.open(QIODevice::ReadOnly | QIODevice::Text));
        const QString content = QString::fromUtf8(f.readAll());

        // Required keys for a freedesktop autostart entry. The fork ships
        // `Exec=<path> --hide` so the daemon starts silently into the tray.
        QVERIFY(content.contains("[Desktop Entry]"));
        QVERIFY(content.contains("Type=Application"));
        QVERIFY(content.contains("--hide"));
        QVERIFY(content.contains("X-GNOME-Autostart-enabled=true"));
    }

    void atomicWriteLeavesNoTmp()
    {
        AutoStartManager m;
        m.setAutoStartEnabled(true);

        QVERIFY(QFile::exists(m_path));
        QVERIFY(!QFile::exists(m_path + ".tmp"));
    }

    void aSymlinkAtTheTemporaryPathIsNotFollowed()
    {
        const QString bystander = QDir::tempPath() + "/openpods-autostart-bystander";
        QFile::remove(bystander);
        QFile keep(bystander);
        QVERIFY(keep.open(QIODevice::WriteOnly | QIODevice::Text));
        keep.write("UNTOUCHED\n");
        keep.close();

        QDir().mkpath(QFileInfo(m_path).absolutePath());
        QVERIFY(QFile::link(bystander, m_path + ".tmp"));

        AutoStartManager m;
        m.setAutoStartEnabled(true);

        QFile check(bystander);
        QVERIFY(check.open(QIODevice::ReadOnly | QIODevice::Text));
        QCOMPARE(check.readAll(), QByteArray("UNTOUCHED\n"));
        check.close();
        QVERIFY(!QFileInfo(m_path).isSymLink());
        QFile::remove(bystander);
    }

    void reenableAfterDisable()
    {
        AutoStartManager m;
        m.setAutoStartEnabled(true);
        m.setAutoStartEnabled(false);
        m.setAutoStartEnabled(true);
        QVERIFY(m.autoStartEnabled());
        QVERIFY(QFile::exists(m_path));
    }

private:
    QString m_path;
};

QTEST_GUILESS_MAIN(TestAutoStartManager)
#include "tst_autostartmanager.moc"
