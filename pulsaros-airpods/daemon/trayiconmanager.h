#include <QObject>
#include <QSystemTrayIcon>

#include "enums.h"

class QMenu;
class QAction;
class QActionGroup;

class TrayIconManager : public QObject
{
    Q_OBJECT

public:
    explicit TrayIconManager(QObject *parent = nullptr);
    ~TrayIconManager() override;

    void updateBatteryStatus(const QString &status);

    void updateNoiseControlState(AirpodsTrayApp::Enums::NoiseControlMode);

    void updateConversationalAwareness(bool enabled);

    // Notifier's fallback when omarchy is not on PATH; Notifier owns the enabled check.
    void showTrayMessage(const QString &title, const QString &message);

    void resetTrayIcon()
    {
        trayIcon->setIcon(QIcon(":/icons/assets/airpods.png"));
        trayIcon->setToolTip("");
    }

signals:
    void trayClicked();
    // Middle-click on tray icon. The OpenPods loop spec maps this to a
    // noise-control mode cycle (matches the macOS Control Center
    // scroll-on-icon shortcut). main.cpp owns the cycle logic — the
    // manager just forwards the activation reason.
    void middleClicked();
    void noiseControlChanged(AirpodsTrayApp::Enums::NoiseControlMode);
    void conversationalAwarenessToggled(bool enabled);
    void openApp();
    void openSettings();

private slots:
    void onTrayIconActivated(QSystemTrayIcon::ActivationReason reason);

private:
    QSystemTrayIcon *trayIcon = nullptr;
    QMenu *trayMenu = nullptr;
    QAction *caToggleAction = nullptr;
    QActionGroup *noiseControlGroup = nullptr;

    void setupMenuActions();
    void updateIconFromBattery(const QString &status);
};