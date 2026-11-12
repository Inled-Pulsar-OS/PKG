#include <QTest>
#include "../snaptogrid.hpp"

class TestSnapToGrid : public QObject {
    Q_OBJECT

private slots:
    void exactMultiplesUnchanged() {
        for (int v = 0; v <= 100; v += 5) {
            QCOMPARE(snapToGrid(v), v);
        }
    }

    void roundsHalfUp() {
        // step=5 default. Midpoint at .5 -> round up.
        QCOMPARE(snapToGrid(0), 0);
        QCOMPARE(snapToGrid(1), 0);
        QCOMPARE(snapToGrid(2), 0);
        QCOMPARE(snapToGrid(3), 5);   // 3 + 5/2 = 5 -> /5 -> 1 -> *5 -> 5
        QCOMPARE(snapToGrid(4), 5);
        QCOMPARE(snapToGrid(5), 5);
        QCOMPARE(snapToGrid(7), 5);   // 7 + 2 = 9 -> /5 -> 1 -> 5
        QCOMPARE(snapToGrid(8), 10);  // 8 + 2 = 10 -> /5 -> 2 -> 10
        QCOMPARE(snapToGrid(50), 50);
        QCOMPARE(snapToGrid(52), 50);
        QCOMPARE(snapToGrid(53), 55);
        QCOMPARE(snapToGrid(97), 95); // 97 + 2 = 99 -> /5 -> 19 -> 95
        QCOMPARE(snapToGrid(98), 100);
        QCOMPARE(snapToGrid(100), 100);
    }

    void avrcpStepsLandOnFives() {
        // AirPods AVRCP 15-step values: 0, 7, 13, 20, 27, 33, 40, 47,
        // 53, 60, 67, 73, 80, 87, 93, 100. Snap each to the nearest 5.
        QCOMPARE(snapToGrid(0),   0);
        QCOMPARE(snapToGrid(7),   5);
        QCOMPARE(snapToGrid(13),  15);
        QCOMPARE(snapToGrid(20),  20);
        QCOMPARE(snapToGrid(27),  25);
        QCOMPARE(snapToGrid(33),  35);
        QCOMPARE(snapToGrid(40),  40);
        QCOMPARE(snapToGrid(47),  45);
        QCOMPARE(snapToGrid(53),  55);
        QCOMPARE(snapToGrid(60),  60);
        QCOMPARE(snapToGrid(67),  65);
        QCOMPARE(snapToGrid(73),  75);
        QCOMPARE(snapToGrid(80),  80);
        QCOMPARE(snapToGrid(87),  85);
        QCOMPARE(snapToGrid(93),  95);
        QCOMPARE(snapToGrid(100), 100);
    }

    void clampsBelowZero() {
        QCOMPARE(snapToGrid(-1),    0);
        QCOMPARE(snapToGrid(-100),  0);
    }

    void clampsAbove100() {
        QCOMPARE(snapToGrid(101),  100);
        QCOMPARE(snapToGrid(150),  100);
        QCOMPARE(snapToGrid(9999), 100);
    }

    void customStepOf10() {
        QCOMPARE(snapToGrid(0,   10), 0);
        QCOMPARE(snapToGrid(4,   10), 0);
        QCOMPARE(snapToGrid(5,   10), 10);
        QCOMPARE(snapToGrid(14,  10), 10);
        QCOMPARE(snapToGrid(15,  10), 20);
        QCOMPARE(snapToGrid(100, 10), 100);
    }

    void customStepOf25() {
        QCOMPARE(snapToGrid(0,   25), 0);
        QCOMPARE(snapToGrid(12,  25), 0);
        QCOMPARE(snapToGrid(13,  25), 25);
        QCOMPARE(snapToGrid(50,  25), 50);
        QCOMPARE(snapToGrid(99,  25), 100);
    }

    void stepZeroFallsBackToOne() {
        // Effective no-op snap (still clamped). 37 -> 37.
        QCOMPARE(snapToGrid(37, 0), 37);
        QCOMPARE(snapToGrid(-3, 0), 0);
        QCOMPARE(snapToGrid(150, 0), 100);
    }

    void stepNegativeFallsBackToOne() {
        QCOMPARE(snapToGrid(37, -5), 37);
        QCOMPARE(snapToGrid(150, -5), 100);
    }

    void stepLargerThanRangeClampedTo100() {
        // step > 100 is clamped to 100. Then anything < 50 -> 0, >= 50 -> 100.
        QCOMPARE(snapToGrid(0,   200), 0);
        QCOMPARE(snapToGrid(49,  200), 0);
        QCOMPARE(snapToGrid(50,  200), 100);
        QCOMPARE(snapToGrid(100, 200), 100);
    }

    void exhaustiveStep5IsMonotonic() {
        // Snap is monotonic non-decreasing on input.
        int prev = 0;
        for (int v = 0; v <= 100; ++v) {
            int s = snapToGrid(v);
            QVERIFY2(s >= prev, qPrintable(QString("at v=%1 s=%2 prev=%3").arg(v).arg(s).arg(prev)));
            QVERIFY2(s % 5 == 0, qPrintable(QString("s=%1 not multiple of 5").arg(s)));
            prev = s;
        }
    }
};

QTEST_GUILESS_MAIN(TestSnapToGrid)
#include "tst_snaptogrid.moc"
