#pragma once

#include <QByteArray>
#include <QString>

// BLE Identity-Resolving-Key + Resolvable-Private-Address utilities.
// Plain namespace — no Qt metatype state, no signals, no QObject
// inheritance. Previously declared as `class BLEUtils : public QObject`
// with Q_OBJECT + Q_INVOKABLE static methods + a never-invoked
// constructor. None of the static functions need an instance and
// BLEUtils is never registered with QML (grep confirmed: no
// qmlRegisterType, no `new BLEUtils`, no instance reference outside
// the generated MOC), so the QObject scaffolding was pure dead weight
// — Q_OBJECT pulls in MOC output for every TU that includes this
// header. Converting to a namespace drops the MOC overhead without
// changing the call-site syntax (`BLEUtils::isValidIrkRpa(...)` reads
// the same way in either form).
namespace BLEUtils
{
    // Verify that `address` is an RPA matching the given Identity
    // Resolving Key (IRK). Address is a colon-separated MAC string
    // (e.g. "AA:BB:CC:DD:EE:FF"); IRK is a 16-byte QByteArray. Returns
    // false on any shape mismatch — never throws.
    bool verifyRPA(const QString &address, const QByteArray &irk);

    // Convenience wrapper around verifyRPA with swapped argument order
    // (irk first) to match Apple's "isValid(IRK, RPA)" convention used
    // in the AAP packet handling code. Kept as a separate name so the
    // call site reads naturally at both ends of the protocol stack.
    bool isValidIrkRpa(const QByteArray &irk, const QString &rpa);

    // AES-128 ECB decrypt of the final 16 bytes of `data` using `key`.
    // Returns the 16-byte plaintext on success, or an empty QByteArray
    // on any shape failure (data < 16 bytes or key != 16 bytes) or
    // OpenSSL error. Never throws.
    QByteArray decryptLastBytes(const QByteArray &data, const QByteArray &key);
}
