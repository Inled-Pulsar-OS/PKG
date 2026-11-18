#include "bleutils.h"

#include <openssl/evp.h>
#include "deviceinfo.hpp"
#include <QDebug>
#include <QByteArray>
#include <QtEndian>
#include <QCryptographicHash>
#include <cstring>

namespace {

// Helper: AES-128 ECB single-block encrypt via OpenSSL 3.x EVP.
// Returns 16-byte ciphertext on success, empty QByteArray on failure.
QByteArray aes128EcbEncryptBlock(const QByteArray &key, const QByteArray &plaintext)
{
    if (key.size() != 16 || plaintext.size() != 16) {
        return QByteArray();
    }
    EVP_CIPHER_CTX *ctx = EVP_CIPHER_CTX_new();
    if (!ctx) return QByteArray();

    QByteArray out(32, '\0'); // worst-case (one block + padding); we disable padding
    int outLen = 0, finalLen = 0;
    QByteArray result;

    do {
        if (EVP_EncryptInit_ex(ctx, EVP_aes_128_ecb(), nullptr,
                               reinterpret_cast<const unsigned char *>(key.constData()),
                               nullptr) != 1) {
            qWarning() << "EVP_EncryptInit_ex failed";
            break;
        }
        EVP_CIPHER_CTX_set_padding(ctx, 0);
        if (EVP_EncryptUpdate(ctx,
                              reinterpret_cast<unsigned char *>(out.data()), &outLen,
                              reinterpret_cast<const unsigned char *>(plaintext.constData()),
                              plaintext.size()) != 1) {
            qWarning() << "EVP_EncryptUpdate failed";
            break;
        }
        if (EVP_EncryptFinal_ex(ctx,
                                reinterpret_cast<unsigned char *>(out.data()) + outLen,
                                &finalLen) != 1) {
            qWarning() << "EVP_EncryptFinal_ex failed";
            break;
        }
        result = out.left(outLen + finalLen);
    } while (false);

    EVP_CIPHER_CTX_free(ctx);
    return result;
}

// Helper: AES-128 CBC decrypt with zero IV, no padding. This is the
// Apple-specified construction for the "Magic Cloud Keys" inner block —
// not a generic CBC use, so the zero IV is intentional and matches the
// counterpart on the AirPods.
QByteArray aes128CbcDecryptZeroIv(const QByteArray &key, const QByteArray &block)
{
    if (key.size() != 16 || block.size() != 16) {
        return QByteArray();
    }
    EVP_CIPHER_CTX *ctx = EVP_CIPHER_CTX_new();
    if (!ctx) return QByteArray();

    unsigned char iv[16] = {0};
    QByteArray out(32, '\0');
    int outLen = 0, finalLen = 0;
    QByteArray result;

    do {
        if (EVP_DecryptInit_ex(ctx, EVP_aes_128_cbc(), nullptr,
                               reinterpret_cast<const unsigned char *>(key.constData()),
                               iv) != 1) {
            qWarning() << "EVP_DecryptInit_ex failed";
            break;
        }
        EVP_CIPHER_CTX_set_padding(ctx, 0);
        if (EVP_DecryptUpdate(ctx,
                              reinterpret_cast<unsigned char *>(out.data()), &outLen,
                              reinterpret_cast<const unsigned char *>(block.constData()),
                              block.size()) != 1) {
            qWarning() << "EVP_DecryptUpdate failed";
            break;
        }
        if (EVP_DecryptFinal_ex(ctx,
                                reinterpret_cast<unsigned char *>(out.data()) + outLen,
                                &finalLen) != 1) {
            qWarning() << "EVP_DecryptFinal_ex failed";
            break;
        }
        result = out.left(outLen + finalLen);
    } while (false);

    EVP_CIPHER_CTX_free(ctx);
    return result;
}

} // namespace (anonymous internal helpers)

// Public BLEUtils namespace: tied to bleutils.h. Static helpers only;
// no instance state to maintain so the previous QObject scaffolding
// + Q_INVOKABLE was pure MOC overhead (review iter-79). Kept the e()
// + ah() helpers internal-linkage via the anonymous namespace block
// above to keep the public surface minimal.
namespace BLEUtils
{

// e() / ah() were previously class-static; move them to file-static
// (anonymous-namespace) here since they're not part of the public API.
static QByteArray e(const QByteArray &key, const QByteArray &data);
static QByteArray ah(const QByteArray &k, const QByteArray &r);

bool verifyRPA(const QString &address, const QByteArray &irk)
{
    if (address.isEmpty() || irk.isEmpty() || irk.size() != 16) {
        return false;
    }

    QStringList parts = address.split(':');
    if (parts.size() != 6) {
        return false;
    }

    QByteArray rpa;
    bool ok;
    for (int i = parts.size() - 1; i >= 0; --i) {
        rpa.append(static_cast<char>(parts[i].toInt(&ok, 16)));
        if (!ok) return false;
    }
    if (rpa.size() != 6) return false;

    QByteArray prand = rpa.mid(3, 3);
    QByteArray hash = rpa.left(3);
    QByteArray computedHash = ah(irk, prand);

    return hash == computedHash;
}

bool isValidIrkRpa(const QByteArray &irk, const QString &rpa)
{
    return verifyRPA(rpa, irk);
}

static QByteArray e(const QByteArray &key, const QByteArray &data)
{
    if (key.size() != 16 || data.size() != 16) {
        return QByteArray();
    }

    // Bluetooth Core spec defines AES-128 with both key and data in
    // big-endian, so we reverse from the host (little-endian) byte
    // order before and after the call.
    QByteArray reversedKey(key);
    std::reverse(reversedKey.begin(), reversedKey.end());

    QByteArray reversedData(data);
    std::reverse(reversedData.begin(), reversedData.end());

    QByteArray result = aes128EcbEncryptBlock(reversedKey, reversedData);
    if (result.size() != 16) {
        return QByteArray();
    }
    std::reverse(result.begin(), result.end());
    return result;
}

static QByteArray ah(const QByteArray &k, const QByteArray &r)
{
    if (r.size() < 3) {
        return QByteArray();
    }

    QByteArray rPadded(16, 0);
    rPadded.replace(0, 3, r.left(3));

    QByteArray encrypted = e(k, rPadded);
    if (encrypted.isEmpty()) {
        return QByteArray();
    }
    return encrypted.left(3);
}

QByteArray decryptLastBytes(const QByteArray &data, const QByteArray &key)
{
    if (data.size() < 16 || key.size() != 16) {
        qWarning() << "decryptLastBytes: data size <16 or key size !=16";
        return QByteArray();
    }
    return aes128CbcDecryptZeroIv(key, data.right(16));
}

} // namespace BLEUtils
