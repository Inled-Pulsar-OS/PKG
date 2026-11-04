// Pure-helper tests for OpenPods::Ipc::parseIntVerb — extracted from
// main.cpp's QLocalServer dispatch so we can exercise the parser
// without booting the daemon.

#include "ipcverb.hpp"

#include <QtTest/QtTest>

class TestIpcVerb : public QObject
{
    Q_OBJECT

private slots:
    void wrongPrefix_returnsNullopt()
    {
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("noise:adaptive"),
                                               QStringLiteral("adaptive:"));
        QVERIFY(!out.has_value());
    }

    void emptyPayload_returnsNullopt()
    {
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("adaptive:"),
                                               QStringLiteral("adaptive:"));
        QVERIFY(!out.has_value());
    }

    void nonNumericPayload_returnsNullopt()
    {
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("adaptive:high"),
                                               QStringLiteral("adaptive:"));
        QVERIFY(!out.has_value());
    }

    void inBoundsPayload_parses()
    {
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("adaptive:37"),
                                               QStringLiteral("adaptive:"));
        // value_or(-1) sentinel keeps clang-tidy happy
        // (bugprone-unchecked-optional-access fires on a separate
        // has_value() check + .value() across statements).
        QCOMPARE(out.value_or(-1), 37);
    }

    void zeroBoundary_parses()
    {
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("adaptive:0"),
                                               QStringLiteral("adaptive:"));
        QCOMPARE(out.value_or(-1), 0);
    }

    void maxBoundary_parses()
    {
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("adaptive:100"),
                                               QStringLiteral("adaptive:"));
        QCOMPARE(out.value_or(-1), 100);
    }

    void aboveMax_returnsNullopt()
    {
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("adaptive:101"),
                                               QStringLiteral("adaptive:"));
        QVERIFY(!out.has_value());
    }

    void belowMin_returnsNullopt()
    {
        // Negative payload — toInt parses it, but bounds gate rejects.
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("adaptive:-1"),
                                               QStringLiteral("adaptive:"));
        QVERIFY(!out.has_value());
    }

    void customRange_respectsBounds()
    {
        // mic:N hypothetical with [0, 2] range.
        QCOMPARE(OpenPods::Ipc::parseIntVerb(QStringLiteral("mic:0"),
                                              QStringLiteral("mic:"), 0, 2)
                     .value_or(-1),
                 0);
        QCOMPARE(OpenPods::Ipc::parseIntVerb(QStringLiteral("mic:2"),
                                              QStringLiteral("mic:"), 0, 2)
                     .value_or(-1),
                 2);
        QVERIFY(!OpenPods::Ipc::parseIntVerb(QStringLiteral("mic:3"),
                                              QStringLiteral("mic:"), 0, 2)
                     .has_value());
    }

    void leadingWhitespace_returnsNullopt()
    {
        // The dispatch never feeds leading whitespace; verify the
        // parser doesn't silently swallow it (would mask client-side
        // bugs that send "adaptive: 50").
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("adaptive: 50"),
                                               QStringLiteral("adaptive:"));
        QVERIFY(!out.has_value());
    }

    void prefixOnlyMessage_returnsNullopt()
    {
        // Exact prefix, no colon-content, no payload.
        auto out = OpenPods::Ipc::parseIntVerb(QStringLiteral("adaptive"),
                                               QStringLiteral("adaptive:"));
        QVERIFY(!out.has_value());
    }
};

QTEST_APPLESS_MAIN(TestIpcVerb)
#include "tst_ipcverb.moc"
