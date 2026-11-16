#pragma once

#include <QQuickImageProvider>
#include <QPainter>
#include "thirdparty/QR-Code-generator/qrcodegen.hpp"

class QRCodeImageProvider : public QQuickImageProvider
{
public:
    QRCodeImageProvider() : QQuickImageProvider(QQuickImageProvider::Image) {}

    QImage requestImage(const QString &id, QSize *size, const QSize &requestedSize) override
    {
        // Parse the keys from id (format: "encKey;irk")
        QStringList keys = id.split(';');
        if (keys.size() != 2)
            return QImage();

        // OpenPods (formerly librepods) magic-keys handoff. URL scheme
        // CONTRACT: the OpenPods Android client recognizes the literal
        // `librepods://` scheme + `add-magic-keys` path + `enc_key`,
        // `irk` query params. Renaming any of these requires coordinated
        // bump on the Android side (see android/ — off-limits per loop
        // spec). Until then the scheme stays librepods://.
        QString data = QString("librepods://add-magic-keys?enc_key=%1&irk=%2").arg(keys[0], keys[1]);

        // Generate QR code using the existing qrcodegen library
        qrcodegen::QrCode qr = qrcodegen::QrCode::encodeText(data.toUtf8().constData(), qrcodegen::QrCode::Ecc::MEDIUM);

        int scale = 8;
        QImage image(qr.getSize() * scale, qr.getSize() * scale, QImage::Format_RGB32);
        image.fill(Qt::white);

        QPainter painter(&image);
        painter.setPen(Qt::NoPen);
        painter.setBrush(Qt::black);

        for (int y = 0; y < qr.getSize(); y++)
        {
            for (int x = 0; x < qr.getSize(); x++)
            {
                if (qr.getModule(x, y))
                {
                    painter.drawRect(x * scale, y * scale, scale, scale);
                }
            }
        }

        if (size)
            *size = image.size();
        return image;
    }
};