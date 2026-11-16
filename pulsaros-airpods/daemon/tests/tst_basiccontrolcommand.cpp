// Regression for BasicControlCommand.hpp bounds-check + #pragma once.
//
// Pre-fix, parseActive() did `data.at(7)` after only verifying the
// first 6 bytes via startsWith(HEADER). A 6- or 7-byte packet that
// happened to start with the header would UB. New code rejects size
// <= 7 up front.
//
// We also intentionally double-include the header in this TU; without
// the pragma the file would multiply-define the inline namespace
// values and the test would fail to compile.

#include <QTest>
#include <QByteArray>

#include "../BasicControlCommand.hpp"
#include "../BasicControlCommand.hpp" // intentional: validates #pragma once

class TestBasicControlCommand : public QObject
{
    Q_OBJECT

private slots:
    void parseActive_rejectsEmpty()
    {
        QVERIFY(!ControlCommand::parseActive(QByteArray()).has_value());
    }

    void parseActive_rejectsShortHeaderOnly()
    {
        // exactly HEADER (6 bytes) — startsWith passes but .at(7) UB
        QVERIFY(!ControlCommand::parseActive(ControlCommand::HEADER).has_value());
    }

    void parseActive_rejectsSevenBytes()
    {
        QByteArray buf = ControlCommand::HEADER + QByteArray(1, 0x42);
        QCOMPARE(buf.size(), 7);
        QVERIFY(!ControlCommand::parseActive(buf).has_value());
    }

    void parseActive_rejectsWrongHeader()
    {
        QByteArray buf = QByteArray::fromHex("ffeeddccbbaa") + QByteArray(2, 0x01);
        QVERIFY(!ControlCommand::parseActive(buf).has_value());
    }

    void parseActive_returnsByteAt7()
    {
        QByteArray buf = ControlCommand::HEADER;
        buf.append(static_cast<char>(0x42)); // identifier (idx 6)
        buf.append(static_cast<char>(0x01)); // value     (idx 7)
        // value_or sidesteps clang-tidy's bugprone-unchecked-optional-access
        // false-positive when chained after QVERIFY(has_value).
        QCOMPARE(static_cast<quint8>(ControlCommand::parseActive(buf).value_or(0)),
                 quint8(0x01));
    }

    void parseState_enabled()
    {
        auto pkt = BasicControlCommand<0x42>::create(0x01);
        QCOMPARE(BasicControlCommand<0x42>::parseState(pkt).value_or(false), true);
    }

    void parseState_disabled()
    {
        auto pkt = BasicControlCommand<0x42>::create(0x02);
        // value_or(true) so if parseState returned nullopt we'd see "true"
        // and the QCOMPARE would still fail, distinguishing from a real
        // "false" return.
        QCOMPARE(BasicControlCommand<0x42>::parseState(pkt).value_or(true), false);
    }

    void parseState_unknownValueIsNullopt()
    {
        // 0x05 isn't enabled/disabled — parseState should return nullopt
        auto pkt = BasicControlCommand<0x42>::create(0x05);
        QVERIFY(!BasicControlCommand<0x42>::parseState(pkt).has_value());
    }

    void create_buildsExpectedShape()
    {
        // HEADER (6) + ID (1) + data1..data4 (4) = 11 bytes
        auto pkt = BasicControlCommand<0x42>::create(0xAA, 0xBB, 0xCC, 0xDD);
        QCOMPARE(pkt.size(), 11);
        QVERIFY(pkt.startsWith(ControlCommand::HEADER));
        QCOMPARE(static_cast<quint8>(pkt.at(6)), quint8(0x42));
        QCOMPARE(static_cast<quint8>(pkt.at(7)), quint8(0xAA));
        QCOMPARE(static_cast<quint8>(pkt.at(8)), quint8(0xBB));
        QCOMPARE(static_cast<quint8>(pkt.at(9)), quint8(0xCC));
        QCOMPARE(static_cast<quint8>(pkt.at(10)), quint8(0xDD));
    }
};

QTEST_GUILESS_MAIN(TestBasicControlCommand)
#include "tst_basiccontrolcommand.moc"
