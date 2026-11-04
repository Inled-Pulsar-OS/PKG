#include <QTest>
#include "../lowbatterywatcher.hpp"

class TestLowBatteryWatcher : public QObject {
    Q_OBJECT

private slots:
    void unavailableNeverTrips() {
        LowBatteryLatch l;
        QVERIFY(!l.evaluate(0, /*available=*/false, /*charging=*/false));
        QVERIFY(!l.evaluate(5, false, false));
        QVERIFY(!l.notified());
    }

    void chargingNeverTripsAndClearsLatch() {
        LowBatteryLatch l;
        // Trip first so the latch is set.
        QCOMPARE(l.evaluate(5, true, false).value_or(quint8(0xff)), quint8(5));
        QVERIFY(l.notified());
        // Charging clears the latch but produces no notification.
        QVERIFY(!l.evaluate(5, true, true));
        QVERIFY(!l.notified());
    }

    void aboveTriggerNeverFires() {
        LowBatteryLatch l;
        QVERIFY(!l.evaluate(11, true, false));
        QVERIFY(!l.evaluate(50, true, false));
        QVERIFY(!l.evaluate(100, true, false));
        QVERIFY(!l.notified());
    }

    void firstCrossBelowTriggerFires() {
        LowBatteryLatch l;
        const auto r = l.evaluate(10, true, false);
        QVERIFY(r.has_value());
        QCOMPARE(r.value_or(quint8(0xff)), quint8(10));
        QVERIFY(l.notified());
    }

    void secondReadingBelowTriggerSuppressed() {
        LowBatteryLatch l;
        QVERIFY(l.evaluate(8, true, false));
        QVERIFY(!l.evaluate(7, true, false));  // already latched
        QVERIFY(!l.evaluate(5, true, false));
    }

    void recoveryToHysteresisRangeStillLatched() {
        LowBatteryLatch l;
        QVERIFY(l.evaluate(9, true, false));
        // Between trigger (10) and reset (15): latch stays set, no
        // notification. This is the hysteresis band.
        QVERIFY(!l.evaluate(12, true, false));
        QVERIFY(l.notified());
        QVERIFY(!l.evaluate(14, true, false));
        QVERIFY(l.notified());
    }

    void recoveryToResetClearsLatchSilently() {
        LowBatteryLatch l;
        QVERIFY(l.evaluate(9, true, false));
        QVERIFY(!l.evaluate(15, true, false));  // reset level: clears latch but no notify
        QVERIFY(!l.notified());
    }

    void retripAfterRecovery() {
        LowBatteryLatch l;
        QVERIFY(l.evaluate(8, true, false));
        QVERIFY(!l.evaluate(20, true, false));
        QVERIFY(!l.notified());
        // Discharge cycle: should fire again.
        QCOMPARE(l.evaluate(8, true, false).value_or(quint8(0xff)), quint8(8));
        QVERIFY(l.notified());
    }

    void retripAfterCharging() {
        LowBatteryLatch l;
        QVERIFY(l.evaluate(7, true, false));
        QVERIFY(!l.evaluate(7, true, true));  // charging clears
        QVERIFY(!l.notified());
        // Unplug while still low: fires immediately.
        QCOMPARE(l.evaluate(7, true, false).value_or(quint8(0xff)), quint8(7));
    }

    void boundariesExact() {
        LowBatteryLatch l(/*trigger=*/10, /*reset=*/15);
        // Exactly trigger -> trip.
        QCOMPARE(l.evaluate(10, true, false).value_or(quint8(0xff)), quint8(10));
        // Exactly reset -> clears latch silently.
        QVERIFY(!l.evaluate(15, true, false));
        QVERIFY(!l.notified());
        // Just above trigger after reset -> no trip.
        QVERIFY(!l.evaluate(11, true, false));
    }

    void customThresholds() {
        LowBatteryLatch l(/*trigger=*/5, /*reset=*/20);
        QCOMPARE(l.trigger(), quint8(5));
        QCOMPARE(l.reset(), quint8(20));
        QVERIFY(!l.evaluate(8, true, false));  // above the custom trigger
        QCOMPARE(l.evaluate(5, true, false).value_or(quint8(0xff)), quint8(5));
        QVERIFY(!l.evaluate(19, true, false));  // still latched in the wider band
        QVERIFY(!l.evaluate(20, true, false));  // resets at custom reset
        QVERIFY(!l.notified());
    }

    void resetLatchMethod() {
        LowBatteryLatch l;
        QVERIFY(l.evaluate(5, true, false));
        QVERIFY(l.notified());
        l.resetLatch();
        QVERIFY(!l.notified());
        // After manual reset, a still-low reading fires again.
        QCOMPARE(l.evaluate(5, true, false).value_or(quint8(0xff)), quint8(5));
    }

    void unavailableDoesNotClearLatch() {
        // Pod went out of range mid-low. Latch must persist so a
        // brief signal blip doesn't unlock a duplicate toast on
        // reconnect.
        LowBatteryLatch l;
        QVERIFY(l.evaluate(8, true, false));
        QVERIFY(!l.evaluate(8, false, false));
        QVERIFY(l.notified());
        QVERIFY(!l.evaluate(8, true, false));  // still latched
    }
};

QTEST_GUILESS_MAIN(TestLowBatteryWatcher)
#include "tst_lowbatterywatcher.moc"
