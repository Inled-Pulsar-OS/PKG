// Regression test for QRCodeImageProvider.hpp include-guard.
//
// Pre-fix, the header lacked `#pragma once`. Including it twice in the
// same translation unit (which can happen via transitive includes)
// produces "redefinition of class" errors at compile time. We force the
// double-include here; if the guard goes missing again, this test fails
// to compile and the gate goes red.
//
// Also exercises a basic happy-path: construct the provider, ask for an
// image, verify dimensions match the QR scale factor.

#include <QTest>
#include <QImage>
#include <QSize>

#include "../QRCodeImageProvider.hpp"
#include "../QRCodeImageProvider.hpp" // intentional: validates #pragma once

class TestQRCodeImageProvider : public QObject
{
    Q_OBJECT

private slots:
    void constructs()
    {
        QRCodeImageProvider provider;
        QCOMPARE(provider.imageType(), QQuickImageProvider::Image);
    }

    void rejectsMalformedId()
    {
        QRCodeImageProvider provider;
        QSize out;
        QImage img = provider.requestImage("missing-semicolon", &out, QSize());
        QVERIFY(img.isNull());
    }

    void rejectsExtraSegments()
    {
        QRCodeImageProvider provider;
        QSize out;
        QImage img = provider.requestImage("a;b;c", &out, QSize());
        QVERIFY(img.isNull());
    }

    void rendersForValidPair()
    {
        QRCodeImageProvider provider;
        QSize out;
        QImage img = provider.requestImage(
            "00112233445566778899aabbccddeeff;ffeeddccbbaa99887766554433221100",
            &out, QSize());

        QVERIFY(!img.isNull());
        QCOMPARE(img.size(), out);
        // Scale factor in the provider is 8; QR matrix is at least 21
        // modules (Version 1). So the rendered image must be >= 168x168.
        QVERIFY(img.width() >= 168);
        QCOMPARE(img.width(), img.height());
        // Format is RGB32 per the implementation.
        QCOMPARE(img.format(), QImage::Format_RGB32);
    }
};

QTEST_MAIN(TestQRCodeImageProvider)
#include "tst_qrcodeimageprovider.moc"
