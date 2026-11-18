// Regression test for the eardetection.hpp boundary fix.
//
// Pre-fix, parseData accepted any input with size >= 2 and then read
// data[6]/data[7] unconditionally — out-of-bounds for inputs of size 2..7.
// Post-fix it requires size >= 8.
//
// Also covers parseStatusByte's 4-byte enum mapping (0x00 InEar,
// 0x01 NotInEar, 0x02 InCase, anything else Disconnected).

#include <QTest>
#include <QByteArray>
#include <QSignalSpy>
#include <QLoggingCategory>

// EarDetection's parseData calls LOG_DEBUG(openpods)... — main.cpp
// defines the Q_LOGGING_CATEGORY symbol, but tests don't link main.cpp.
// Define the category locally so the linker is satisfied. Silent by
// default.
Q_LOGGING_CATEGORY(openpods, "openpods.test", QtWarningMsg)

#include "../eardetection.hpp"

using Status = EarDetection::EarDetectionStatus;

class TestEarDetection : public QObject
{
    Q_OBJECT

private slots:
    void initialState()
    {
        EarDetection d;
        QCOMPARE(d.getprimaryStatus(), Status::Disconnected);
        QCOMPARE(d.getsecondaryStatus(), Status::Disconnected);
        QVERIFY(!d.isPrimaryInEar());
        QVERIFY(!d.isSecondaryInEar());
        QVERIFY(!d.oneOrMorePodsInEar());
        QVERIFY(!d.oneOrMorePodsInCase());
    }

    void rejectsShortPackets_data()
    {
        QTest::addColumn<int>("size");
        // pre-fix would accept sizes 2..7 and read past the buffer
        for (int s : {0, 1, 2, 3, 4, 5, 6, 7}) {
            QTest::newRow(QByteArray::number(s).constData()) << s;
        }
    }

    void rejectsShortPackets()
    {
        QFETCH(int, size);
        EarDetection d;
        QByteArray buf(size, '\0');
        QSignalSpy spy(&d, &EarDetection::statusChanged);
        QVERIFY(!d.parseData(buf));
        QCOMPARE(spy.count(), 0);
    }

    void acceptsEightBytePacket()
    {
        EarDetection d;
        QByteArray buf(8, '\0');
        buf[6] = 0x00; // primary InEar
        buf[7] = 0x01; // secondary NotInEar
        QSignalSpy spy(&d, &EarDetection::statusChanged);

        QVERIFY(d.parseData(buf));
        QCOMPARE(spy.count(), 1);
        QCOMPARE(d.getprimaryStatus(), Status::InEar);
        QCOMPARE(d.getsecondaryStatus(), Status::NotInEar);
        QVERIFY(d.isPrimaryInEar());
        QVERIFY(!d.isSecondaryInEar());
        QVERIFY(d.oneOrMorePodsInEar());
    }

    void parsesAllStatusBytes_data()
    {
        QTest::addColumn<int>("byte");
        QTest::addColumn<int>("status");
        QTest::newRow("InEar 0x00")     << 0x00 << int(Status::InEar);
        QTest::newRow("NotInEar 0x01")  << 0x01 << int(Status::NotInEar);
        QTest::newRow("InCase 0x02")    << 0x02 << int(Status::InCase);
        QTest::newRow("Other 0x03")     << 0x03 << int(Status::Disconnected);
        QTest::newRow("Other 0xFF")     << 0xFF << int(Status::Disconnected);
    }

    void parsesAllStatusBytes()
    {
        QFETCH(int, byte);
        QFETCH(int, status);

        EarDetection d;
        QByteArray buf(8, '\0');
        buf[6] = static_cast<char>(byte);
        buf[7] = 0x01;
        QVERIFY(d.parseData(buf));
        QCOMPARE(int(d.getprimaryStatus()), status);
    }

    void inCaseSemantic()
    {
        EarDetection d;
        QByteArray buf(8, '\0');
        buf[6] = 0x02; // primary InCase
        buf[7] = 0x01; // secondary NotInEar
        QVERIFY(d.parseData(buf));
        QVERIFY(d.oneOrMorePodsInCase());
        QVERIFY(!d.oneOrMorePodsInEar());
    }

    void resetClearsState()
    {
        EarDetection d;
        QByteArray buf(8, '\0');
        buf[6] = 0x00;
        buf[7] = 0x00;
        QVERIFY(d.parseData(buf));
        QVERIFY(d.oneOrMorePodsInEar());

        d.reset();
        QCOMPARE(d.getprimaryStatus(), Status::Disconnected);
        QCOMPARE(d.getsecondaryStatus(), Status::Disconnected);
        QVERIFY(!d.oneOrMorePodsInEar());
    }

    void overrideMatchesProtocol()
    {
        EarDetection d;
        QSignalSpy spy(&d, &EarDetection::statusChanged);

        d.overrideEarDetectionStatus(true, false);
        QCOMPARE(d.getprimaryStatus(), Status::InEar);
        QCOMPARE(d.getsecondaryStatus(), Status::NotInEar);
        QCOMPARE(spy.count(), 1);

        d.overrideEarDetectionStatus(false, true);
        QCOMPARE(d.getprimaryStatus(), Status::NotInEar);
        QCOMPARE(d.getsecondaryStatus(), Status::InEar);
        QCOMPARE(spy.count(), 2);
    }
};

QTEST_GUILESS_MAIN(TestEarDetection)
#include "tst_eardetection.moc"
