#pragma once

#include <QByteArray>
#include <optional>

// Control Command Header
namespace ControlCommand
{
    // inline (C++17) instead of static so all TUs share one definition.
    inline const QByteArray HEADER = QByteArray::fromHex("040004000900");

    // Helper function to create control command packets
    inline QByteArray createCommand(quint8 identifier, quint8 data1 = 0x00, quint8 data2 = 0x00,
                                    quint8 data3 = 0x00, quint8 data4 = 0x00)
    {
        QByteArray packet = HEADER;
        packet.append(static_cast<char>(identifier));
        packet.append(static_cast<char>(data1));
        packet.append(static_cast<char>(data2));
        packet.append(static_cast<char>(data3));
        packet.append(static_cast<char>(data4));
        return packet;
    }

    inline std::optional<char> parseActive(const QByteArray &data)
    {
        // Bounds check before .at(7) — startsWith(HEADER) only verifies
        // the first 6 bytes; data could still be 6 or 7 bytes and the
        // .at(7) would UB. A well-formed control command is
        // HEADER (6) + identifier (1) + value (1) = 8 bytes minimum.
        if (data.size() <= 7 || !data.startsWith(ControlCommand::HEADER))
            return std::nullopt;

        return static_cast<quint8>(data.at(7));
    }
}

template <quint8 CommandId>
struct BasicControlCommand
{
    static constexpr quint8 ID = CommandId;
    static const QByteArray HEADER;

    static const QByteArray ENABLED;
    static const QByteArray DISABLED;

    static QByteArray create(quint8 data1 = 0x00, quint8 data2 = 0x00,
                             quint8 data3 = 0x00, quint8 data4 = 0x00)
    {
        return ControlCommand::createCommand(ID, data1, data2, data3, data4);
    }

    // Basically returns the byte at the index 7
    static std::optional<bool> parseState(const QByteArray &data)
    {
        switch (ControlCommand::parseActive(data).value_or(0x00))
        {
        case 0x01: // Enabled
            return true;
        case 0x02: // Disabled
            return false;
        default:
            return std::nullopt;
        }
    }

    static std::optional<char> getValue(const QByteArray &data)
    {
        return ControlCommand::parseActive(data);
    }
};

template <quint8 CommandId>
const QByteArray BasicControlCommand<CommandId>::HEADER = ControlCommand::HEADER + static_cast<char>(CommandId);

template <quint8 CommandId>
const QByteArray BasicControlCommand<CommandId>::ENABLED = create(0x01);

template <quint8 CommandId>
const QByteArray BasicControlCommand<CommandId>::DISABLED = create(0x02);