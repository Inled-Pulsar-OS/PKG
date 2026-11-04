#ifndef AUTOSTARTMANAGER_HPP
#define AUTOSTARTMANAGER_HPP

#include <QObject>
#include <QSettings>
#include <QStandardPaths>
#include <QFile>
#include <QSaveFile>
#include <QDir>
#include <QCoreApplication>

class AutoStartManager : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool autoStartEnabled READ autoStartEnabled WRITE setAutoStartEnabled NOTIFY autoStartEnabledChanged)

public:
    explicit AutoStartManager(QObject *parent = nullptr) : QObject(parent)
    {
        QString autostartDir = QStandardPaths::writableLocation(QStandardPaths::ConfigLocation) + "/autostart";
        QDir().mkpath(autostartDir);
        m_autostartFilePath = autostartDir + "/" + QCoreApplication::applicationName() + ".desktop";
    }

    bool autoStartEnabled() const
    {
        return QFile::exists(m_autostartFilePath);
    }

    void setAutoStartEnabled(bool enabled)
    {
        if (autoStartEnabled() == enabled)
        {
            return;
        }

        if (enabled)
        {
            createAutoStartEntry();
        }
        else
        {
            removeAutoStartEntry();
        }

        emit autoStartEnabledChanged(enabled);
    }

private:
    void createAutoStartEntry()
    {
        QString appPath = QCoreApplication::applicationFilePath();
        if (appPath.contains(' ')) {
            appPath = "\"" + appPath + "\"";
        }

        const QString content = QStringLiteral(
            "[Desktop Entry]\n"
            "Type=Application\n"
            "Name=%1\n"
            "Exec=%2 --hide\n"
            "Icon=%3\n"
            "Comment=%4\n"
            "X-GNOME-Autostart-enabled=true\n"
            "Terminal=false\n")
            .arg(
                QCoreApplication::applicationName(),
                appPath,
                QCoreApplication::applicationName().toLower(),
                QCoreApplication::applicationName() + " autostart");

        // QSaveFile writes a randomly named temporary and renames it over the target, so
        // no reader sees half a file and no symlink at a guessable path takes the write.
        QSaveFile file(m_autostartFilePath);
        if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
            qWarning() << "Failed to open autostart file:" << file.errorString();
            return;
        }
        const QByteArray payload = content.toUtf8();
        if (file.write(payload) != payload.size()) {
            qWarning() << "Short write to autostart file";
            file.cancelWriting();
            return;
        }
        if (!file.commit()) {
            qWarning() << "Failed to write autostart file:" << file.errorString();
        }
    }

    void removeAutoStartEntry()
    {
        // Unconditional remove — exists() then remove() is a TOCTOU race;
        // QFile::remove returns false harmlessly if the file is gone.
        QFile::remove(m_autostartFilePath);
    }

    QString m_autostartFilePath;

signals:
    void autoStartEnabledChanged(bool enabled);
};

#endif // AUTOSTARTMANAGER_HPP