#include "trayiconmanager.h"
#include <QStandardPaths>
#include <QProcess>

#include <QSystemTrayIcon>
#include <QMenu>
#include <QAction>
#include <QApplication>
#include <QPainter>
#include <QFont>
#include <QColor>
#include <QActionGroup>

using namespace AirpodsTrayApp::Enums;

TrayIconManager::TrayIconManager(QObject *parent) : QObject(parent)
{
    // Initialize tray icon
    trayIcon = new QSystemTrayIcon(QIcon(":/icons/assets/airpods.png"), this);
    // QSystemTrayIcon::setContextMenu does not take ownership. Iter-17
    // tried QObject-reparenting the menu to `this`, but QMenu is a
    // QWidget and the widget-tree machinery doesn't tolerate a non-
    // QWidget parent — destruction crashed in
    // QWidgetPrivate::invalidateGraphicsEffectsRecursively (caught by
    // the iter-28 restart soak). Keep the menu unparented and delete
    // it explicitly from the dtor instead.
    trayMenu = new QMenu();

    // Setup basic menu actions
    setupMenuActions();

    // Connect signals
    trayIcon->setContextMenu(trayMenu);
    connect(trayIcon, &QSystemTrayIcon::activated, this, &TrayIconManager::onTrayIconActivated);

    trayIcon->show();
}

TrayIconManager::~TrayIconManager()
{
    // Explicit cleanup. trayMenu has no QObject parent (see ctor comment);
    // trayIcon is parented to `this` so QObject hierarchy handles it.
    delete trayMenu;
    trayMenu = nullptr;
}

// Three seconds is Qt's own default toast dwell and is what this replaced.
static constexpr int kTrayMessageMs = 3000;

void TrayIconManager::showTrayMessage(const QString &title, const QString &message)
{
    trayIcon->showMessage(title, message, QSystemTrayIcon::Information, kTrayMessageMs);
}

void TrayIconManager::updateBatteryStatus(const QString &status)
{
    trayIcon->setToolTip(tr("Battery Status: ") + status);
    updateIconFromBattery(status);
}

void TrayIconManager::updateNoiseControlState(NoiseControlMode mode)
{
    QList<QAction *> actions = noiseControlGroup->actions();
    for (QAction *action : actions)
    {
        action->setChecked(action->data().toInt() == (int)mode);
    }
}

void TrayIconManager::updateConversationalAwareness(bool enabled)
{
    caToggleAction->setChecked(enabled);
}

void TrayIconManager::setupMenuActions()
{
    // Open action
    QAction *openAction = new QAction(tr("Open"), trayMenu);
    trayMenu->addAction(openAction);
    connect(openAction, &QAction::triggered, qApp, [this](){emit openApp();});

    // Settings Menu

    QAction *settingsMenu = new QAction(tr("Settings"), trayMenu);
    trayMenu->addAction(settingsMenu);
    connect(settingsMenu, &QAction::triggered, qApp, [this](){emit openSettings();});

    trayMenu->addSeparator();

    // Conversational Awareness Toggle
    caToggleAction = new QAction(tr("Toggle Conversational Awareness"), trayMenu);
    caToggleAction->setCheckable(true);
    trayMenu->addAction(caToggleAction);
    connect(caToggleAction, &QAction::triggered, this, [this](bool checked)
            { emit conversationalAwarenessToggled(checked); });

    trayMenu->addSeparator();

    // Noise Control Options
    noiseControlGroup = new QActionGroup(trayMenu);
    const QPair<QString, NoiseControlMode> noiseOptions[] = {
        {tr("Adaptive"), NoiseControlMode::Adaptive},
        {tr("Transparency"), NoiseControlMode::Transparency},
        {tr("Noise Cancellation"), NoiseControlMode::NoiseCancellation},
        {tr("Off"), NoiseControlMode::Off}};

    for (const auto &option : noiseOptions)
    {
        QAction *action = new QAction(option.first, trayMenu);
        action->setCheckable(true);
        action->setData((int)option.second);
        noiseControlGroup->addAction(action);
        trayMenu->addAction(action);
        connect(action, &QAction::triggered, this, [this, mode = option.second]()
                { emit noiseControlChanged(mode); });
    }

    trayMenu->addSeparator();

    // Quit action
    QAction *quitAction = new QAction(tr("Quit"), trayMenu);
    trayMenu->addAction(quitAction);
    connect(quitAction, &QAction::triggered, qApp, &QApplication::quit);
}

void TrayIconManager::updateIconFromBattery(const QString &status)
{
    // Render the AirPods glyph as the tray icon base. Pre-fix this slot
    // painted "N%" white text on a transparent 32x32 pixmap and called
    // setIcon with that — which replaced the AirPods asset entirely so
    // the system tray showed bare text like "31%" with no glyph. Users
    // reasonably read that as a volume widget.
    //
    // Behavior now: keep the AirPods glyph as the icon and let the
    // existing tooltip ("Battery Status: ...") carry the percentages.
    // When at least one source is critically low (<=10%) overlay a
    // small red badge in the bottom-right corner so the low state is
    // visible at a glance without requiring hover.
    int leftLevel = 0;
    int rightLevel = 0;
    int minLevel = 0;

    // Each part is "Label: <n>%". Helper guards against malformed input —
    // a missing ": " separator used to crash via QStringList()[1].
    auto extractLevel = [](const QString &chunk) -> int {
        const QStringList kv = chunk.split(": ");
        if (kv.size() < 2) return 0;
        return QStringView(kv.at(1)).left(kv.at(1).length() - (kv.at(1).endsWith('%') ? 1 : 0))
            .toString().toInt();
    };

    if (!status.isEmpty()) {
        const QStringList parts = status.split(", ");
        if (parts.size() >= 2) {
            leftLevel = extractLevel(parts.at(0));
            rightLevel = extractLevel(parts.at(1));
            minLevel = (leftLevel == 0) ? rightLevel
                     : (rightLevel == 0) ? leftLevel
                     : qMin(leftLevel, rightLevel);
        } else if (parts.size() == 1) {
            minLevel = extractLevel(parts.at(0));
        }
    }

    QPixmap base(":/icons/assets/airpods.png");
    if (base.isNull()) {
        // Asset missing — keep whatever icon was previously set instead
        // of going blank.
        return;
    }
    QPixmap scaled = base.scaled(32, 32, Qt::KeepAspectRatio, Qt::SmoothTransformation);
    QPixmap pixmap(32, 32);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    const int xOff = (pixmap.width()  - scaled.width())  / 2;
    const int yOff = (pixmap.height() - scaled.height()) / 2;
    painter.drawPixmap(xOff, yOff, scaled);

    if (minLevel > 0 && minLevel <= 10) {
        const int badge = 12;
        const int bx = pixmap.width()  - badge - 1;
        const int by = pixmap.height() - badge - 1;
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setBrush(QColor("#d94a4a"));
        painter.setPen(Qt::NoPen);
        painter.drawEllipse(bx, by, badge, badge);
    }
    painter.end();

    trayIcon->setIcon(QIcon(pixmap));
}

void TrayIconManager::onTrayIconActivated(QSystemTrayIcon::ActivationReason reason)
{
    if (reason == QSystemTrayIcon::Trigger)
    {
        emit trayClicked();
    }
    else if (reason == QSystemTrayIcon::MiddleClick)
    {
        emit middleClicked();
    }
}

