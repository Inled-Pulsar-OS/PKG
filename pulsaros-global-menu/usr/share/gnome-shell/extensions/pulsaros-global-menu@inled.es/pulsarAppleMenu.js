import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as ModalDialog from 'resource:///org/gnome/shell/ui/modalDialog.js';
import St from 'gi://St';
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Pango from 'gi://Pango';
import * as Appearance from './appearance.js';

const I18N = {
    en: {
        aboutPulsar: "About Pulsar OS",
        systemSettings: "System Settings...",
        appStore: "App Store...",
        lockScreen: "Lock Screen",
        logOut: "Log Out...",
        sleep: "Sleep",
        restart: "Restart...",
        shutDown: "Shut Down...",
        aboutApp: "About %s",
        hideApp: "Hide %s",
        hideOthers: "Hide Others",
        showAll: "Show All",
        quitApp: "Quit %s",
        file: "File",
        newWindow: "New Window",
        closeWindow: "Close Window",
        edit: "Edit",
        undo: "Undo",
        redo: "Redo",
        cut: "Cut",
        copy: "Copy",
        paste: "Paste",
        go: "Go",
        back: "Back",
        home: "Home",
        documents: "Documents",
        downloads: "Downloads",
        pictures: "Pictures",
        window: "Window",
        minimize: "Minimize",
        maximize: "Maximize",
        close: "Close",
        help: "Help",
        confirmRestartTitle: "Are you sure you want to restart your computer now?",
        confirmShutdownTitle: "Are you sure you want to shut down your computer now?",
        confirmRestartDesc: "If you do nothing, the computer will restart automatically in %d seconds.",
        confirmShutdownDesc: "If you do nothing, the computer will shut down automatically in %d seconds.",
        reopenWindows: "Reopen windows when logging in",
        cancel: "Cancel",
        restartBtn: "Restart",
        shutDownBtn: "Shut Down",
        deviceName: "Device Name:",
        processor: "Processor:",
        memory: "Memory:",
        graphics: "Graphics:",
        storage: "Storage:",
        copyInfo: "Copy Info",
        viewLogs: "View Logs",
        version: "Version",
        themeLogoutTitle: "Appearance Changed",
        themeLogoutDesc: "Logging out in %d seconds to apply the new appearance completely.",
        logoutNowBtn: "Log Out Now",
    },
    es: {
        aboutPulsar: "Acerca de Pulsar OS",
        systemSettings: "Ajustes del Sistema...",
        appStore: "App Store...",
        lockScreen: "Bloquear Pantalla",
        logOut: "Cerrar Sesión...",
        sleep: "Reposo",
        restart: "Reiniciar...",
        shutDown: "Apagar...",
        aboutApp: "Acerca de %s",
        hideApp: "Ocultar %s",
        hideOthers: "Ocultar Otros",
        showAll: "Mostrar Todo",
        quitApp: "Salir de %s",
        file: "Archivo",
        newWindow: "Nueva Ventana",
        closeWindow: "Cerrar Ventana",
        edit: "Edición",
        undo: "Deshacer",
        redo: "Rehacer",
        cut: "Cortar",
        copy: "Copiar",
        paste: "Pegar",
        go: "Ir",
        back: "Atrás",
        home: "Inicio",
        documents: "Documentos",
        downloads: "Descargas",
        pictures: "Imágenes",
        window: "Ventana",
        minimize: "Minimizar",
        maximize: "Maximizar",
        close: "Cerrar",
        help: "Ayuda",
        confirmRestartTitle: "¿Seguro que quieres reiniciar el ordenador ahora?",
        confirmShutdownTitle: "¿Seguro que quieres apagar el ordenador ahora?",
        confirmRestartDesc: "Si no haces nada, el ordenador se reiniciará automáticamente en %d segundos.",
        confirmShutdownDesc: "Si no haces nada, el ordenador se apagará automáticamente en %d segundos.",
        reopenWindows: "Volver a abrir las ventanas al reiniciar sesión",
        cancel: "Cancelar",
        restartBtn: "Reiniciar",
        shutDownBtn: "Apagar",
        deviceName: "Nombre del dispositivo:",
        processor: "Procesador:",
        memory: "Memoria:",
        graphics: "Gráficos:",
        storage: "Almacenamiento:",
        copyInfo: "Copiar información",
        viewLogs: "Ver registros",
        version: "Versión",
        themeLogoutTitle: "Cambio de aspecto aplicado",
        themeLogoutDesc: "Se cerrará la sesión en %d segundos para aplicar el nuevo tema por completo.",
        logoutNowBtn: "Cerrar sesión ahora",
    },
    fr: {
        aboutPulsar: "À propos de Pulsar OS",
        systemSettings: "Réglages Système...",
        appStore: "App Store...",
        lockScreen: "Verrouiller l'écran",
        logOut: "Fermer la session...",
        sleep: "Suspendre",
        restart: "Redémarrer...",
        shutDown: "Éteindre...",
        aboutApp: "À propos de %s",
        hideApp: "Masquer %s",
        hideOthers: "Masquer les autres",
        showAll: "Tout afficher",
        quitApp: "Quitter %s",
        file: "Fichier",
        newWindow: "Nouvelle fenêtre",
        closeWindow: "Fermer la fenêtre",
        edit: "Édition",
        undo: "Annuler",
        redo: "Rétablir",
        cut: "Couper",
        copy: "Copier",
        paste: "Coller",
        go: "Aller",
        back: "Retour",
        home: "Départ",
        documents: "Documents",
        downloads: "Téléchargements",
        pictures: "Images",
        window: "Fenêtre",
        minimize: "Réduire",
        maximize: "Agrandir",
        close: "Fermer",
        help: "Aide",
        confirmRestartTitle: "Voulez-vous vraiment redémarrer votre ordinateur maintenant ?",
        confirmShutdownTitle: "Voulez-vous vraiment éteindre votre ordinateur maintenant ?",
        confirmRestartDesc: "Sans action de votre part, l'ordinateur redémarrera automatiquement dans %d secondes.",
        confirmShutdownDesc: "Sans action de votre part, l'ordinateur s'éteindra automatiquement dans %d secondes.",
        reopenWindows: "Rouvrir toutes les fenêtres à la réouverture de session",
        cancel: "Annuler",
        restartBtn: "Redémarrer",
        shutDownBtn: "Éteindre",
        deviceName: "Nom de l'appareil :",
        processor: "Processeur :",
        memory: "Mémoire :",
        graphics: "Graphismes :",
        storage: "Stockage :",
        copyInfo: "Copier les infos",
        version: "Version",
    },
    de: {
        aboutPulsar: "Über Pulsar OS",
        systemSettings: "Systemeinstellungen...",
        appStore: "App Store...",
        lockScreen: "Bildschirm sperren",
        logOut: "Abmelden...",
        sleep: "Ruhezustand",
        restart: "Neustart...",
        shutDown: "Ausschalten...",
        aboutApp: "Über %s",
        hideApp: "%s ausblenden",
        hideOthers: "Andere ausblenden",
        showAll: "Alle einblenden",
        quitApp: "%s beenden",
        file: "Ablage",
        newWindow: "Neues Fenster",
        closeWindow: "Fenster schließen",
        edit: "Bearbeiten",
        undo: "Widerrufen",
        redo: "Wiederholen",
        cut: "Ausschneiden",
        copy: "Kopieren",
        paste: "Einsetzen",
        go: "Gehe zu",
        back: "Zurück",
        home: "Benutzerordner",
        documents: "Dokumente",
        downloads: "Downloads",
        pictures: "Bilder",
        window: "Fenster",
        minimize: "Minimieren",
        maximize: "Maximieren",
        close: "Schließen",
        help: "Hilfe",
        confirmRestartTitle: "Möchtest du deinen Computer jetzt wirklich neu starten?",
        confirmShutdownTitle: "Möchtest du deinen Computer jetzt wirklich ausschalten?",
        confirmRestartDesc: "Wenn du nichts tust, startet der Computer in %d Sekunden automatisch neu.",
        confirmShutdownDesc: "Wenn du nichts tust, schaltet sich der Computer in %d Sekunden automatisch aus.",
        reopenWindows: "Beim nächsten Anmelden alle Fenster wieder öffnen",
        cancel: "Abbrechen",
        restartBtn: "Neustart",
        shutDownBtn: "Ausschalten",
        deviceName: "Gerätename:",
        processor: "Prozessor:",
        memory: "Speicher:",
        graphics: "Grafik:",
        storage: "Festplatte:",
        copyInfo: "Info kopieren",
        version: "Version",
    },
    it: {
        aboutPulsar: "Informazioni su Pulsar OS",
        systemSettings: "Impostazioni di Sistema...",
        appStore: "App Store...",
        lockScreen: "Blocca schermo",
        logOut: "Esci...",
        sleep: "Stop",
        restart: "Riavvia...",
        shutDown: "Spegni...",
        aboutApp: "Informazioni su %s",
        hideApp: "Nascondi %s",
        hideOthers: "Nascondi altre",
        showAll: "Mostra tutte",
        quitApp: "Esci da %s",
        file: "File",
        newWindow: "Nuova finestra",
        closeWindow: "Chiudi finestra",
        edit: "Modifica",
        undo: "Annulla",
        redo: "Ripristina",
        cut: "Taglia",
        copy: "Copia",
        paste: "Incolla",
        go: "Vai",
        back: "Indietro",
        home: "Inizio",
        documents: "Documenti",
        downloads: "Download",
        pictures: "Immagini",
        window: "Finestra",
        minimize: "Contrai",
        maximize: "Ingrandisci",
        close: "Chiudi",
        help: "Aiuto",
        confirmRestartTitle: "Sei sicuro di voler riavviare il computer adesso?",
        confirmShutdownTitle: "Sei sicuro di voler spegnere il computer adesso?",
        confirmRestartDesc: "Se non esegui alcuna operazione, il computer si riavvierà automaticamente tra %d secondi.",
        confirmShutdownDesc: "Se non esegui alcuna operazione, il computer si spegnerà automaticamente tra %d secondi.",
        reopenWindows: "Riapri le finestre al login successivo",
        cancel: "Annulla",
        restartBtn: "Riavvia",
        shutDownBtn: "Spegni",
        deviceName: "Nome dispositivo:",
        processor: "Processore:",
        memory: "Memoria:",
        graphics: "Grafica:",
        storage: "Spazio:",
        copyInfo: "Copia info",
        version: "Versione",
    },
    pt: {
        aboutPulsar: "Acerca do Pulsar OS",
        systemSettings: "Definições do Sistema...",
        appStore: "App Store...",
        lockScreen: "Bloquear ecrã",
        logOut: "Terminar sessão...",
        sleep: "Suspender",
        restart: "Reiniciar...",
        shutDown: "Desligar...",
        aboutApp: "Acerca de %s",
        hideApp: "Ocultar %s",
        hideOthers: "Ocultar outras",
        showAll: "Mostrar tudo",
        quitApp: "Sair do %s",
        file: "Ficheiro",
        newWindow: "Nova janela",
        closeWindow: "Fechar janela",
        edit: "Editar",
        undo: "Desfazer",
        redo: "Refazer",
        cut: "Cortar",
        copy: "Copiar",
        paste: "Colar",
        go: "Ir",
        back: "Recuar",
        home: "Pasta pessoal",
        documents: "Documentos",
        downloads: "Descargas",
        pictures: "Imagens",
        window: "Janela",
        minimize: "Minimizar",
        maximize: "Maximizar",
        close: "Fechar",
        help: "Ajuda",
        confirmRestartTitle: "Tem a certeza de que pretende reiniciar o computador agora?",
        confirmShutdownTitle: "Tem a certeza de que pretende desligar o computador agora?",
        confirmRestartDesc: "Se não fizer nada, o computador será reiniciado automaticamente em %d segundos.",
        confirmShutdownDesc: "Se não fizer nada, o computador será desligado automaticamente em %d segundos.",
        reopenWindows: "Reabrir janelas ao iniciar sessão",
        cancel: "Cancelar",
        restartBtn: "Reiniciar",
        shutDownBtn: "Desligar",
        deviceName: "Nome do dispositivo:",
        processor: "Processador:",
        memory: "Memória:",
        graphics: "Placa gráfica:",
        storage: "Armazenamento:",
        copyInfo: "Copiar informações",
        version: "Versão",
    },
    zh: {
        aboutPulsar: "关于 Pulsar OS",
        systemSettings: "系统设置...",
        appStore: "App Store...",
        lockScreen: "锁定屏幕",
        logOut: "退出登录...",
        sleep: "睡眠",
        restart: "重新启动...",
        shutDown: "关机...",
        aboutApp: "关于 %s",
        hideApp: "隐藏 %s",
        hideOthers: "隐藏其他",
        showAll: "显示全部",
        quitApp: "退出 %s",
        file: "文件",
        newWindow: "新建窗口",
        closeWindow: "关闭窗口",
        edit: "编辑",
        undo: "撤销",
        redo: "重做",
        cut: "剪切",
        copy: "拷贝",
        paste: "粘贴",
        go: "前往",
        back: "返回",
        home: "个人目录",
        documents: "文稿",
        downloads: "下载",
        pictures: "图片",
        window: "窗口",
        minimize: "最小化",
        maximize: "最大化",
        close: "关闭",
        help: "帮助",
        confirmRestartTitle: "您确定现在要重新启动电脑吗？",
        confirmShutdownTitle: "您确定现在要将电脑关机吗？",
        confirmRestartDesc: "如果您不采取任何操作，电脑将在 %d 秒后自动重新启动。",
        confirmShutdownDesc: "如果您不采取任何操作，电脑将在 %d 秒后自动关机。",
        reopenWindows: "再次登录时重新打开窗口",
        cancel: "取消",
        restartBtn: "重新启动",
        shutDownBtn: "关机",
        deviceName: "设备名称：",
        processor: "处理器：",
        memory: "内存：",
        graphics: "图形卡：",
        storage: "存储空间：",
        copyInfo: "拷贝信息",
        version: "版本",
    },
    ja: {
        aboutPulsar: "このコンピュータについて",
        systemSettings: "システム設定...",
        appStore: "App Store...",
        lockScreen: "画面をロック",
        logOut: "ログアウト...",
        sleep: "スリープ",
        restart: "再起動...",
        shutDown: "システム終了...",
        aboutApp: "%s について",
        hideApp: "%s を非表示",
        hideOthers: "ほかを非表示",
        showAll: "すべてを表示",
        quitApp: "%s を終了",
        file: "ファイル",
        newWindow: "新規ウインドウ",
        closeWindow: "ウインドウを閉じる",
        edit: "編集",
        undo: "取り消す",
        redo: "やり直す",
        cut: "カット",
        copy: "コピー",
        paste: "ペースト",
        go: "移動",
        back: "戻る",
        home: "ホーム",
        documents: "書類",
        downloads: "ダウンロード",
        pictures: "ピクチャ",
        window: "ウインドウ",
        minimize: "しまう",
        maximize: "拡大",
        close: "閉じる",
        help: "ヘルプ",
        confirmRestartTitle: "今すぐコンピュータを再起動してもよろしいですか？",
        confirmShutdownTitle: "今すぐコンピュータをシステム終了してもよろしいですか？",
        confirmRestartDesc: "何もしない場合、コンピュータは %d 秒後に自動的に再起動します。",
        confirmShutdownDesc: "何もしない場合、コンピュータは %d 秒後に自動的にシステム終了します。",
        reopenWindows: "再ログイン時にウインドウを再度開く",
        cancel: "キャンセル",
        restartBtn: "再起動",
        shutDownBtn: "システム終了",
        deviceName: "デバイス名:",
        processor: "プロセッサ:",
        memory: "メモリ:",
        graphics: "グラフィックス:",
        storage: "ストレージ:",
        copyInfo: "情報をコピー",
        version: "バージョン",
    },
    ru: {
        aboutPulsar: "Об этом компьютере",
        systemSettings: "Системные настройки...",
        appStore: "App Store...",
        lockScreen: "Заблокировать экран",
        logOut: "Завершить сеанс...",
        sleep: "Режим сна",
        restart: "Перезагрузить...",
        shutDown: "Выключить...",
        aboutApp: "О программе %s",
        hideApp: "Скрыть %s",
        hideOthers: "Скрыть остальные",
        showAll: "Показать все",
        quitApp: "Завершить %s",
        file: "Файл",
        newWindow: "Новое окно",
        closeWindow: "Закрыть окно",
        edit: "Правка",
        undo: "Отменить",
        redo: "Повторить",
        cut: "Вырезать",
        copy: "Скопировать",
        paste: "Вставить",
        go: "Переход",
        back: "Назад",
        home: "Личная папка",
        documents: "Документы",
        downloads: "Загрузки",
        pictures: "Изображения",
        window: "Окно",
        minimize: "Свернуть",
        maximize: "Развернуть",
        close: "Закрыть",
        help: "Справка",
        confirmRestartTitle: "Вы действительно хотите перезагрузить компьютер?",
        confirmShutdownTitle: "Вы действительно хотите выключить компьютер?",
        confirmRestartDesc: "Если не выполнить никаких действий, компьютер перезагрузится через %d сек.",
        confirmShutdownDesc: "Если не выполнить никаких действий, компьютер выключится через %d сек.",
        reopenWindows: "Снова открывать окна при повторном входе в систему",
        cancel: "Отменить",
        restartBtn: "Перезагрузить",
        shutDownBtn: "Выключить",
        deviceName: "Имя устройства:",
        processor: "Процессор:",
        memory: "Память:",
        graphics: "Графика:",
        storage: "Хранилище:",
        copyInfo: "Скопировать сведения",
        version: "Версия",
    },
    ar: {
        aboutPulsar: "حول Pulsar OS",
        systemSettings: "إعدادات النظام...",
        appStore: "App Store...",
        lockScreen: "قفل الشاشة",
        logOut: "تسجيل الخروج...",
        sleep: "إسبات",
        restart: "إعادة التشغيل...",
        shutDown: "إيقاف التشغيل...",
        aboutApp: "حول %s",
        hideApp: "إخفاء %s",
        hideOthers: "إخفاء البقية",
        showAll: "إظهار الكل",
        quitApp: "إنهاء %s",
        file: "ملف",
        newWindow: "نافذة جديدة",
        closeWindow: "إغلاق النافذة",
        edit: "تعديل",
        undo: "تراجع",
        redo: "إعادة",
        cut: "قص",
        copy: "نسخ",
        paste: "لصق",
        go: "انتقال",
        back: "رجوع",
        home: "الصفحة الرئيسية",
        documents: "المستندات",
        downloads: "التنزيلات",
        pictures: "الصور",
        window: "نافذة",
        minimize: "تصغير",
        maximize: "تكبير",
        close: "إغلاق",
        help: "مساعدة",
        confirmRestartTitle: "هل أنت متأكد من أنك تريد إعادة تشغيل الكمبيوتر الآن؟",
        confirmShutdownTitle: "هل أنت متأكد من أنك تريد إيقاف تشغيل الكمبيوتر الآن؟",
        confirmRestartDesc: "إذا لم تقم بأي إجراء، فستتم إعادة تشغيل الكمبيوتر تلقائيًا خلال %d ثانية.",
        confirmShutdownDesc: "إذا لم تقم بأي إجراء، فسيتم إيقاف تشغيل الكمبيوتر تلقائيًا خلال %d ثانية.",
        reopenWindows: "إعادة فتح النوافذ عند تسجيل الدخول مرة أخرى",
        cancel: "إلغاء",
        restartBtn: "إعادة التشغيل",
        shutDownBtn: "إيقاف التشغيل",
        deviceName: "اسم الجهاز:",
        processor: "المعالج:",
        memory: "الذاكرة:",
        graphics: "الرسومات:",
        storage: "مساحة التخزين:",
        copyInfo: "نسخ المعلومات",
        version: "الإصدار",
    }
};

function _t(key, ...args) {
    let lang = "en";
    try {
        let langs = GLib.get_language_names();
        for (let l of langs) {
            let prefix = l.split(/[@._-]/)[0].toLowerCase();
            if (I18N[prefix] && prefix !== "c" && prefix !== "posix") {
                lang = prefix;
                break;
            }
        }
        if (lang === "en") {
            let userConfig = GLib.build_filenamev([GLib.get_user_config_dir(), 'locale.conf']);
            let paths = [userConfig, '/etc/locale.conf', '/etc/default/locale'];
            for (let fpath of paths) {
                let file = Gio.File.new_for_path(fpath);
                if (file.query_exists(null)) {
                    let [ok, bytes] = file.load_contents(null);
                    if (ok) {
                        let text = new TextDecoder().decode(bytes);
                        let match = text.match(/(?:LANG|LANGUAGE)=([a-zA-Z0-9_]+)/);
                        if (match) {
                            let cand = match[1].split(/[@._-]/)[0].toLowerCase();
                            if (I18N[cand]) {
                                lang = cand;
                                break;
                            }
                        }
                    }
                }
            }
        }
    } catch (e) {
        lang = "en";
    }
    let dict = I18N[lang] || I18N.en;
    let text = dict[key] || I18N.en[key] || key;
    if (args.length > 0) {
        let idx = 0;
        text = text.replace(/%[sd]/g, () => args[idx++] !== undefined ? args[idx - 1] : '');
    }
    return text;
}


const AboutDialog = GObject.registerClass({
    GTypeName: 'PulsarosAboutDialog'
}, class AboutDialog extends ModalDialog.ModalDialog {
    _init(osName, osVersion, hostName, cpuModel, memTotal, gpuModel, diskInfo) {
        super._init({ styleClass: 'pulsaros-about-dialog' });

        let mainBox = new St.BoxLayout({
            vertical: true,
            style_class: 'pulsaros-about-mainbox'
        });
        this.contentLayout.add_child(mainBox);

        // Add a nice Logo at the top
        let logoTexture = new St.Icon({
            icon_name: 'pulsar-logo',
            icon_size: 96,
            style_class: 'pulsaros-about-logo',
            x_align: Clutter.ActorAlign.CENTER
        });
        
        let logoBox = new St.BoxLayout({
            style_class: 'pulsaros-about-logobox',
            x_align: Clutter.ActorAlign.CENTER
        });
        logoBox.add_child(logoTexture);
        mainBox.add_child(logoBox);

        // Title
        let titleLabel = new St.Label({
            text: osName || "Pulsar OS",
            style_class: 'pulsaros-about-title',
            x_align: Clutter.ActorAlign.CENTER
        });
        titleLabel.clutter_text.selectable = true;
        mainBox.add_child(titleLabel);

        // Version Info
        let verLabel = new St.Label({
            text: `${_t('version')} ${osVersion}`,
            style_class: 'pulsaros-about-subtitle',
            x_align: Clutter.ActorAlign.CENTER
        });
        verLabel.clutter_text.selectable = true;
        mainBox.add_child(verLabel);

        // Details Grid/Table
        let detailsBox = new St.BoxLayout({
            vertical: true,
            style_class: 'pulsaros-about-details'
        });
        mainBox.add_child(detailsBox);

        let addDetail = (label, value) => {
            let row = new St.BoxLayout({
                vertical: false,
                style_class: 'pulsaros-about-row'
            });
            let lbl = new St.Label({
                text: label,
                style_class: 'pulsaros-about-row-label',
                width: 140
            });
            let val = new St.Label({
                text: value,
                style_class: 'pulsaros-about-row-value'
            });
            val.clutter_text.selectable = true;
            row.add_child(lbl);
            row.add_child(val);
            detailsBox.add_child(row);
        };

        addDetail(_t('deviceName'), hostName);
        addDetail(_t('processor'), cpuModel);
        addDetail(_t('memory'), memTotal);
        addDetail(_t('graphics'), gpuModel);
        addDetail(_t('storage'), diskInfo);

        // View Logs Button
        this.addButton({
            label: _t('viewLogs') || "Ver registros",
            action: () => {
                try {
                    let logsDir = "/var/log/pulsaros";
                    let file = Gio.File.new_for_path(logsDir);
                    if (!file.query_exists(null)) {
                        GLib.mkdir_with_parents(logsDir, 0o755);
                    }
                    let context = global.create_app_launch_context(0, -1);
                    Gio.AppInfo.launch_default_for_uri(`file://${logsDir}`, context);
                } catch (e) {
                    logError(e);
                }
            }
        });

        // Copy Info Button
        this.addButton({
            label: _t('copyInfo'),
            action: () => {
                let clipboardText = `${osName || "Pulsar OS"}\n` +
                                    `Version: ${osVersion}\n` +
                                    `Device Name: ${hostName}\n` +
                                    `Processor: ${cpuModel}\n` +
                                    `Memory: ${memTotal}\n` +
                                    `Graphics: ${gpuModel}\n` +
                                    `Storage: ${diskInfo}`;
                St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, clipboardText);
            }
        });

        // Close Button
        this.addButton({
            label: _t('close') || "Close",
            action: () => {
                this.close();
            },
            key: Clutter.KEY_Escape
        });
    }
});

const _isSpanish = () => {
    let langs = GLib.get_language_names ? GLib.get_language_names() : [];
    return langs.length > 0 && langs[0].startsWith('es');
};

const PowerConfirmDialog = GObject.registerClass({
    GTypeName: 'PulsarosPowerConfirmDialog'
}, class PowerConfirmDialog extends ModalDialog.ModalDialog {
    _init(actionType, callback) {
        super._init({ styleClass: 'pulsaros-power-dialog' });

        this._actionType = actionType; // 'shutdown' | 'restart'
        this._callback = callback;
        this._restoreSession = false;
        this._countdown = 60;
        this._timerId = 0;
        this._progressTimerId = 0;

        let mainBox = new St.BoxLayout({
            vertical: true,
            style_class: 'pulsaros-power-mainbox'
        });
        this.contentLayout.add_child(mainBox);
        this._mainBox = mainBox;

        // Circular Icon Badge matching macOS Tahoe (properly centered)
        let circleBox = new St.Bin({
            style_class: 'pulsaros-power-circle-badge',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
            x_expand: false,
            y_expand: false
        });
        let iconName = actionType === 'restart' ? 'system-restart-symbolic' : 'system-shutdown-symbolic';
        this._icon = new St.Icon({
            icon_name: iconName,
            icon_size: 32,
            style_class: 'pulsaros-power-circle-icon'
        });
        circleBox.set_child(this._icon);

        let iconContainer = new St.BoxLayout({
            vertical: false,
            x_align: Clutter.ActorAlign.START,
            style_class: 'pulsaros-power-icon-container'
        });
        iconContainer.add_child(circleBox);
        mainBox.add_child(iconContainer);

        // Title (Left aligned, wrapping, full text)
        let isRestart = actionType === 'restart';
        let titleText = isRestart
            ? _t('confirmRestartTitle')
            : _t('confirmShutdownTitle');
        this._titleLabel = new St.Label({
            text: titleText,
            style_class: 'pulsaros-power-title',
            x_align: Clutter.ActorAlign.START
        });
        this._titleLabel.clutter_text.line_wrap = true;
        this._titleLabel.clutter_text.line_wrap_mode = Pango.WrapMode.WORD;
        this._titleLabel.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
        mainBox.add_child(this._titleLabel);

        // Subtitle / Countdown prompt (Left aligned, wrapping)
        let getDescText = (secs) => isRestart
            ? _t('confirmRestartDesc', secs)
            : _t('confirmShutdownDesc', secs);

        this._descLabel = new St.Label({
            text: getDescText(this._countdown),
            style_class: 'pulsaros-power-subtitle',
            x_align: Clutter.ActorAlign.START
        });
        this._descLabel.clutter_text.line_wrap = true;
        this._descLabel.clutter_text.line_wrap_mode = Pango.WrapMode.WORD;
        this._descLabel.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
        mainBox.add_child(this._descLabel);

        // Option Container (Reopen windows checkbox, left-aligned)
        this._optionBox = new St.BoxLayout({
            vertical: true,
            style_class: 'pulsaros-power-option-box',
            x_align: Clutter.ActorAlign.START
        });
        mainBox.add_child(this._optionBox);

        // Checkbox Toggle Row
        let checkRow = new St.Button({
            style_class: 'pulsaros-power-checkbox-button',
            reactive: true,
            can_focus: true,
            toggle_mode: true,
            checked: false,
            x_align: Clutter.ActorAlign.START
        });

        let checkLayout = new St.BoxLayout({
            vertical: false,
            style_class: 'pulsaros-power-checkbox-layout',
            y_align: Clutter.ActorAlign.CENTER
        });

        let checkIcon = new St.Icon({
            icon_name: 'checkbox-symbolic',
            icon_size: 18,
            style_class: 'pulsaros-power-checkbox-icon'
        });
        checkLayout.add_child(checkIcon);

        let checkLabel = new St.Label({
            text: _t('reopenWindows'),
            style_class: 'pulsaros-power-checkbox-label',
            y_align: Clutter.ActorAlign.CENTER
        });
        checkLayout.add_child(checkLabel);

        checkRow.set_child(checkLayout);

        let syncCheck = (val) => {
            this._restoreSession = val;
            checkRow.checked = val;
            checkIcon.icon_name = val ? 'checkbox-checked-symbolic' : 'checkbox-symbolic';
            if (val) {
                checkIcon.add_style_pseudo_class('checked');
            } else {
                checkIcon.remove_style_pseudo_class('checked');
            }
        };

        checkRow.connect('clicked', () => {
            syncCheck(!this._restoreSession);
        });

        this._optionBox.add_child(checkRow);

        // Cancel Button
        this._cancelBtn = this.addButton({
            label: _t('cancel'),
            action: () => {
                this._cleanup();
                this.close();
            },
            key: Clutter.KEY_Escape
        });
        if (this._cancelBtn && this._cancelBtn.add_style_class_name) {
            this._cancelBtn.add_style_class_name('pulsaros-power-cancel-btn');
        }

        // Action Button (Restart / Shut Down)
        let actionLabel = isRestart ? _t('restartBtn') : _t('shutDownBtn');
        this._actionBtn = this.addButton({
            label: actionLabel,
            action: () => {
                let restore = this._restoreSession;
                if (restore) {
                    this._showProgressState();
                } else {
                    this._cleanup();
                    this.close();
                    if (this._callback) {
                        this._callback(false);
                    }
                }
            },
            default: true
        });
        if (this._actionBtn && this._actionBtn.add_style_class_name) {
            this._actionBtn.add_style_class_name('pulsaros-power-confirm-btn');
        }

        // 60-Second live countdown timer
        this._timerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 1, () => {
            this._countdown--;
            if (this._countdown <= 0) {
                this._timerId = 0;
                let restore = this._restoreSession;
                if (restore) {
                    this._showProgressState();
                } else {
                    this._cleanup();
                    this.close();
                    if (this._callback) {
                        this._callback(false);
                    }
                }
                return GLib.SOURCE_REMOVE;
            }
            this._descLabel.text = getDescText(this._countdown);
            return GLib.SOURCE_CONTINUE;
        });
    }

    _cleanup() {
        if (this._timerId > 0) {
            GLib.source_remove(this._timerId);
            this._timerId = 0;
        }
        if (this._progressTimerId > 0) {
            GLib.source_remove(this._progressTimerId);
            this._progressTimerId = 0;
        }
    }

    _showProgressState() {
        this._cleanup();
        // Hide interactive elements
        this._optionBox.hide();
        if (this._cancelBtn) this._cancelBtn.hide();
        if (this._actionBtn) this._actionBtn.hide();

        let isRestart = this._actionType === 'restart';
        let actionName = isRestart ? 'Restarting' : 'Shutting down';
        let targetAction = isRestart ? 'restart' : 'power off';

        this._titleLabel.text = `${actionName}…`;
        this._descLabel.text = "Saving session state to disk…";

        // Progress Container
        let progressBox = new St.BoxLayout({
            vertical: true,
            style_class: 'pulsaros-power-progress-container',
            x_align: Clutter.ActorAlign.CENTER
        });
        this._mainBox.add_child(progressBox);

        // Progress Bar Background
        let progressBg = new St.BoxLayout({
            style_class: 'pulsaros-power-progress-bg',
            x_align: Clutter.ActorAlign.START
        });

        let progressFill = new St.Widget({
            style_class: 'pulsaros-power-progress-fill',
            width: 20
        });
        progressBg.add_child(progressFill);
        progressBox.add_child(progressBg);

        // Progress Status Text
        let statusLabel = new St.Label({
            text: "Preparing memory and application snapshot…",
            style_class: 'pulsaros-power-progress-status',
            x_align: Clutter.ActorAlign.CENTER
        });
        progressBox.add_child(statusLabel);

        let noteLabel = new St.Label({
            text: `The computer will ${targetAction} automatically. Your session will be restored on next boot.`,
            style_class: 'pulsaros-power-progress-note',
            x_align: Clutter.ActorAlign.CENTER
        });
        progressBox.add_child(noteLabel);

        // Animate progress bar smoothly
        let startTime = GLib.get_monotonic_time();
        let totalBgWidth = 380;
        let powerTriggered = false;

        this._progressTimerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 80, () => {
            let elapsedSec = (GLib.get_monotonic_time() - startTime) / 1000000;
            
            // Smooth asymptotic progress animation
            let progress = Math.min(0.95, 1 - Math.exp(-elapsedSec / 2.5));
            let currentWidth = Math.max(20, Math.floor(totalBgWidth * progress));
            progressFill.set_width(currentWidth);

            if (elapsedSec > 0.8 && elapsedSec < 2.0) {
                statusLabel.text = "Syncing system state and VRAM to disk…";
            } else if (elapsedSec >= 2.0) {
                statusLabel.text = "Writing memory snapshot to SSD…";
            }

            // Trigger the power action and close modal immediately so snapshot is clean
            if (elapsedSec >= 0.4 && !powerTriggered) {
                powerTriggered = true;
                if (this._callback) {
                    this._callback(true);
                }
                this._cleanup();
                this.close();
            }

            return GLib.SOURCE_CONTINUE;
        });
    }

    destroy() {
        this._cleanup();
        super.destroy();
    }
});

export const ThemeLogoutPromptDialog = GObject.registerClass({
    GTypeName: 'PulsarosThemeLogoutPromptDialog'
}, class ThemeLogoutPromptDialog extends ModalDialog.ModalDialog {
    _init() {
        super._init({ styleClass: 'pulsaros-power-dialog' });

        this._countdown = 6;
        this._timerId = 0;

        let mainBox = new St.BoxLayout({
            vertical: true,
            style_class: 'pulsaros-power-mainbox'
        });
        this.contentLayout.add_child(mainBox);

        // Icon Header
        let iconContainer = new St.BoxLayout({
            style_class: 'pulsaros-power-icon-container',
            x_align: Clutter.ActorAlign.CENTER
        });
        let circleBadge = new St.Bin({
            style_class: 'pulsaros-power-circle-badge',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
            x_expand: false,
            y_expand: false
        });
        let icon = new St.Icon({
            icon_name: 'preferences-desktop-theme-symbolic',
            icon_size: 32,
            style_class: 'pulsaros-power-circle-icon'
        });
        circleBadge.set_child(icon);
        iconContainer.add_child(circleBadge);
        mainBox.add_child(iconContainer);

        // Title
        let titleLabel = new St.Label({
            text: _t('themeLogoutTitle') || "Cambio de aspecto aplicado",
            style_class: 'pulsaros-theme-prompt-title',
            x_align: Clutter.ActorAlign.CENTER
        });
        mainBox.add_child(titleLabel);

        // Description
        let getDescText = (sec) => {
            let t = _t('themeLogoutDesc', sec);
            return t || `Se cerrará la sesión en ${sec} segundos para aplicar el nuevo tema por completo.`;
        };

        this._descLabel = new St.Label({
            text: getDescText(this._countdown),
            style_class: 'pulsaros-theme-prompt-subtitle',
            x_align: Clutter.ActorAlign.CENTER
        });
        mainBox.add_child(this._descLabel);

        // Cancel Button
        this._cancelBtn = this.addButton({
            label: _t('cancel') || "Cancelar",
            action: () => {
                this._cleanup();
                this.close();
            },
            key: Clutter.KEY_Escape
        });
        if (this._cancelBtn && this._cancelBtn.add_style_class_name) {
            this._cancelBtn.add_style_class_name('pulsaros-power-cancel-btn');
        }

        // Action Button
        this._actionBtn = this.addButton({
            label: _t('logoutNowBtn') || "Cerrar sesión ahora",
            action: () => {
                this._cleanup();
                this.close();
                this._executeLogout();
            },
            default: true
        });
        if (this._actionBtn && this._actionBtn.add_style_class_name) {
            this._actionBtn.add_style_class_name('pulsaros-power-confirm-btn');
        }

        this._timerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 1, () => {
            this._countdown--;
            if (this._countdown <= 0) {
                this._timerId = 0;
                this._cleanup();
                this.close();
                this._executeLogout();
                return GLib.SOURCE_REMOVE;
            }
            this._descLabel.text = getDescText(this._countdown);
            return GLib.SOURCE_CONTINUE;
        });
    }

    _executeLogout() {
        try {
            GLib.spawn_command_line_async("gnome-session-quit --logout --no-prompt");
        } catch (e) {
            console.error("[GlobalMenu] Logout error:", e);
        }
    }

    _cleanup() {
        if (this._timerId > 0) {
            GLib.source_remove(this._timerId);
            this._timerId = 0;
        }
    }

    destroy() {
        this._cleanup();
        super.destroy();
    }
});

export const PulsarLogoButton = GObject.registerClass({
    GTypeName: 'PulsarLogoButton',
}, class PulsarLogoButton extends PanelMenu.Button {
    _init(extension) {
        super._init(0.0, 'Pulsar Apple Menu', false);
        this._extension = extension;
        this._pulsarAppleLogo = true;

        let iconFile = Gio.File.new_for_path(extension.dir.get_path() + '/pulsar-white-sf.png');
        let fileIcon = new Gio.FileIcon({ file: iconFile });
        this.icon = new St.Icon({
            gicon: fileIcon,
            style_class: 'global-menu-logo-icon'
        });
        this.add_child(this.icon);

        this._globalMenuButton = true;
        this.menu.actor.add_style_class_name('global-menu-popup');

        this._buildMenu();

        this.menu.connect('open-state-changed', (menu, open) => {
            if (open) {
                Appearance.onMenuOpened(this);
            }
        });
        Appearance.registerButton(this);
    }

    _openUri(uri) {
        try {
            Gio.AppInfo.launch_default_for_uri(uri, null);
        } catch (e) {
            console.error('[GlobalMenu] Failed to open URI ' + uri, e);
        }
    }

    _buildMenu() {
        let aboutItem = new PopupMenu.PopupMenuItem(_t('aboutPulsar'));
        aboutItem.connect('activate', () => {
            let hostName = GLib.get_host_name();
            let memTotal = 'N/A';
            try {
                let [ok, content] = GLib.file_get_contents('/proc/meminfo');
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let match = contentStr.match(/MemTotal:\s+(\d+)\s+kB/);
                    if (match) {
                        let gb = (parseInt(match[1]) / 1024 / 1024).toFixed(1);
                        memTotal = gb + ' GB';
                    }
                }
            } catch (e) {}

            let cpuModel = 'Unknown CPU';
            try {
                let [ok, content] = GLib.file_get_contents('/proc/cpuinfo');
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let match = contentStr.match(/model name\s+:\s+(.+)/);
                    if (match) cpuModel = match[1].trim();
                }
            } catch (e) {}

            let gpuModel = 'Unknown GPU';
            try {
                let [success, stdout] = GLib.spawn_command_line_sync('lspci');
                if (success) {
                    let stdoutStr = new TextDecoder().decode(stdout);
                    for (let line of stdoutStr.split('\n')) {
                        if (line.match(/VGA compatible controller|3D controller|Display controller/i)) {
                            let parts = line.split(': ');
                            if (parts.length > 1) gpuModel = parts[1].trim();
                            break;
                        }
                    }
                }
            } catch (e) {}

            let diskInfo = 'N/A';
            try {
                let [success, stdout] = GLib.spawn_command_line_sync('df -h /');
                if (success) {
                    let stdoutStr = new TextDecoder().decode(stdout);
                    let dlines = stdoutStr.split('\n');
                    if (dlines.length > 1) {
                        let parts = dlines[1].split(/\s+/);
                        if (parts.length > 4) diskInfo = parts[1] + ' (' + parts[3] + ' available)';
                    }
                }
            } catch (e) {}

            let osName = 'Pulsar OS Bitten Fruit';
            let osVersion = '1.0';
            try {
                let [ok, content] = GLib.file_get_contents('/etc/os-release');
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let prettyNameMatch = contentStr.match(/^PRETTY_NAME="?([^"\n]+)"?/m);
                    let verMatch = contentStr.match(/^VERSION="?([^"\n]+)"?/m);
                    if (prettyNameMatch) osName = prettyNameMatch[1];
                    if (verMatch) osVersion = verMatch[1];
                }
            } catch (e) {}

            let dialog = new AboutDialog(osName, osVersion, hostName, cpuModel, memTotal, gpuModel, diskInfo);
            dialog.open();
        });
        this.menu.addMenuItem(aboutItem);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        let settingsItem = new PopupMenu.PopupMenuItem(_t('systemSettings'));
        settingsItem.connect('activate', () => {
            try {
                let app = Gio.AppInfo.create_from_commandline('gnome-control-center', 'GNOME Settings', Gio.AppInfoCreateFlags.NONE);
                app.launch([], null);
            } catch (e) {
                console.error('[GlobalMenu] Failed to open GNOME Settings:', e);
            }
        });
        this.menu.addMenuItem(settingsItem);

        let appStoreItem = new PopupMenu.PopupMenuItem(_t('appStore'));
        appStoreItem.connect('activate', () => {
            try {
                let app = Gio.AppInfo.create_from_commandline('es.inled.AppInstall', 'App Store', Gio.AppInfoCreateFlags.NONE);
                app.launch([], null);
            } catch (e) {
                this._openUri('appstream://org.gnome.Software');
            }
        });
        this.menu.addMenuItem(appStoreItem);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        let lockItem = new PopupMenu.PopupMenuItem(_t('lockScreen'));
        lockItem.connect('activate', () => {
            if (this._extension._lockScreenOverlay) {
                this._extension._lockScreenOverlay.lock();
            } else {
                try {
                    let sm = new Gio.Settings({ schema_id: 'org.gnome.desktop.lockdown' });
                    Main.screenShield.lock(true);
                } catch (e) {}
            }
        });
        this.menu.addMenuItem(lockItem);

        let logoutItem = new PopupMenu.PopupMenuItem(_t('logOut'));
        logoutItem.connect('activate', () => {
            try {
                let session = new Gio.DBusProxy({
                    g_connection: Gio.DBus.session,
                    g_name: 'org.gnome.SessionManager',
                    g_object_path: '/org/gnome/SessionManager',
                    g_interface_name: 'org.gnome.SessionManager'
                });
                session.init(null);
                session.call_sync('Logout', GLib.Variant.new('(u)', [0]), Gio.DBusCallFlags.NONE, -1, null);
            } catch (e) {
                console.error('[GlobalMenu] Logout DBus error:', e);
            }
        });
        this.menu.addMenuItem(logoutItem);

        let sleepItem = new PopupMenu.PopupMenuItem(_t('sleep'));
        sleepItem.connect('activate', () => {
            try {
                let login1 = new Gio.DBusProxy({
                    g_connection: Gio.DBus.system,
                    g_name: 'org.freedesktop.login1',
                    g_object_path: '/org/freedesktop/login1',
                    g_interface_name: 'org.freedesktop.login1.Manager'
                });
                login1.init(null);
                login1.call_sync('Suspend', GLib.Variant.new('(b)', [true]), Gio.DBusCallFlags.NONE, -1, null);
            } catch (e) {
                console.error('[GlobalMenu] Suspend DBus error:', e);
            }
        });
        this.menu.addMenuItem(sleepItem);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        let restartItem = new PopupMenu.PopupMenuItem(_t('restart'));
        restartItem.connect('activate', () => {
            let dialog = new PowerConfirmDialog('restart', () => {
                try {
                    let login1 = new Gio.DBusProxy({
                        g_connection: Gio.DBus.system,
                        g_name: 'org.freedesktop.login1',
                        g_object_path: '/org/freedesktop/login1',
                        g_interface_name: 'org.freedesktop.login1.Manager'
                    });
                    login1.init(null);
                    login1.call_sync('Reboot', GLib.Variant.new('(b)', [true]), Gio.DBusCallFlags.NONE, -1, null);
                } catch (e) {}
            });
            this._extension._activePowerDialog = dialog;
            dialog.open();
        });
        this.menu.addMenuItem(restartItem);

        let shutdownItem = new PopupMenu.PopupMenuItem(_t('shutDown'));
        shutdownItem.connect('activate', () => {
            this.triggerPowerAction('shutdown');
        });
        this.menu.addMenuItem(shutdownItem);

        global._pulsarTriggerPowerAction = (action) => this.triggerPowerAction(action);
    }

    triggerPowerAction(action) {
        switch (action) {
            case 'lock-screen':
            case 'lock':
                if (this._extension._lockScreenOverlay) {
                    this._extension._lockScreenOverlay.lock();
                } else {
                    try { Main.screenShield.lock(true); } catch (_) {}
                }
                break;
            case 'logout':
                try {
                    let session = new Gio.DBusProxy({
                        g_connection: Gio.DBus.session,
                        g_name: 'org.gnome.SessionManager',
                        g_object_path: '/org/gnome/SessionManager',
                        g_interface_name: 'org.gnome.SessionManager'
                    });
                    session.init(null);
                    session.call_sync('Logout', GLib.Variant.new('(u)', [0]), Gio.DBusCallFlags.NONE, -1, null);
                } catch (e) {
                    GLib.spawn_command_line_async("gnome-session-quit --logout");
                }
                break;
            case 'suspend':
            case 'sleep':
                try {
                    let login1 = new Gio.DBusProxy({
                        g_connection: Gio.DBus.system,
                        g_name: 'org.freedesktop.login1',
                        g_object_path: '/org/freedesktop/login1',
                        g_interface_name: 'org.freedesktop.login1.Manager'
                    });
                    login1.init(null);
                    login1.call_sync('Suspend', GLib.Variant.new('(b)', [true]), Gio.DBusCallFlags.NONE, -1, null);
                } catch (e) {}
                break;
            case 'restart':
            case 'reboot': {
                let dialog = new PowerConfirmDialog('restart', () => {
                    try {
                        let login1 = new Gio.DBusProxy({
                            g_connection: Gio.DBus.system,
                            g_name: 'org.freedesktop.login1',
                            g_object_path: '/org/freedesktop/login1',
                            g_interface_name: 'org.freedesktop.login1.Manager'
                        });
                        login1.init(null);
                        login1.call_sync('Reboot', GLib.Variant.new('(b)', [true]), Gio.DBusCallFlags.NONE, -1, null);
                    } catch (e) {}
                });
                this._extension._activePowerDialog = dialog;
                dialog.open();
                break;
            }
            case 'power-off':
            case 'poweroff':
            case 'shutdown': {
                let dialog = new PowerConfirmDialog('shutdown', () => {
                    try {
                        let login1 = new Gio.DBusProxy({
                            g_connection: Gio.DBus.system,
                            g_name: 'org.freedesktop.login1',
                            g_object_path: '/org/freedesktop/login1',
                            g_interface_name: 'org.freedesktop.login1.Manager'
                        });
                        login1.init(null);
                        login1.call_sync('PowerOff', GLib.Variant.new('(b)', [true]), Gio.DBusCallFlags.NONE, -1, null);
                    } catch (e) {}
                });
                this._extension._activePowerDialog = dialog;
                dialog.open();
                break;
            }
        }
    }

    destroy() {
        if (global._pulsarTriggerPowerAction) {
            global._pulsarTriggerPowerAction = null;
        }
        super.destroy();
    }
});
