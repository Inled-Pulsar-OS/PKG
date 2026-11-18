// In-process stochastic stress over protocol parsers.
//
// Not a libFuzzer harness — that needs a clang build dir parallel to
// the GCC main build. This sweeps QRandomGenerator across the same
// parsers a fuzzer would target, asserting no crash / no
// std::out_of_range / no QByteArray::at assertion-failure under tens
// of thousands of random inputs. Builds in the standard ctest pipeline.
//
// Targets:
//   - ControlCommand::parseActive   (size+startsWith bounds)
//   - BLEUtils::decryptLastBytes    (key+data shape checks)
//   - BLEUtils::isValidIrkRpa       (address-string parsing)

#include <QTest>
#include <QByteArray>
#include <QRandomGenerator>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(openpods, "openpods.test", QtWarningMsg)

#include "../BasicControlCommand.hpp"
#include "../ble/bleutils.h"

namespace {

QByteArray randomBytes(int maxLen, QRandomGenerator &rng)
{
    int n = static_cast<int>(rng.bounded(maxLen + 1));
    QByteArray b(n, 0);
    for (int i = 0; i < n; ++i) {
        b[i] = static_cast<char>(rng.bounded(256));
    }
    return b;
}

QString randomMacish(QRandomGenerator &rng)
{
    // Mix valid and malformed addresses to exercise the early-return paths.
    static const char hex[] = "0123456789abcdef:GHIJ ";
    constexpr int hexLen = static_cast<int>(sizeof(hex) - 1);
    QString s;
    int n = static_cast<int>(rng.bounded(25));
    for (int i = 0; i < n; ++i) {
        s.append(hex[rng.bounded(hexLen)]);
    }
    return s;
}

} // namespace

class TestStress : public QObject
{
    Q_OBJECT

private:
    // 20000 was a fine smoke level; 50000 across 4 slots = 200k random
    // inputs per ctest run, still <1s under ASAN. The wall budget is
    // dominated by Qt initialization, not the parse loops.
    static constexpr int kIterations = 50000;

private slots:
    void parseActive_fuzzShape()
    {
        auto rng = QRandomGenerator::securelySeeded();
        for (int i = 0; i < kIterations; ++i) {
            // Mix three input shapes: pure-random short, full header +
            // tail, and short prefix of the header.
            QByteArray buf;
            switch (i % 3) {
            case 0:
                buf = randomBytes(20, rng);
                break;
            case 1:
                buf = ControlCommand::HEADER + randomBytes(16, rng);
                break;
            case 2:
                buf = ControlCommand::HEADER.left(rng.bounded(7));
                break;
            default:
                break;
            }
            // Must never crash. Value optionality is fine either way.
            (void)ControlCommand::parseActive(buf);
        }
        QVERIFY(true); // reached without termination
    }

    void parseState_fuzzValueByte()
    {
        auto rng = QRandomGenerator::securelySeeded();
        for (int i = 0; i < kIterations; ++i) {
            QByteArray buf = ControlCommand::HEADER;
            buf.append(static_cast<char>(rng.bounded(256))); // identifier
            buf.append(static_cast<char>(rng.bounded(256))); // value
            buf.append(randomBytes(rng.bounded(8), rng));
            auto s = BasicControlCommand<0x42>::parseState(buf);
            // Spec: nullopt unless value is 0x01 or 0x02.
            quint8 v = static_cast<quint8>(buf.at(7));
            if (v == 0x01) {
                QVERIFY(s.value_or(false));
            } else if (v == 0x02) {
                QCOMPARE(s.value_or(true), false);
            } else {
                QVERIFY(!s.has_value());
            }
        }
    }

    void decryptLastBytes_rejectsShape()
    {
        auto rng = QRandomGenerator::securelySeeded();
        for (int i = 0; i < kIterations; ++i) {
            QByteArray data = randomBytes(40, rng);
            QByteArray key  = randomBytes(20, rng);
            QByteArray out = BLEUtils::decryptLastBytes(data, key);
            // Spec: empty when data <16 or key !=16. Otherwise 16-byte result.
            if (data.size() < 16 || key.size() != 16) {
                QVERIFY(out.isEmpty());
            } else {
                QCOMPARE(out.size(), 16);
            }
        }
    }

    void isValidIrkRpa_doesNotCrash()
    {
        auto rng = QRandomGenerator::securelySeeded();
        for (int i = 0; i < kIterations; ++i) {
            QString rpa = randomMacish(rng);
            QByteArray irk = randomBytes(20, rng);
            // Always returns false on malformed input; never throws.
            (void)BLEUtils::isValidIrkRpa(irk, rpa);
        }
        QVERIFY(true);
    }
};

QTEST_GUILESS_MAIN(TestStress)
#include "tst_stress.moc"
