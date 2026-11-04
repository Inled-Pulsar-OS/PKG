// Which of the two paths a given PATH and enabled state picks, since headless has no tray.

#include "notifier.hpp"

#include <QtTest/QtTest>
#include <QSignalSpy>
#include <QTemporaryDir>

class TestNotifier : public QObject
{
    Q_OBJECT

    // notification, send, --app-name, AirPods, -g, the glyph, title, message.
    static constexpr int kExpectedArgCount = 8;

private slots:
    void init()
    {
        m_dir.reset(new QTemporaryDir);
        QVERIFY(m_dir->isValid());
        m_argsFile = m_dir->path() + QStringLiteral("/args");
        qputenv("PATH", m_dir->path().toLocal8Bit());
    }

    void omarchyOnPath_isSpawnedWithTheArguments()
    {
        writeFakeOmarchy();
        Notifier notifier;
        QSignalSpy fallback(&notifier, &Notifier::fallbackRequested);

        notifier.notify(QStringLiteral("Left AirPod Low Battery"), QStringLiteral("10% remaining"));

        // The stub's redirect creates the file before printf fills it, so wait on the content.
        QTRY_VERIFY_WITH_TIMEOUT(readArgs().size() == kExpectedArgCount, 3000);
        const QStringList args = readArgs();
        QCOMPARE(args.value(0), QStringLiteral("notification"));
        QCOMPARE(args.value(1), QStringLiteral("send"));
        QCOMPARE(args.value(2), QStringLiteral("--app-name"));
        QCOMPARE(args.value(3), QStringLiteral("AirPods"));
        QCOMPARE(args.value(6), QStringLiteral("Left AirPod Low Battery"));
        QCOMPARE(args.value(7), QStringLiteral("10% remaining"));
        // The tray fallback is for when omarchy is absent, so spawning it must not also fire.
        QCOMPARE(fallback.count(), 0);
    }

    void omarchyMissing_asksForTheTrayFallback()
    {
        Notifier notifier;
        QSignalSpy fallback(&notifier, &Notifier::fallbackRequested);

        notifier.notify(QStringLiteral("AirPods Disconnected"), QStringLiteral("bye"));

        QCOMPARE(fallback.count(), 1);
        QCOMPARE(fallback.first().at(0).toString(), QStringLiteral("AirPods Disconnected"));
        QCOMPARE(fallback.first().at(1).toString(), QStringLiteral("bye"));
    }

    void disabled_doesNeither()
    {
        writeFakeOmarchy();
        Notifier notifier;
        notifier.setEnabled(false);
        QSignalSpy fallback(&notifier, &Notifier::fallbackRequested);

        notifier.notify(QStringLiteral("Left AirPod Low Battery"), QStringLiteral("10% remaining"));

        QTest::qWait(300);
        QVERIFY(!QFile::exists(m_argsFile));
        QCOMPARE(fallback.count(), 0);
    }

    void enabledChanged_onlyFiresOnAChange()
    {
        Notifier notifier;
        QSignalSpy changed(&notifier, &Notifier::enabledChanged);

        notifier.setEnabled(true);
        QCOMPARE(changed.count(), 0);
        notifier.setEnabled(false);
        QCOMPARE(changed.count(), 1);
        notifier.setEnabled(false);
        QCOMPARE(changed.count(), 1);
    }

    void cleanup()
    {
        qunsetenv("PATH");
        m_dir.reset();
    }

private:
    void writeFakeOmarchy()
    {
        const QString path = m_dir->path() + QStringLiteral("/omarchy");
        QFile f(path);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write("#!/bin/sh\nprintf '%s\\n' \"$@\" > \"" + m_argsFile.toLocal8Bit() + "\"\n");
        f.close();
        QVERIFY(f.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner));
    }

    // One argument per line: "notification\nsend\n--app-name\nAirPods\n-g\n\uF025\nTitle\nMessage\n"
    QStringList readArgs()
    {
        QFile f(m_argsFile);
        if (!f.open(QIODevice::ReadOnly)) {
            return {};
        }
        return QString::fromUtf8(f.readAll()).split(QLatin1Char('\n'), Qt::SkipEmptyParts);
    }

    QScopedPointer<QTemporaryDir> m_dir;
    QString m_argsFile;
};

QTEST_GUILESS_MAIN(TestNotifier)
#include "tst_notifier.moc"
