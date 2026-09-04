/**
 * Pulsar OS - Global Menu Extension
 * Extension to display macOS-like application menus in GNOME top bar.
 * 
 * Extensión de Pulsar OS - Menú Global
 * Extensión para mostrar menús de aplicación al estilo macOS en la barra superior de GNOME.
 * 
 * Compatible with GNOME 45-50 & Wayland.
 * Compatible con GNOME 45-50 y Wayland.
 */

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as ModalDialog from 'resource:///org/gnome/shell/ui/modalDialog.js';
import { getPointerWatcher } from 'resource:///org/gnome/shell/ui/pointerWatcher.js';
import * as SystemActions from 'resource:///org/gnome/shell/misc/systemActions.js';

import St from 'gi://St';
import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Shell from 'gi://Shell';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';
import Pango from 'gi://Pango';
import Gst from 'gi://Gst';
import GstApp from 'gi://GstApp';

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
        version: "Version",
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
        version: "Versión",
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
            if (I18N[prefix]) {
                lang = prefix;
                break;
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

// Custom GObject class to construct the menu buttons in the panel
// Clase GObject personalizada para construir los botones de menú en el panel
const GlobalMenuButton = GObject.registerClass({
    GTypeName: 'PulsarosGlobalMenuButton'
}, class GlobalMenuButton extends PanelMenu.Button {
    _init(title, isAppMenu = false) {
        super._init(0.0, title, false);
        
        // St.Label widget to show the menu text on the top bar
        // Widget St.Label para mostrar el texto del menú en la barra superior
        this.label = new St.Label({
            text: title,
            y_align: Clutter.ActorAlign.CENTER,
            style_class: isAppMenu ? 'global-menu-app-label' : 'global-menu-label'
        });
        
        this.add_child(this.label);
    }
    
    // Updates the label text dynamically (e.g., when active window changes)
    // Actualiza el texto de la etiqueta dinámicamente (p. ej., al cambiar la ventana activa)
    setText(text) {
        this.label.set_text(text);
    }
});

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
            label: "Close",
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

const LockScreen = GObject.registerClass({
    GTypeName: 'PulsarosLockScreen'
}, class LockScreen extends St.Widget {
    _init(extension) {
        super._init({
            name: 'pulsaros-lockscreen',
            visible: false,
            opacity: 0,
            reactive: false,
            x_expand: true,
            y_expand: true
        });
        
        this._extension = extension;
        this._isLocked = false;
        this._hasGrab = false;
        this._authenticating = false;
        this._timerId = 0;
        this._sizeChangedId = 0;
        this._sizeChangedId2 = 0;
        this._monitorContainers = [];
        this._clocks = [];
        this._passwordEntry = null;
        this._lockIdleTimerId = 0;
        this._lockIdleTimeoutSeconds = 60;
        this._stageEventId = 0;

        this._bgSettings = new Gio.Settings({ schema_id: 'org.gnome.desktop.background' });
        this._ifaceSettings = new Gio.Settings({ schema_id: 'org.gnome.desktop.interface' });
        
        this._bgChangedId1 = this._bgSettings.connect('changed::picture-uri', () => this._updateWallpapers());
        this._bgChangedId2 = this._bgSettings.connect('changed::picture-uri-dark', () => this._updateWallpapers());
        this._bgChangedId3 = this._ifaceSettings.connect('changed::color-scheme', () => this._updateWallpapers());

        // Monitor screen and layout changes to remain fullscreen and handle multi-monitor layouts
        this._sizeChangedId = global.stage.connect('notify::width', () => this._onSizeChanged());
        this._sizeChangedId2 = global.stage.connect('notify::height', () => this._onSizeChanged());
        this._monitorsChangedId = Main.layoutManager.connect('monitors-changed', () => this._onSizeChanged());
        
        this._onSizeChanged();
    }
    
    _getWallpaperUrl() {
        try {
            let homeDir = GLib.get_home_dir();
            
            // 1. Check Pulsar OS Native Live Wallpaper configuration first
            let pulsarLiveCfg = GLib.build_filenamev([homeDir, '.config', 'pulsaros', 'live-wallpaper.json']);
            let pulsarFile = Gio.File.new_for_path(pulsarLiveCfg);
            if (pulsarFile.query_exists(null)) {
                let [ok, contents] = pulsarFile.load_contents(null);
                if (ok) {
                    let json = JSON.parse(new TextDecoder().decode(contents));
                    if (json && json.enabled && json.file) {
                        let rawPath = json.file.startsWith('file://') ? decodeURIComponent(json.file.substring(7)) : json.file;
                        let f = Gio.File.new_for_path(rawPath);
                        if (f.query_exists(null)) {
                            return f.get_uri();
                        }
                    }
                }
            }

            // 2. Check Hidamari active animated/video wallpaper configuration if present
            let hidamariPaths = [
                GLib.build_filenamev([homeDir, '.var', 'app', 'io.github.jeffshee.Hidamari', 'config', 'hidamari', 'hidamari.json']),
                GLib.build_filenamev([homeDir, '.config', 'hidamari', 'hidamari.json'])
            ];
            for (let cfgPath of hidamariPaths) {
                let file = Gio.File.new_for_path(cfgPath);
                if (file.query_exists(null)) {
                    let [ok, contents] = file.load_contents(null);
                    if (ok) {
                        let json = JSON.parse(new TextDecoder().decode(contents));
                        if (json && json.file) {
                            let rawPath = json.file.startsWith('file://') ? decodeURIComponent(json.file.substring(7)) : json.file;
                            let f = Gio.File.new_for_path(rawPath);
                            if (f.query_exists(null)) {
                                return f.get_uri();
                            }
                        }
                    }
                }
            }

            // 3. Check system default live video wallpaper
            let sddmVideo = '/var/lib/pulsar-sddm/pulsar-wallpaper.mp4';
            if (GLib.file_test(sddmVideo, GLib.FileTest.EXISTS)) {
                return `file://${sddmVideo}`;
            }

            // 2. Read active GNOME background settings
            let colorScheme = this._ifaceSettings.get_string('color-scheme');
            let uri = (colorScheme === 'prefer-dark')
                ? this._bgSettings.get_string('picture-uri-dark')
                : this._bgSettings.get_string('picture-uri');
            
            if (!uri || uri === 'none') {
                uri = this._bgSettings.get_string('picture-uri');
            }
            if (!uri || uri === 'none') {
                uri = this._bgSettings.get_string('picture-uri-dark');
            }
            if (uri && uri !== 'none') {
                // If it is an XML slideshow file, extract the real image file
                if (uri.endsWith('.xml')) {
                    let path = uri.startsWith('file://') ? uri.substring(7) : uri;
                    let xmlFile = Gio.File.new_for_path(path);
                    if (xmlFile.query_exists(null)) {
                        let [ok, contents] = xmlFile.load_contents(null);
                        if (ok) {
                            let text = new TextDecoder().decode(contents);
                            let match = text.match(/<file>([^<]+)<\/file>/);
                            if (match && match[1] && Gio.File.new_for_path(match[1]).query_exists(null)) {
                                return `file://${match[1]}`;
                            }
                        }
                    }
                }
                return uri;
            }
        } catch (e) {
            console.error("[LockScreen] Error resolving wallpaper:", e);
        }
        return `file://${this._extension.path}/background.webp`;
    }

    _getPosterUrl(videoUrl) {
        let homeDir = GLib.get_home_dir();
        let primaryPoster = GLib.build_filenamev([homeDir, '.local', 'share', 'backgrounds', 'pulsar-live-wallpaper.png']);
        if (GLib.file_test(primaryPoster, GLib.FileTest.EXISTS)) {
            return `file://${primaryPoster}`;
        }
        let sddmPoster = '/var/lib/pulsar-sddm/pulsar-wallpaper.png';
        if (GLib.file_test(sddmPoster, GLib.FileTest.EXISTS)) {
            return `file://${sddmPoster}`;
        }
        return `file:///usr/share/backgrounds/pulsar-os-tahoe.png`;
    }

    _isVideoFile(url) {
        if (!url) return false;
        let clean = url.toLowerCase();
        return clean.endsWith('.mp4') || clean.endsWith('.webm') || clean.endsWith('.mkv') || clean.endsWith('.mov') || clean.endsWith('.avi');
    }

    _startVideoWallpaper(videoPath) {
        this._stopVideoWallpaper();
        try {
            Gst.init(null);
            let localPath = videoPath.startsWith('file://') ? decodeURIComponent(videoPath.substring(7)) : videoPath;
            let file = Gio.File.new_for_path(localPath);
            if (!file.query_exists(null)) {
                return;
            }
            let videoUri = file.get_uri();
            let posterUrl = this._getPosterUrl(videoPath);

            this._videoContent = new Clutter.Image();
            for (let container of this._monitorContainers) {
                if (container._videoActor) {
                    container._videoActor.set_content(this._videoContent);
                    container._videoActor.visible = true;
                }
                container.style = `background-image: none; background-color: #000000;`;
            }

            let videoSinkBin = Gst.parse_bin_from_description(
                'videoconvert ! video/x-raw,format=RGBA ! appsink name=sink emit-signals=false max-buffers=2 drop=true sync=false',
                true
            );
            this._videoPipeline = Gst.ElementFactory.make('playbin', 'lockscreen-player');
            this._videoPipeline.set_property('uri', videoUri);
            this._videoPipeline.set_property('video-sink', videoSinkBin);
            let audioSink = Gst.ElementFactory.make('fakesink', 'lockscreen-audiosink');
            this._videoPipeline.set_property('audio-sink', audioSink);

            this._videoSink = videoSinkBin.get_by_name('sink');

            let bus = this._videoPipeline.get_bus();
            bus.add_signal_watch();
            this._busWatchId = bus.connect('message::eos', () => {
                if (this._videoPipeline) {
                    this._videoPipeline.seek_simple(Gst.Format.TIME, Gst.SeekFlags.FLUSH | Gst.SeekFlags.KEY_UNIT, 0);
                }
            });

            this._videoPipeline.set_state(Gst.State.PLAYING);

            this._videoTimerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 33, () => {
                if (!this._isLocked || !this._videoSink) {
                    return GLib.SOURCE_CONTINUE;
                }
                try {
                    let sample = this._videoSink.try_pull_sample(0);
                    if (sample) {
                        let buffer = sample.get_buffer();
                        let caps = sample.get_caps();
                        let s = caps.get_structure(0);
                        let [okW, width] = s.get_int('width');
                        let [okH, height] = s.get_int('height');
                        let [okMap, mapInfo] = buffer.map(Gst.MapFlags.READ);
                        if (okMap) {
                            let bytes = GLib.Bytes.new(mapInfo.data);
                            buffer.unmap(mapInfo);
                            if (this._videoContent) {
                                this._videoContent.set_bytes(
                                    bytes,
                                    Cogl.PixelFormat.RGBA_8888,
                                    width,
                                    height,
                                    width * 4
                                );
                            }
                        }
                    }
                } catch (pullErr) {}
                return GLib.SOURCE_CONTINUE;
            });
        } catch (e) {
            console.error("[LockScreen] Failed to start GStreamer video wallpaper:", e);
        }
    }

    _stopVideoWallpaper() {
        if (this._videoTimerId) {
            GLib.source_remove(this._videoTimerId);
            this._videoTimerId = 0;
        }
        if (this._busWatchId && this._videoPipeline) {
            let bus = this._videoPipeline.get_bus();
            bus.disconnect(this._busWatchId);
            this._busWatchId = 0;
        }
        if (this._videoPipeline) {
            this._videoPipeline.set_state(Gst.State.NULL);
            this._videoPipeline = null;
        }
        this._videoSink = null;
        this._videoContent = null;
        if (this._monitorContainers) {
            for (let container of this._monitorContainers) {
                if (container._videoActor) {
                    container._videoActor.set_content(null);
                    container._videoActor.visible = false;
                }
            }
        }
    }

    _updateWallpapers() {
        if (!this._monitorContainers || this._monitorContainers.length === 0) {
            return;
        }
        let bgUrl = this._getWallpaperUrl();
        if (this._isVideoFile(bgUrl)) {
            if (this._isLocked) {
                this._startVideoWallpaper(bgUrl);
            }
        } else {
            this._stopVideoWallpaper();
            for (let container of this._monitorContainers) {
                container.style = `background-image: url("${bgUrl}"); background-size: cover; background-position: center;`;
            }
        }
    }
    
    _onSizeChanged() {
        if (!this._isLocked) {
            this.set_position(0, 0);
            this.set_size(0, 0);
            this.visible = false;
            this.opacity = 0;
            this.reactive = false;
            return;
        }
        this.set_position(0, 0);
        this.set_size(global.stage.width, global.stage.height);
        
        this._rebuildMonitors();
    }

    _rebuildMonitors() {
        // Destroy old containers
        if (this._monitorContainers) {
            for (let container of this._monitorContainers) {
                container.destroy();
            }
        }
        this._monitorContainers = [];
        this._clocks = [];
        this._passwordEntry = null;

        this.visible = this._isLocked;
        this.opacity = this._isLocked ? 255 : 0;
        this.reactive = this._isLocked;

        let monitors = Main.layoutManager.monitors;
        let primaryMonitor = Main.layoutManager.primaryMonitor;
        let bgUrl = this._getWallpaperUrl();
        let isVideo = this._isVideoFile(bgUrl);

        for (let i = 0; i < monitors.length; i++) {
            let monitor = monitors[i];
            let isPrimary = (monitor === primaryMonitor);

            let container = new St.Widget({
                name: `pulsaros-lockscreen-monitor-${i}`,
                clip_to_allocation: true,
                reactive: true
            });

            let videoActor = new Clutter.Actor({
                name: `pulsaros-lockscreen-video-${i}`,
                x_expand: true,
                y_expand: true,
                width: monitor.width,
                height: monitor.height,
                visible: false
            });
            container.add_child(videoActor);
            container._videoActor = videoActor;

            if (this._isLocked) {
                if (isVideo) {
                    container.style = `background-image: url("${this._getPosterUrl(bgUrl)}"); background-size: cover; background-position: center;`;
                } else {
                    container.style = `background-image: url("${bgUrl}"); background-size: cover; background-position: center;`;
                }
            } else {
                container.style = 'background-image: none; background-color: transparent;';
            }
            container.set_position(monitor.x, monitor.y);
            container.set_size(monitor.width, monitor.height);

            this.add_child(container);
            this._monitorContainers.push(container);

            this._buildMonitorUI(container, monitor, isPrimary);
        }

        if (this._isLocked && isVideo) {
            this._startVideoWallpaper(bgUrl);
        }

        // Live clock updates
        if (this._isLocked) {
            this._updateClock();
        }

        if (this._isLocked) {
            GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
                if (this._passwordEntry) {
                    let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
                    activeText.grab_key_focus();
                }
                return GLib.SOURCE_REMOVE;
            });
        }
    }

    _buildMonitorUI(container, monitor, isPrimary) {
        let contentLayout = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            clip_to_allocation: true,
            x_expand: true,
            y_expand: true
        });
        contentLayout.set_size(monitor.width, monitor.height);
        container.add_child(contentLayout);

        // Click anywhere to focus password entry on primary monitor
        container.connect('button-press-event', () => {
            if (this._passwordEntry) {
                let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
                activeText.grab_key_focus();
            }
            return Clutter.EVENT_PROPAGATE;
        });

        if (isPrimary) {
            // 1. Top bar for power actions (Shutdown, Reboot)
            let topBar = new St.BoxLayout({
                style_class: 'pulsaros-lockscreen-topbar',
                x_align: Clutter.ActorAlign.END,
                y_align: Clutter.ActorAlign.START
            });
            contentLayout.add_child(topBar);

            let rebootBtn = new St.Button({
                style_class: 'pulsaros-lockscreen-power-button',
                reactive: true,
                can_focus: true,
                child: new St.Icon({
                    icon_name: 'system-restart-symbolic',
                    icon_size: 20
                })
            });
            rebootBtn.connect('clicked', () => {
                GLib.spawn_command_line_async("systemctl reboot");
            });
            topBar.add_child(rebootBtn);

            let shutdownBtn = new St.Button({
                style_class: 'pulsaros-lockscreen-power-button',
                reactive: true,
                can_focus: true,
                child: new St.Icon({
                    icon_name: 'system-shutdown-symbolic',
                    icon_size: 20
                })
            });
            shutdownBtn.connect('clicked', () => {
                GLib.spawn_command_line_async("systemctl poweroff");
            });
            topBar.add_child(shutdownBtn);

            let spacer = new St.Widget({
                style_class: 'pulsaros-lockscreen-spacer',
                height: 60
            });
            contentLayout.add_child(spacer);

            // 2. Central Clock displays
            let clockBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.START,
                style_class: 'pulsaros-lockscreen-clock-box'
            });
            contentLayout.add_child(clockBox);

            let dateLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-date-label',
                text: ''
            });
            clockBox.add_child(dateLabel);

            let timeLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-time-label',
                text: '00:00'
            });
            clockBox.add_child(timeLabel);

            this._clocks.push({ timeLabel, dateLabel });

            let middleSpacer = new St.Widget({
                y_expand: true,
                style_class: 'pulsaros-lockscreen-middle-spacer'
            });
            contentLayout.add_child(middleSpacer);

            // 3. User Credentials login card
            let userCard = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.END,
                style_class: 'pulsaros-lockscreen-user-card'
            });
            contentLayout.add_child(userCard);

            let username = GLib.get_user_name();
            let avatarWidget = new St.Widget({
                style_class: 'pulsaros-lockscreen-avatar',
                x_align: Clutter.ActorAlign.CENTER
            });

            try {
                let avatarPath = `/var/lib/AccountsService/icons/${username}`;
                let avatarFile = Gio.File.new_for_path(avatarPath);
                if (avatarFile.query_exists(null)) {
                    avatarWidget.style = `background-image: url("file://${avatarPath}"); background-size: cover; border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9);`;
                } else {
                    let faceFile = Gio.File.new_for_path(GLib.get_home_dir() + '/.face');
                    if (faceFile.query_exists(null)) {
                        avatarWidget.style = `background-image: url("file://${GLib.get_home_dir()}/.face"); background-size: cover; border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9);`;
                    } else {
                        avatarWidget.style = `border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9); background-color: rgba(255, 255, 255, 0.15);`;
                        let defaultIcon = new St.Icon({
                            icon_name: 'avatar-default-symbolic',
                            icon_size: 64,
                            style_class: 'pulsaros-lockscreen-avatar-default',
                            x_align: Clutter.ActorAlign.CENTER,
                            y_align: Clutter.ActorAlign.CENTER
                        });
                        avatarWidget.add_child(defaultIcon);
                    }
                }
            } catch (e) {
                console.error("[LockScreen] Failed to load avatar:", e);
                avatarWidget.style = `border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9); background-color: rgba(255, 255, 255, 0.15);`;
                let defaultIcon = new St.Icon({
                    icon_name: 'avatar-default-symbolic',
                    icon_size: 64,
                    style_class: 'pulsaros-lockscreen-avatar-default',
                    x_align: Clutter.ActorAlign.CENTER,
                    y_align: Clutter.ActorAlign.CENTER
                });
                avatarWidget.add_child(defaultIcon);
            }
            userCard.add_child(avatarWidget);

            let realName = username;
            try {
                let gn = GLib.get_real_name();
                if (gn && gn !== 'Unknown' && gn.trim() !== '') {
                    realName = gn;
                }
            } catch (e) {}
            realName = realName.charAt(0).toUpperCase() + realName.slice(1);

            let nameLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-name-label',
                x_align: Clutter.ActorAlign.CENTER,
                text: realName
            });
            userCard.add_child(nameLabel);

            this._passwordEntry = new St.Entry({
                style_class: 'pulsaros-lockscreen-entry',
                x_align: Clutter.ActorAlign.CENTER,
                hint_text: 'Enter Password',
                can_focus: true,
                reactive: true
            });

            let clutterText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText;
            if (!clutterText && typeof this._passwordEntry.get_clutter_text === 'function') {
                clutterText = this._passwordEntry.get_clutter_text();
            }

            if (clutterText) {
                clutterText.set_password_char('●');
                clutterText.connect('activate', () => {
                    let password = this._passwordEntry.get_text();
                    if (password && password.length > 0) {
                        this._authenticate(password);
                    }
                });
                clutterText.connect('text-changed', () => {
                    this._passwordEntry.style_class = 'pulsaros-lockscreen-entry';
                    this._passwordEntry.set_hint_text('Enter Password');
                });
                clutterText.connect('key-press-event', (actor, event) => {
                    let symbol = event.get_key_symbol();
                    if (symbol === Clutter.KEY_Escape) {
                        this._passwordEntry.set_text('');
                        return Clutter.EVENT_STOP;
                    }
                    return Clutter.EVENT_PROPAGATE;
                });
            }
            userCard.add_child(this._passwordEntry);

            let bottomSpacer = new St.Widget({
                style_class: 'pulsaros-lockscreen-bottom-spacer',
                height: 40
            });
            contentLayout.add_child(bottomSpacer);
        } else {
            // Secondary Monitor: Center clean standard clock
            let topSpacer = new St.Widget({
                y_expand: true
            });
            contentLayout.add_child(topSpacer);

            let clockBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER,
                style_class: 'pulsaros-lockscreen-clock-box'
            });
            contentLayout.add_child(clockBox);

            let dateLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-date-label',
                style: 'font-size: 28px; font-weight: 500; text-shadow: 0px 2px 10px rgba(0, 0, 0, 0.4); font-family: "SF Pro Text", "Cantarell", sans-serif; color: rgba(255, 255, 255, 0.9); text-align: center;',
                text: ''
            });
            clockBox.add_child(dateLabel);

            let timeLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-time-label',
                style: 'font-size: 140px; font-weight: bold; text-shadow: 0px 4px 18px rgba(0, 0, 0, 0.5); font-family: "SF Pro Display", "SF Pro Text", "Cantarell", sans-serif; color: #ffffff; text-align: center;',
                text: '00:00'
            });
            clockBox.add_child(timeLabel);

            this._clocks.push({ timeLabel, dateLabel });

            let bottomSpacer = new St.Widget({
                y_expand: true
            });
            contentLayout.add_child(bottomSpacer);
        }
    }
    
    _updateClock() {
        let now = GLib.DateTime.new_now_local();
        let timeStr = now.format('%H:%M');
        let dateStr = now.format('%A, %B %d');

        if (this._clocks) {
            for (let clock of this._clocks) {
                if (clock.dateLabel) clock.dateLabel.set_text(dateStr);
                if (clock.timeLabel) clock.timeLabel.set_text(timeStr);
            }
        }
    }
    
    _resetLockIdleTimer() {
        if (!this._isLocked) return;
        if (this._lockIdleTimerId) {
            GLib.source_remove(this._lockIdleTimerId);
            this._lockIdleTimerId = 0;
        }
        this._lockIdleTimerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, this._lockIdleTimeoutSeconds, () => {
            this._lockIdleTimerId = 0;
            if (this._isLocked) {
                this._suspendOnLockIdle();
            }
            return GLib.SOURCE_REMOVE;
        });
    }

    _suspendOnLockIdle() {
        if (!this._isLocked) return;
        try {
            let bus = Gio.DBus.system;
            bus.call(
                'org.freedesktop.login1',
                '/org/freedesktop/login1',
                'org.freedesktop.login1.Manager',
                'Suspend',
                new GLib.Variant('(b)', [true]),
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                (connection, res) => {
                    try {
                        connection.call_finish(res);
                    } catch (e) {
                        GLib.spawn_command_line_async("systemctl suspend");
                    }
                }
            );
        } catch (e) {
            console.error("[LockScreen] Failed to trigger lockscreen idle suspend:", e);
            GLib.spawn_command_line_async("systemctl suspend");
        }
    }
    
    lock() {
        if (this._isLocked) {
            // Already locked: ensure top of uiGroup stack and reset lock idle timer
            try {
                let parent = this.get_parent();
                if (parent) {
                    parent.set_child_at_index(this, -1);
                }
            } catch (e) {}
            this._resetLockIdleTimer();
            if (this._passwordEntry) {
                let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
                if (activeText && activeText.grab_key_focus) activeText.grab_key_focus();
            }
            return;
        }
        this._isLocked = true;
        this.visible = true;
        this.opacity = 255;
        this.reactive = true;
        this.set_position(0, 0);
        this.set_size(global.stage.width, global.stage.height);
        
        this._rebuildMonitors();
        this._updateWallpapers();
        
        if (this._passwordEntry) {
            this._passwordEntry.text = '';
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry';
        }
        this._authenticating = false;
        
        // Put lockscreen overlay on the absolute top of the uiGroup stack
        try {
            let parent = this.get_parent();
            if (parent) {
                parent.set_child_at_index(this, -1);
            }
        } catch (e) {
            console.error("[LockScreen] Failed to raise lockscreen overlay via set_child_at_index:", e);
            try {
                Main.uiGroup.set_child_above_sibling(this, null);
            } catch (e2) {
                console.error("[LockScreen] Fallback set_child_above_sibling failed too:", e2);
            }
        }
        
        // Defer input grab and key focus to the next main loop cycle to guarantee the actor is mapped
        GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
            if (!this._isLocked) return GLib.SOURCE_REMOVE;
            
            if (Main.pushModal(this)) {
                this._hasGrab = true;
            } else {
                console.error("[LockScreen] Failed to acquire input grab");
                this._hasGrab = false;
            }
            
            if (this._passwordEntry) {
                let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
                if (activeText && activeText.grab_key_focus) activeText.grab_key_focus();
            }
            return GLib.SOURCE_REMOVE;
        });
        
        // Start live clock updates
        this._updateClock();
        this._timerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 1, () => {
            this._updateClock();
            return GLib.SOURCE_CONTINUE;
        });

        // Start lockscreen inactivity timer for automatic suspension
        this._resetLockIdleTimer();

        // Listen to global events to reset the inactivity timer
        if (!this._stageEventId) {
            this._stageEventId = global.stage.connect('captured-event', () => {
                if (this._isLocked) {
                    this._resetLockIdleTimer();
                }
                return Clutter.EVENT_PROPAGATE;
            });
        }
    }
    
    unlock() {
        if (!this._isLocked) return;
        this._isLocked = false;
        this.visible = false;
        this.opacity = 0;
        this.reactive = false;
        this.set_size(0, 0);
        
        this._stopVideoWallpaper();
        
        // Destroy monitor containers when unlocked
        if (this._monitorContainers) {
            for (let container of this._monitorContainers) {
                container.destroy();
            }
        }
        this._monitorContainers = [];
        this._clocks = [];
        this._passwordEntry = null;
        
        // Release modal input grab
        if (this._hasGrab) {
            Main.popModal(this);
            this._hasGrab = false;
        }
        
        // Clean clock timer
        if (this._timerId) {
            GLib.source_remove(this._timerId);
            this._timerId = 0;
        }

        // Clean lock idle timer and stage event listener
        if (this._lockIdleTimerId) {
            GLib.source_remove(this._lockIdleTimerId);
            this._lockIdleTimerId = 0;
        }
        if (this._stageEventId) {
            global.stage.disconnect(this._stageEventId);
            this._stageEventId = 0;
        }
    }
    
    _authenticate(password) {
        if (this._authenticating) return;
        this._authenticating = true;
        
        if (this._passwordEntry) {
            this._passwordEntry.set_reactive(false);
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry-authenticating';
        }
        
        let username = GLib.get_user_name();
        
        // Check if the PAM service file is present. If not, fallback to developer passwords for local testing on host
        let pamFile = Gio.File.new_for_path('/etc/pam.d/pulsaros-lock');
        if (!pamFile.query_exists(null)) {
            console.warn("[LockScreen] PAM service '/etc/pam.d/pulsaros-lock' is missing. Falling back to developer passwords.");
            if (password === 'pulsar' || password === 'live' || password === 'jaime') {
                this._onAuthSuccess();
            } else {
                this._onAuthFailure();
            }
            return;
        }
        
        try {
            // Run pamtester asynchronously, piping the password via stdin
            let proc = Gio.Subprocess.new(
                ['/usr/bin/pamtester', 'pulsaros-lock', username, 'authenticate'],
                Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
            );
            
            proc.communicate_utf8_async(password + '\n', null, (obj, res) => {
                try {
                    let [ok, , stderrText] = obj.communicate_utf8_finish(res);
                    let success = ok && obj.get_successful();
                    if (!success) {
                        console.warn(`[LockScreen] pamtester auth failed for user '${username}': exit=${obj.get_exit_status()} stderr=${(stderrText || '').trim()}`);
                    }
                    if (success) {
                        this._onAuthSuccess();
                    } else {
                        this._onAuthFailure();
                    }
                } catch (e) {
                    console.error("[LockScreen] pamtester wait error:", e);
                    this._onAuthFailure();
                }
            });
        } catch (e) {
            console.error("[LockScreen] pamtester launch failed:", e);
            this._onAuthFailure();
        }
    }
    
    _onAuthSuccess() {
        this._authenticating = false;
        this.unlock();
    }
    
    _onAuthFailure() {
        this._authenticating = false;
        if (this._passwordEntry) {
            this._passwordEntry.set_reactive(true);
            this._passwordEntry.set_text('');
            this._passwordEntry.set_hint_text('Incorrect Password');
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry-failed';
            this._passwordEntry.grab_key_focus();
            
            // Shake animation
            let originalX = this._passwordEntry.translation_x;
            let shakeOffset = 10;
            let step = 0;
            let shakeInterval = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 50, () => {
                if (step >= 6) {
                    if (this._passwordEntry) {
                        this._passwordEntry.translation_x = originalX;
                    }
                    return GLib.SOURCE_REMOVE;
                }
                if (this._passwordEntry) {
                    this._passwordEntry.translation_x = originalX + (step % 2 === 0 ? shakeOffset : -shakeOffset);
                }
                step++;
                return GLib.SOURCE_CONTINUE;
            });
        }
    }
    
    destroy() {
        this._stopVideoWallpaper();
        if (this._bgChangedId1 && this._bgSettings) {
            this._bgSettings.disconnect(this._bgChangedId1);
            this._bgChangedId1 = 0;
        }
        if (this._bgChangedId2 && this._bgSettings) {
            this._bgSettings.disconnect(this._bgChangedId2);
            this._bgChangedId2 = 0;
        }
        if (this._bgChangedId3 && this._ifaceSettings) {
            this._ifaceSettings.disconnect(this._bgChangedId3);
            this._bgChangedId3 = 0;
        }
        if (this._monitorsChangedId) {
            Main.layoutManager.disconnect(this._monitorsChangedId);
            this._monitorsChangedId = 0;
        }
        if (this._sizeChangedId) {
            global.stage.disconnect(this._sizeChangedId);
            this._sizeChangedId = 0;
        }
        if (this._sizeChangedId2) {
            global.stage.disconnect(this._sizeChangedId2);
            this._sizeChangedId2 = 0;
        }
        if (this._timerId) {
            GLib.source_remove(this._timerId);
            this._timerId = 0;
        }
        if (this._lockIdleTimerId) {
            GLib.source_remove(this._lockIdleTimerId);
            this._lockIdleTimerId = 0;
        }
        if (this._stageEventId) {
            global.stage.disconnect(this._stageEventId);
            this._stageEventId = 0;
        }
        if (this._isLocked && this._hasGrab) {
            Main.popModal(this);
        }
        if (this._monitorContainers) {
            for (let container of this._monitorContainers) {
                container.destroy();
            }
        }
        this._monitorContainers = [];
        super.destroy();
    }
});

class DesktopLiveWallpaperManager {
    constructor(extension) {
        this._extension = extension;
        this._videoPipeline = null;
        this._videoSink = null;
        this._videoTimerId = 0;
        this._busWatchId = 0;
        this._bgActor = null;
        this._videoContent = null;
        this._currentFile = null;

        this._setupConfigMonitor();
        this._updateWallpaper();
    }

    _setupConfigMonitor() {
        try {
            let homeDir = GLib.get_home_dir();
            let configDir = Gio.File.new_for_path(GLib.build_filenamev([homeDir, '.config', 'pulsaros']));
            if (!configDir.query_exists(null)) {
                configDir.make_directory_with_parents(null);
            }
            this._monitor = configDir.monitor_directory(Gio.FileMonitorFlags.NONE, null);
            this._monitorId = this._monitor.connect('changed', (mon, file, other, eventType) => {
                let name = file.get_basename();
                if (name === 'live-wallpaper.json') {
                    this._updateWallpaper();
                }
            });
        } catch (e) {
            console.error("[DesktopLiveWallpaper] Failed to setup config monitor:", e);
        }
    }

    _getConfig() {
        try {
            let homeDir = GLib.get_home_dir();
            let cfgFile = Gio.File.new_for_path(GLib.build_filenamev([homeDir, '.config', 'pulsaros', 'live-wallpaper.json']));
            if (cfgFile.query_exists(null)) {
                let [ok, contents] = cfgFile.load_contents(null);
                if (ok) {
                    return JSON.parse(new TextDecoder().decode(contents));
                }
            }
        } catch (e) {}
        return null;
    }

    _isVideoFile(path) {
        if (!path) return false;
        let clean = path.toLowerCase();
        return clean.endsWith('.mp4') || clean.endsWith('.webm') || clean.endsWith('.mkv') || clean.endsWith('.mov') || clean.endsWith('.avi');
    }

    _updateWallpaper() {
        let cfg = this._getConfig();
        if (cfg && cfg.enabled && cfg.file && (cfg.type === 'video' || this._isVideoFile(cfg.file))) {
            let localPath = cfg.file.startsWith('file://') ? decodeURIComponent(cfg.file.substring(7)) : cfg.file;
            if (GLib.file_test(localPath, GLib.FileTest.EXISTS)) {
                if (this._currentFile !== localPath) {
                    this._startVideo(localPath);
                }
                return;
            }
        }
        this._stopVideo();
    }

    _startVideo(videoPath) {
        this._stopVideo();
        this._currentFile = videoPath;
        try {
            Gst.init(null);
            let file = Gio.File.new_for_path(videoPath);
            let videoUri = file.get_uri();

            this._bgActor = new Clutter.Actor({
                name: 'pulsaros-desktop-live-wallpaper',
                x: 0,
                y: 0,
                width: global.stage.width,
                height: global.stage.height
            });

            if (Main.layoutManager._backgroundGroup) {
                Main.layoutManager._backgroundGroup.add_child(this._bgActor);
                Main.layoutManager._backgroundGroup.set_child_at_index(this._bgActor, 0);
            }

            let videoSinkBin = Gst.parse_bin_from_description(
                'videoconvert ! video/x-raw,format=RGBA ! appsink name=sink emit-signals=false max-buffers=2 drop=true sync=false',
                true
            );
            this._videoPipeline = Gst.ElementFactory.make('playbin', 'desktop-live-wallpaper');
            this._videoPipeline.set_property('uri', videoUri);
            this._videoPipeline.set_property('video-sink', videoSinkBin);
            let audioSink = Gst.ElementFactory.make('fakesink', 'desktop-audiosink');
            this._videoPipeline.set_property('audio-sink', audioSink);

            this._videoSink = videoSinkBin.get_by_name('sink');

            let bus = this._videoPipeline.get_bus();
            bus.add_signal_watch();
            this._busWatchId = bus.connect('message::eos', () => {
                if (this._videoPipeline) {
                    this._videoPipeline.seek_simple(Gst.Format.TIME, Gst.SeekFlags.FLUSH | Gst.SeekFlags.KEY_UNIT, 0);
                }
            });

            this._videoPipeline.set_state(Gst.State.PLAYING);

            this._videoTimerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 33, () => {
                if (!this._bgActor || !this._videoSink) {
                    return GLib.SOURCE_CONTINUE;
                }
                try {
                    let sample = this._videoSink.try_pull_sample(0);
                    if (sample) {
                        let buffer = sample.get_buffer();
                        let caps = sample.get_caps();
                        let s = caps.get_structure(0);
                        let [okW, width] = s.get_int('width');
                        let [okH, height] = s.get_int('height');
                        let [okMap, mapInfo] = buffer.map(Gst.MapFlags.READ);
                        if (okMap) {
                            let bytes = GLib.Bytes.new(mapInfo.data);
                            buffer.unmap(mapInfo);
                            if (!this._videoContent) {
                                this._videoContent = new Clutter.Image();
                                this._bgActor.set_content(this._videoContent);
                            }
                            this._videoContent.set_bytes(
                                bytes,
                                Cogl.PixelFormat.RGBA_8888,
                                width,
                                height,
                                width * 4
                            );
                        }
                    }
                } catch (pullErr) {}
                return GLib.SOURCE_CONTINUE;
            });
        } catch (e) {
            console.error("[DesktopLiveWallpaper] Failed to start live wallpaper:", e);
        }
    }

    _stopVideo() {
        this._currentFile = null;
        if (this._videoTimerId) {
            GLib.source_remove(this._videoTimerId);
            this._videoTimerId = 0;
        }
        if (this._busWatchId && this._videoPipeline) {
            let bus = this._videoPipeline.get_bus();
            bus.disconnect(this._busWatchId);
            this._busWatchId = 0;
        }
        if (this._videoPipeline) {
            this._videoPipeline.set_state(Gst.State.NULL);
            this._videoPipeline = null;
        }
        this._videoSink = null;
        this._videoContent = null;
        if (this._bgActor) {
            this._bgActor.destroy();
            this._bgActor = null;
        }
    }

    destroy() {
        this._stopVideo();
        if (this._monitorId && this._monitor) {
            this._monitor.disconnect(this._monitorId);
            this._monitorId = 0;
        }
    }
}

const IGNORED_APPS = [
    'welcome.py',
    'recovery.py',
    'welcome',
    'recovery',
    'pulsaros-welcome',
    'pulsaros-recovery',
    'org.pulsaros.welcome',
    'org.pulsaros.recovery',
    'calamares',
    'io.calamares.calamares',
    'gnome-control-center',
    'org.gnome.Settings',
    'spotlight-gtk',
    'io.github.jeffshee.Hidamari',
    'hidamari',
    'pulsaros-live-wallpaper',
    'ding',
    'org.rastersoft.ding',
    'ding.js',
    'desktop-icons',
    'gcr-prompter',
    'polkit-gnome-authentication-agent-1'
];

class MacOSFullscreenManager {
    constructor(extension) {
        this._extension = extension;
        this._windowSignals = new Map();
        this._spaceWindows = new Map();
        this._enabled = false;
        this._panelVisible = true;
        this._hideTimeoutId = 0;
        this._pointerWatcher = null;
        this._pointerWatch = null;
        this._wsChangedId = 0;
        this._windowCreatedId = 0;
        this._stageResizeId = 0;
        this._topTrigger = null;
        this._refreshingGeometry = false;

        this._setupSettings();
        this._setupSignals();
    }

    // ─── Utilities ─────────────────────────────────────────────────────────────

    _hasOpenMenu() {
        try {
            if (Main.overview?.visible) return true;
            if (Main.panel?.menuManager?.activeMenu) return true;
            if (Main.panel?.statusArea) {
                for (let k in Main.panel.statusArea) {
                    let item = Main.panel.statusArea[k];
                    if (item?.menu?.isOpen) return true;
                }
            }
        } catch (e) {}
        return false;
    }

    _isIgnoredWindow(win) {
        if (!win) return true;
        try {
            if (win.is_override_redirect?.()) return true;
            if (win.is_skip_taskbar?.()) return true;
            if (win.is_on_all_workspaces?.()) return true;
            let type = win.get_window_type?.() ?? Meta.WindowType.NORMAL;
            if (type !== Meta.WindowType.NORMAL) return true;
            if (win.get_transient_for?.() !== null) return true;

            let wmClass = win.get_wm_class?.() ?? '';
            let appId  = win.get_gtk_application_id?.() ?? '';
            let title  = win.get_title?.() ?? '';
            let sand   = win.get_sandboxed_app_id?.() ?? '';
            let cmdline = '';
            let pid = win.get_pid?.() ?? 0;
            if (pid > 0) {
                try {
                    let [ok, buf] = GLib.file_get_contents(`/proc/${pid}/cmdline`);
                    if (ok) cmdline = new TextDecoder().decode(buf).replace(/\0/g, ' ');
                } catch (_) {}
            }
            let check = `${wmClass} ${appId} ${title} ${sand} ${cmdline}`.toLowerCase();
            for (let ign of IGNORED_APPS)
                if (check.includes(ign.toLowerCase())) return true;
        } catch (e) { return true; }
        return false;
    }

    _isSpaceWorkspace(ws) {
        if (!this._enabled || !ws) return false;
        if (this._spaceWindows.size === 0) return false;

        for (let [win] of this._spaceWindows) {
            try {
                if (win && !win.unmanaged && !win.minimized && win.get_workspace() === ws) {
                    let isMax = (win.maximized_horizontally && win.maximized_vertically) || (win.is_fullscreen && win.is_fullscreen());
                    if (isMax) return true;
                }
            } catch (_) {}
        }
        return false;
    }

    _isCurrentWorkspaceFullscreenSpace() {
        if (!this._enabled) return false;
        return this._isSpaceWorkspace(global.workspace_manager?.get_active_workspace());
    }

    // ─── Settings ──────────────────────────────────────────────────────────────

    _setupSettings() {
        try {
            let schema = 'org.gnome.shell.extensions.pulsaros-global-menu';
            if (Gio.SettingsSchemaSource.get_default()?.lookup(schema, true)) {
                this._settings = new Gio.Settings({ schema_id: schema });
                this._enabled = this._settings.get_boolean('macos-fullscreen-spaces');
                this._settings.connect('changed::macos-fullscreen-spaces', () => {
                    this._enabled = this._settings.get_boolean('macos-fullscreen-spaces');
                    this._syncPanelForCurrentWorkspace();
                });
            }
        } catch (_) {}
    }

    // ─── Signal Setup ──────────────────────────────────────────────────────────

    _setupSignals() {
        this._windowCreatedId = global.display.connect('window-created', (_d, win) => {
            this._trackWindow(win);
        });
        for (let actor of global.get_window_actors())
            this._trackWindow(actor.meta_window);

        this._wsChangedId = global.workspace_manager.connect('active-workspace-changed', () => {
            this._syncPanelForCurrentWorkspace();
        });

        // PointerWatcher for reliable edge detection
        try {
            this._pointerWatcher = getPointerWatcher();
            this._pointerWatch = this._pointerWatcher.addWatch(50, (x, y) => this._onPointerMoved(x, y));
        } catch (_) {}

        this._syncPanelForCurrentWorkspace();
    }

    _trackWindow(win) {
        if (this._isIgnoredWindow(win)) return;
        let id = win.get_id?.();
        if (!id || this._windowSignals.has(id)) return;

        let sigs = [
            win.connect('notify::maximized-horizontally', () => this._onMaximizeChanged(win)),
            win.connect('notify::maximized-vertically',   () => this._onMaximizeChanged(win)),
            win.connect('notify::minimized',              () => this._syncPanelForCurrentWorkspace()),
            win.connect('workspace-changed',              () => this._syncPanelForCurrentWorkspace()),
            win.connect('unmanaged',                      () => this._untrackWindow(win)),
        ];
        this._windowSignals.set(id, { win, sigs });
    }

    // ─── Workspace / Maximize Logic ────────────────────────────────────────────

    _onMaximizeChanged(win) {
        if (!this._enabled || win._pulsarLock || this._isIgnoredWindow(win)) return;

        let isMax     = win.maximized_horizontally && win.maximized_vertically;
        let isTracked = this._spaceWindows.has(win);

        if (isMax && !isTracked) {
            win._pulsarLock = true;
            try {
                let wsm   = global.workspace_manager;
                let curWs = wsm.get_active_workspace();
                let origI = curWs.index();

                let newWs = wsm.append_new_workspace(false, global.get_current_time());
                let dest  = origI + 1;
                if (dest < wsm.n_workspaces && wsm.reorder_workspace)
                    wsm.reorder_workspace(newWs, dest);

                this._spaceWindows.set(win, { origWsIndex: origI });
                win.change_workspace(newWs);
                newWs.activate_with_focus(win, global.get_current_time());
            } catch (e) {
                console.error('[MacOSFullscreen] move workspace:', e);
            } finally {
                win._pulsarLock = false;
            }
            this._syncPanelForCurrentWorkspace();

        } else if (!isMax && isTracked) {
            this._restoreWindow(win);
            this._syncPanelForCurrentWorkspace();
        }
    }

    _restoreWindow(win) {
        let saved = this._spaceWindows.get(win);
        if (!saved) return;

        win._pulsarLock = true;
        try {
            this._spaceWindows.delete(win);

            let wsm = global.workspace_manager;
            let targetWs = wsm.get_workspace_by_index(
                Math.min(saved.origWsIndex, wsm.n_workspaces - 1));

            if (win.maximized_horizontally || win.maximized_vertically)
                win.unmaximize(Meta.MaximizeFlags.BOTH);

            if (targetWs) {
                win.change_workspace(targetWs);
                targetWs.activate_with_focus(win, global.get_current_time());
            }
        } catch (e) {
            console.error('[MacOSFullscreen] restore window:', e);
        } finally {
            win._pulsarLock = false;
            this._syncPanelForCurrentWorkspace();
        }
    }

    // ─── Struts ────────────────────────────────────────────────────────────────

    /**
     * Toggle affectsStruts on the panelBox Chrome entry WITHOUT removing it
     * (removing it from chrome breaks GNOME Shell's panel lifecycle).
     * After toggling, we must force maximized windows to recalculate their geometry
     * by briefly unmaximizing + re-maximizing them with _pulsarLock held.
     */
    _setPanelStruts(affectsStruts) {
        try {
            let chrome = Main.layoutManager._chrome;
            let entry = chrome?._trackedActors?.find(a => a.actor === Main.layoutManager.panelBox);
            if (entry && entry.affectsStruts !== affectsStruts) {
                entry.affectsStruts = affectsStruts;
                // trackFullscreen must be false so panel reacts normally
                entry.trackFullscreen = false;
                Main.layoutManager._queueUpdateRegions?.();
            }
        } catch (e) {
            console.error('[MacOSFullscreen] struts:', e);
        }
    }

    _forceWindowsRefreshGeometry(ws) {
        // Guard against re-entry
        if (this._refreshingGeometry) return;
        this._refreshingGeometry = true;

        // Use GLib.PRIORITY_DEFAULT so this runs after _updateRegions (which uses
        // Meta.later_add BEFORE_REDRAW). By this point struts=0 in Mutter.
        GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
            try {
                for (let [win] of this._spaceWindows) {
                    try {
                        if (win.unmanaged || win.get_workspace() !== ws) continue;
                        if (!win.maximized_horizontally || !win.maximized_vertically) continue;

                        let actor = win.get_compositor_private();
                        win._pulsarLock = true;

                        // Hide actor so GNOME Shell skips unmaximize/maximize animations
                        if (actor) actor.hide();
                        try {
                            win.unmaximize(Meta.MaximizeFlags.BOTH);
                            // At this point Mutter work area = (0,0,w,h) since struts=0
                            win.maximize(Meta.MaximizeFlags.BOTH);
                        } finally {
                            if (actor) actor.show();
                            // Release lock right away – notify:: signals fire synchronously
                            // during unmaximize/maximize, while the lock was still held
                            win._pulsarLock = false;
                        }
                    } catch (_) {}
                }
            } finally {
                this._refreshingGeometry = false;
            }
            return GLib.SOURCE_REMOVE;
        });
    }

    // ─── Panel show/hide ───────────────────────────────────────────────────────

    _showPanel(animated) {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = 0;
        }
        this._panelVisible = true;

        let panelBox = Main.layoutManager.panelBox;
        let panelH   = panelBox?.height || 36;

        Main.panel.remove_all_transitions();
        Main.panel.translation_y = 0;
        Main.panel.reactive = true;
        Main.panel.opacity  = 255;

        if (!panelBox) return;

        if (animated) {
            // Slide in from off-screen: start at y=-panelH then ease to y=0
            // affectsStruts is still false (space mode), so showing doesn't add struts
            panelBox.remove_all_transitions();
            panelBox.ease({
                y: 0,
                duration: 200,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            });
        } else {
            // Normal workspace: restore y=0 so get_transformed_position() is on-screen → struts restored
            panelBox.remove_all_transitions();
            panelBox.set_y(0);
            // Synchronously update regions so struts take effect immediately
            try { Main.layoutManager._updateRegions?.(); } catch (_) {}
            Main.layoutManager._queueUpdateRegions?.();
        }
    }

    _hidePanel(animated) {
        if (this._hasOpenMenu()) return;
        this._panelVisible = false;

        let panelBox = Main.layoutManager.panelBox;
        let panelH   = panelBox?.height || 36;

        Main.panel.remove_all_transitions();
        Main.panel.translation_y = 0;

        if (!panelBox) return;

        if (animated) {
            // Slide panel upward by animating its actual y position.
            // When y = -panelH, get_transformed_position() returns (0, -panelH)
            // → _updateRegions computes strut rect at (0, -panelH, w, panelH)
            // → Mutter clips to screen: effective top strut = 0
            panelBox.remove_all_transitions();
            panelBox.ease({
                y: -panelH,
                duration: 200,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            });
        } else {
            // Move panelBox off-screen immediately.
            // y=-panelH → get_transformed_position returns off-screen → struts=0
            panelBox.remove_all_transitions();
            panelBox.set_y(-panelH);
        }
    }

    // ─── Master sync ───────────────────────────────────────────────────────────

    _syncPanelForCurrentWorkspace() {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = 0;
        }

        if (!this._enabled) {
            this._setPanelStruts(true);
            this._showPanel(false);
            return;
        }

        let activeWs = global.workspace_manager?.get_active_workspace();
        let isSpace  = this._isSpaceWorkspace(activeWs);

        if (isSpace) {
            // 1. Set affectsStruts=false (belt-and-suspenders)
            this._setPanelStruts(false);
            // 2. Move panelBox to y=-panelH so get_transformed_position() is off-screen
            //    → _updateRegions computes struts=0 regardless of affectsStruts
            this._hidePanel(false);
            try { Main.layoutManager._updateRegions?.(); } catch (_) {}
            Main.layoutManager._queueUpdateRegions?.();
            // 4. Now that Mutter has struts=0, force windows to recalculate their maximized rect
            this._forceWindowsRefreshGeometry(activeWs);
        } else {
            // Restore struts first, then show panel (order matters for window repositioning)
            this._setPanelStruts(true);
            this._showPanel(false);
        }
    }

    // ─── Pointer hover ─────────────────────────────────────────────────────────

    _onPointerMoved(x, y) {
        if (!this._enabled) return;
        if (!this._isCurrentWorkspaceFullscreenSpace()) {
            if (!this._panelVisible || (Main.layoutManager.panelBox && Main.layoutManager.panelBox.y !== 0)) {
                this._setPanelStruts(true);
                this._showPanel(false);
            }
            return;
        }

        let panelH = (Main.panel.height || 36) + 16;

        if (y <= 4) {
            // Cursor at top edge – show panel
            if (this._hideTimeoutId) {
                GLib.source_remove(this._hideTimeoutId);
                this._hideTimeoutId = 0;
            }
            if (!this._panelVisible) this._showPanel(true);
        } else if (y < panelH) {
            // Cursor still inside panel area – cancel any pending hide
            if (this._hideTimeoutId) {
                GLib.source_remove(this._hideTimeoutId);
                this._hideTimeoutId = 0;
            }
        } else {
            // Cursor below panel – schedule hide
            if (this._panelVisible && !this._hasOpenMenu() && !this._hideTimeoutId) {
                this._hideTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 400, () => {
                    this._hideTimeoutId = 0;
                    if (this._isCurrentWorkspaceFullscreenSpace() && !this._hasOpenMenu())
                        this._hidePanel(true);
                    return GLib.SOURCE_REMOVE;
                });
            }
        }
    }

    // ─── Cleanup ───────────────────────────────────────────────────────────────

    _untrackWindow(win) {
        let id = win.get_id?.();
        if (id && this._windowSignals.has(id)) {
            let { sigs } = this._windowSignals.get(id);
            for (let s of sigs) try { win.disconnect(s); } catch (_) {}
            this._windowSignals.delete(id);
        }
        this._spaceWindows.delete(win);
        this._syncPanelForCurrentWorkspace();
    }

    destroy() {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = 0;
        }
        if (this._pointerWatch && this._pointerWatcher) {
            try { this._pointerWatcher._removeWatch?.(this._pointerWatch); } catch (_) {}
            this._pointerWatch = null;
        }
        if (this._topTrigger) {
            try { Main.layoutManager.removeChrome(this._topTrigger); this._topTrigger.destroy(); } catch (_) {}
            this._topTrigger = null;
        }
        if (this._windowCreatedId) {
            global.display.disconnect(this._windowCreatedId);
            this._windowCreatedId = 0;
        }
        if (this._wsChangedId) {
            global.workspace_manager.disconnect(this._wsChangedId);
            this._wsChangedId = 0;
        }
        if (this._stageResizeId) {
            global.stage.disconnect(this._stageResizeId);
            this._stageResizeId = 0;
        }
        for (let [, { win, sigs }] of this._windowSignals)
            for (let s of sigs) try { win.disconnect(s); } catch (_) {}
        this._windowSignals.clear();
        this._spaceWindows.clear();

        // Always restore struts and panel position on disable
        this._setPanelStruts(true);
        this._showPanel(false);
    }
}


export default class PulsarosGlobalMenuExtension extends Extension {
    enable() {
        this._menuButtons = [];
        this._focusNotifyId = 0;
        this._virtualKeyboard = null;
        this._origCanLock = null;
        this._origLock = null;
        this._origUnlock = null;
        this._activeAppWindow = null;
        this._activeChangedId = 0;
        this._activePowerDialog = null;
        this._prepareForSleepId = 0;
        this._login1SessionLockId = 0;

        try {
            this._prepareForSleepId = Gio.DBus.system.signal_subscribe(
                'org.freedesktop.login1',
                'org.freedesktop.login1.Manager',
                'PrepareForSleep',
                '/org/freedesktop/login1',
                null,
                Gio.DBusSignalFlags.NONE,
                (connection, senderName, objectPath, interfaceName, signalName, parameters) => {
                    try {
                        let [aboutToSuspend] = parameters.deep_unpack();
                        if (this._activePowerDialog) {
                            this._activePowerDialog._cleanup();
                            this._activePowerDialog.close();
                            this._activePowerDialog = null;
                        }
                    } catch (e) {
                        console.error("[GlobalMenu] Error handling PrepareForSleep:", e);
                    }
                }
            );
        } catch (e) {
            console.error("[GlobalMenu] Failed to subscribe to PrepareForSleep signal:", e);
        }

        // Also subscribe to login1 Session Lock signal
        try {
            this._login1SessionLockId = Gio.DBus.system.signal_subscribe(
                'org.freedesktop.login1',
                'org.freedesktop.login1.Session',
                'Lock',
                null,
                null,
                Gio.DBusSignalFlags.NONE,
                () => {
                    if (this._lockScreenOverlay) {
                        this._lockScreenOverlay.lock();
                    }
                }
            );
        } catch (e) {
            console.error("[GlobalMenu] Failed to subscribe to login1 Session Lock signal:", e);
        }

        // Desktop Live Wallpaper Manager
        try {
            this._desktopLiveWallpaperManager = new DesktopLiveWallpaperManager(this);
        } catch (e) {
            console.error("[GlobalMenu] Failed to start desktop live wallpaper manager:", e);
        }

        // macOS Fullscreen Spaces & Top Panel auto-hide manager
        try {
            this._macOSFullscreenManager = new MacOSFullscreenManager(this);
        } catch (e) {
            console.error("[GlobalMenu] Failed to start macOS fullscreen manager:", e);
        }

        // Initialize the virtual keyboard device for injecting keystrokes (Wayland native)
        // Inicializar el dispositivo de teclado virtual para inyectar pulsaciones de tecla (nativo en Wayland)
        try {
            let seat = Clutter.get_default_backend().get_default_seat();
            if (seat) {
                this._virtualKeyboard = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to create virtual keyboard device:", e);
        }
        
        // Create lockscreen overlay instance and add it to top of uiGroup
        // Crear la instancia de pantalla de bloqueo y añadirla a uiGroup
        this._lockScreenOverlay = new LockScreen(this);
        Main.uiGroup.add_child(this._lockScreenOverlay);
        
        // Bind the native lock-screen media key shortcut (Super+L) to our custom LockScreen overlay
        // Vincular el atajo de teclado nativo de bloqueo (Super+L) a nuestra pantalla de bloqueo
        try {
            Main.wm.addKeybinding(
                'screensaver',
                new Gio.Settings({ schema_id: 'org.gnome.settings-daemon.plugins.media-keys' }),
                Meta.KeyBindingFlags.NONE,
                Shell.ActionMode.ALL,
                () => {
                    this._lockScreenOverlay.lock();
                }
            );
        } catch (e) {
            console.error("[GlobalMenu] Failed to bind screensaver keybinding:", e);
        }
        
        // Create each of the menu buttons
        // Crear cada uno de los botones de menú
        this._createLogoMenu();
        this._createAppMenu();
        this._createFileMenu();
        this._createEditMenu();
        this._createGoMenu();
        this._createWindowMenu();
        this._createHelpMenu();
        
        // Add menu buttons to GNOME top panel (Left Box)
        // Añadir botones de menú al panel superior de GNOME (caja izquierda)
        let pos = 1; 
        for (let button of this._menuButtons) {
            Main.panel.addToStatusArea(`global-menu-${button.uuid_suffix}`, button, pos++, 'left');
        }
        
        // Listen to window focus change notifications from display
        // Escuchar notificaciones de cambio de foco de ventana de la pantalla
        this._focusNotifyId = global.display.connect('notify::focus-window', () => {
            this._onFocusWindowChanged();
        });
        
        // Trigger initial focus state setup
        // Lanzar configuración inicial del estado de foco
        this._onFocusWindowChanged();
        
        // Force the native GNOME Shell lock button in Quick Settings to be always visible
        // Forzar a que el botón de bloqueo nativo en los Quick Settings de GNOME Shell sea siempre visible
        try {
            let actions = SystemActions?.getDefault ? SystemActions.getDefault() : null;
            if (actions) {
                this._origCanLock = Object.getOwnPropertyDescriptor(actions, 'can_lock');
                Object.defineProperty(actions, 'can_lock', {
                    get: () => true,
                    configurable: true
                });
                if (actions.notify) actions.notify('can-lock');
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to override can_lock:", e);
        }

        // Intercept native screenShield lock and unlock events to use our custom overlay
        // Interceptar eventos de bloqueo y desbloqueo nativos del screenshield para usar nuestra pantalla
        try {
            if (Main.screenShield) {
                this._origLock = Main.screenShield.lock;
                let self = this;
                Main.screenShield.lock = function(animate) {
                    try {
                        self._lockScreenOverlay.lock();
                    } catch (e) {
                        console.error("[GlobalMenu] Failed to trigger custom lock:", e);
                    }
                    return true;
                };
                
                this._origUnlock = Main.screenShield.unlock;
                Main.screenShield.unlock = function(animate) {
                    try {
                        self._lockScreenOverlay.unlock();
                    } catch (e) {
                        console.error("[GlobalMenu] Failed to trigger custom unlock:", e);
                    }
                    if (self._origUnlock) {
                        self._origUnlock.call(Main.screenShield, animate);
                    }
                };
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to override Main.screenShield:", e);
        }

        // Listen to native screenShield active status changes to unlock our overlay when the session is unlocked
        try {
            if (Main.screenShield) {
                this._activeChangedId = Main.screenShield.connect('active-changed', () => {
                    if (!Main.screenShield.active) {
                        if (this._lockScreenOverlay) {
                            this._lockScreenOverlay.unlock();
                        }
                    }
                });
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to connect to active-changed signal:", e);
        }
    }
    
    disable() {
        // Disconnect display focus notification signal
        // Desconectar la señal de notificación de foco de la pantalla
        if (this._focusNotifyId) {
            global.display.disconnect(this._focusNotifyId);
            this._focusNotifyId = 0;
        }
        
        // Unbind the screensaver keybinding
        // Desvincular el atajo de teclado de screensaver
        try {
            Main.wm.removeKeybinding('screensaver');
        } catch (e) {
            // ignore
        }
        
        // Destroy custom lockscreen overlay
        // Destruir la pantalla de bloqueo personalizada
        if (this._lockScreenOverlay) {
            Main.uiGroup.remove_child(this._lockScreenOverlay);
            this._lockScreenOverlay.destroy();
            this._lockScreenOverlay = null;
        }
        
        // Safely destroy and remove all panel buttons
        // Destruir y quitar todos los botones del panel de forma segura
        for (let button of this._menuButtons) {
            button.destroy();
        }
        this._menuButtons = [];
        
        // Restore the original can_lock descriptor
        // Restaurar el descriptor de can_lock original
        try {
            if (this._origCanLock) {
                Object.defineProperty(Shell.SystemActions.get_default(), 'can_lock', this._origCanLock);
                Shell.SystemActions.get_default().notify('can-lock');
            }
        } catch (e) {
            // ignore
        }

        // Restore original lock and unlock functions
        // Restaurar las funciones de bloqueo y desbloqueo originales
        try {
            if (Main.screenShield) {
                if (this._origLock) {
                    Main.screenShield.lock = this._origLock;
                }
                if (this._origUnlock) {
                    Main.screenShield.unlock = this._origUnlock;
                }
            }
        } catch (e) {
            // ignore
        }

        // Disconnect screenShield active-changed signal
        if (Main.screenShield && this._activeChangedId) {
            Main.screenShield.disconnect(this._activeChangedId);
            this._activeChangedId = 0;
        }

        // Clean up desktop live wallpaper manager
        if (this._desktopLiveWallpaperManager) {
            this._desktopLiveWallpaperManager.destroy();
            this._desktopLiveWallpaperManager = null;
        }

        // Clean up macOS fullscreen spaces manager
        if (this._macOSFullscreenManager) {
            this._macOSFullscreenManager.destroy();
            this._macOSFullscreenManager = null;
        }
        
        // Nullify the virtual keyboard device reference
        // Anular la referencia del dispositivo de teclado virtual
        this._virtualKeyboard = null;

        if (this._prepareForSleepId) {
            try {
                Gio.DBus.system.signal_unsubscribe(this._prepareForSleepId);
            } catch (e) {}
            this._prepareForSleepId = 0;
        }

        if (this._login1SessionLockId) {
            try {
                Gio.DBus.system.signal_unsubscribe(this._login1SessionLockId);
            } catch (e) {}
            this._login1SessionLockId = 0;
        }

        if (this._activePowerDialog) {
            try {
                this._activePowerDialog._cleanup();
                this._activePowerDialog.close();
            } catch (e) {}
            this._activePowerDialog = null;
        }
    }
    
    // --- Logo Menu Button (Pulsar OS logo) ---
    // --- Botón de menú con Logo (logo de Pulsar OS) ---
    _createLogoMenu() {
        this.logoMenuButton = new GlobalMenuButton("");
        this.logoMenuButton.uuid_suffix = "logo";
        
        this.logoMenuButton.label.destroy();
        
        // Load custom Pulsar OS PNG logo icon from the extension directory path
        // Cargar el icono del logo PNG personalizado de Pulsar OS desde la ruta del directorio de la extensión
        let iconFile = Gio.File.new_for_path(this.path + '/pulsar-white-sf.png');
        let fileIcon = new Gio.FileIcon({ file: iconFile });
        this.logoMenuButton.icon = new St.Icon({
            gicon: fileIcon,
            style_class: 'global-menu-logo-icon'
        });
        this.logoMenuButton.add_child(this.logoMenuButton.icon);
        
        // About Pulsar OS
        let aboutItem = new PopupMenu.PopupMenuItem(_t('aboutPulsar'));
        aboutItem.connect('activate', () => {
            let hostName = GLib.get_host_name();
            
            // Memory (RAM)
            let memTotal = "N/A";
            try {
                let [ok, content] = GLib.file_get_contents("/proc/meminfo");
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let match = contentStr.match(/MemTotal:\s+(\d+)\s+kB/);
                    if (match) {
                        let gb = (parseInt(match[1]) / 1024 / 1024).toFixed(1);
                        memTotal = `${gb} GB`;
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to read /proc/meminfo:", e);
            }

            // Processor (CPU)
            let cpuModel = "Unknown CPU";
            try {
                let [ok, content] = GLib.file_get_contents("/proc/cpuinfo");
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let match = contentStr.match(/model name\s+:\s+(.+)/);
                    if (match) {
                        cpuModel = match[1].trim();
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to read /proc/cpuinfo:", e);
            }

            // Graphics (GPU)
            let gpuModel = "Unknown GPU";
            try {
                let [success, stdout, stderr, status] = GLib.spawn_command_line_sync("lspci");
                if (success) {
                    let stdoutStr = new TextDecoder().decode(stdout);
                    let lines = stdoutStr.split("\n");
                    for (let line of lines) {
                        if (line.match(/VGA compatible controller|3D controller|Display controller/i)) {
                            let parts = line.split(": ");
                            if (parts.length > 1) {
                                gpuModel = parts[1].trim();
                            }
                            break;
                        }
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to run lspci:", e);
            }

            // Storage (Disk)
            let diskInfo = "N/A";
            try {
                let [success, stdout, stderr, status] = GLib.spawn_command_line_sync("df -h /");
                if (success) {
                    let stdoutStr = new TextDecoder().decode(stdout);
                    let lines = stdoutStr.split("\n");
                    if (lines.length > 1) {
                        let parts = lines[1].split(/\s+/);
                        if (parts.length > 4) {
                            let size = parts[1];
                            let avail = parts[3];
                            diskInfo = `${size} (${avail} available)`;
                        }
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to run df:", e);
            }

            // OS Name & Version from /etc/os-release
            let osName = "Pulsar OS";
            let osVersion = "";
            let distroBase = "debian";
            try {
                let [ok, content] = GLib.file_get_contents("/etc/os-release");
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let prettyNameMatch = contentStr.match(/^PRETTY_NAME="?([^"\n]+)"?/m);
                    let nameMatch = contentStr.match(/^NAME="?([^"\n]+)"?/m);
                    let idLikeMatch = contentStr.match(/^ID_LIKE="?([^"\n]+)"?/m);
                    let idMatch = contentStr.match(/^ID="?([^"\n]+)"?/m);
                    let verMatch = contentStr.match(/^VERSION="?([^"\n]+)"?/m);
                    let verIdMatch = contentStr.match(/^VERSION_ID="?([^"\n]+)"?/m);

                    if (prettyNameMatch) {
                        osName = prettyNameMatch[1];
                    } else if (nameMatch) {
                        osName = nameMatch[1];
                    }

                    if (/debian/i.test(osName) || (idLikeMatch && /debian/i.test(idLikeMatch[1])) || (idMatch && /debian/i.test(idMatch[1]))) {
                        distroBase = "debian";
                    } else if (/arch/i.test(osName) || (idLikeMatch && /arch/i.test(idLikeMatch[1])) || (idMatch && /arch/i.test(idMatch[1]))) {
                        distroBase = "arch";
                    } else if (idLikeMatch) {
                        distroBase = idLikeMatch[1].split(/\s+/)[0].toLowerCase();
                    }

                    if (verMatch) {
                        osVersion = verMatch[1];
                    } else if (verIdMatch) {
                        osVersion = verIdMatch[1];
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to read /etc/os-release:", e);
            }

            if (!osVersion) {
                osVersion = distroBase === "debian" ? "13 (Debian)" : "rolling (Arch)";
            }
            let dialog = new AboutDialog(osName, osVersion, hostName, cpuModel, memTotal, gpuModel, diskInfo);
            dialog.open();
        });
        this.logoMenuButton.menu.addMenuItem(aboutItem);
        
        // System Settings
        let settingsItem = new PopupMenu.PopupMenuItem(_t('systemSettings'));
        settingsItem.connect('activate', () => {
            this._runCommand("gnome-control-center");
        });
        this.logoMenuButton.menu.addMenuItem(settingsItem);
        
        // App Store
        let appStoreItem = new PopupMenu.PopupMenuItem(_t('appStore'));
        appStoreItem.connect('activate', () => {
            this._openUri("appstream://");
        });
        this.logoMenuButton.menu.addMenuItem(appStoreItem);
        
        this.logoMenuButton.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        // Lock Screen (Solves missing lock option in system menu when not using GDM)
        // Bloquear Pantalla (Soluciona la falta de opción de bloqueo en el menú del sistema al no usar GDM)
        let lockItem = new PopupMenu.PopupMenuItem(_t('lockScreen'));
        lockItem.connect('activate', () => {
            this._lockScreen();
        });
        this.logoMenuButton.menu.addMenuItem(lockItem);
        
        // Log Out
        let logoutItem = new PopupMenu.PopupMenuItem(_t('logOut'));
        logoutItem.connect('activate', () => {
            try {
                let actions = SystemActions?.getDefault ? SystemActions.getDefault() : null;
                if (actions && typeof actions.activateLogout === 'function' && actions.can_logout) {
                    actions.activateLogout();
                } else {
                    this._runCommand("gnome-session-quit --logout --no-prompt || loginctl terminate-session '' || loginctl terminate-session self");
                }
            } catch (e) {
                this._runCommand("gnome-session-quit --logout --no-prompt || loginctl terminate-session '' || loginctl terminate-session self");
            }
        });
        this.logoMenuButton.menu.addMenuItem(logoutItem);
        
        // Sleep
        let sleepItem = new PopupMenu.PopupMenuItem(_t('sleep'));
        sleepItem.connect('activate', () => {
            try {
                let actions = SystemActions?.getDefault ? SystemActions.getDefault() : null;
                if (actions && actions.activateSuspend) {
                    actions.activateSuspend();
                } else {
                    this._runCommand("systemctl suspend");
                }
            } catch (e) {
                this._runCommand("systemctl suspend");
            }
        });
        this.logoMenuButton.menu.addMenuItem(sleepItem);
        
        // Restart
        let restartItem = new PopupMenu.PopupMenuItem(_t('restart'));
        restartItem.connect('activate', () => {
            let dialog = new PowerConfirmDialog('restart', (restore) => {
                this._executePowerAction('restart', restore);
            });
            this._activePowerDialog = dialog;
            dialog.open();
        });
        this.logoMenuButton.menu.addMenuItem(restartItem);
        
        // Shut Down
        let shutdownItem = new PopupMenu.PopupMenuItem(_t('shutDown'));
        shutdownItem.connect('activate', () => {
            let dialog = new PowerConfirmDialog('shutdown', (restore) => {
                this._executePowerAction('shutdown', restore);
            });
            this._activePowerDialog = dialog;
            dialog.open();
        });
        this.logoMenuButton.menu.addMenuItem(shutdownItem);
        
        this._menuButtons.push(this.logoMenuButton);
    }
    
    // --- App Menu Button (Finder / Active App name) ---
    // --- Botón de menú de App (Finder / Nombre de App activa) ---
    _createAppMenu() {
        this.appMenuButton = new GlobalMenuButton("Finder", true);
        this.appMenuButton.uuid_suffix = "app";
        
        // Dynamic items that will adapt their labels to the active window title
        // Elementos dinámicos que adaptarán sus etiquetas al título de la ventana activa
        this.aboutItem = new PopupMenu.PopupMenuItem(_t('aboutApp', 'Finder'));
        this.aboutItem.connect('activate', () => {
            let activeApp = this._getAppName(this._activeAppWindow);
            Main.notify("Pulsar OS", `Active App: ${activeApp}`);
        });
        this.appMenuButton.menu.addMenuItem(this.aboutItem);
        
        this.hideItem = new PopupMenu.PopupMenuItem(_t('hideApp', 'Finder'));
        this.hideItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.minimize();
            }
        });
        this.appMenuButton.menu.addMenuItem(this.hideItem);
        
        this.quitItem = new PopupMenu.PopupMenuItem(_t('quitApp', 'Finder'));
        this.quitItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.delete(global.get_current_time());
            }
        });
        this.appMenuButton.menu.addMenuItem(this.quitItem);
        
        this._menuButtons.push(this.appMenuButton);
    }
    
    // --- File Menu Button ---
    // --- Botón de menú de Archivo ---
    _createFileMenu() {
        let fileBtn = new GlobalMenuButton(_t('file'));
        fileBtn.uuid_suffix = "file";
        
        let newWindowItem = new PopupMenu.PopupMenuItem(_t('newWindow'));
        newWindowItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                let tracker = Shell.WindowTracker.get_default();
                let app = tracker.get_window_app(window);
                if (app) {
                    app.open_new_window(-1);
                    return;
                }
            }
            // If no application has focus (Finder state), launch the default file manager (Home)
            // Si ninguna aplicación tiene el foco (estado Finder), abrir gestor de archivos predeterminado (Home)
            this._openUri("file://" + GLib.get_home_dir());
        });
        fileBtn.menu.addMenuItem(newWindowItem);
        
        let closeItem = new PopupMenu.PopupMenuItem(_t('closeWindow'));
        closeItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.delete(global.get_current_time());
            }
        });
        fileBtn.menu.addMenuItem(closeItem);
        
        this._menuButtons.push(fileBtn);
    }
    
    // --- Edit Menu Button ---
    // --- Botón de menú de Edición ---
    _createEditMenu() {
        let editBtn = new GlobalMenuButton(_t('edit'));
        editBtn.uuid_suffix = "edit";
        
        let undoItem = new PopupMenu.PopupMenuItem(_t('undo'));
        undoItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_z);
        });
        editBtn.menu.addMenuItem(undoItem);
        
        let redoItem = new PopupMenu.PopupMenuItem(_t('redo'));
        redoItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_y);
        });
        editBtn.menu.addMenuItem(redoItem);
        
        editBtn.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        let cutItem = new PopupMenu.PopupMenuItem(_t('cut'));
        cutItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_x);
        });
        editBtn.menu.addMenuItem(cutItem);
        
        let copyItem = new PopupMenu.PopupMenuItem(_t('copy'));
        copyItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_c);
        });
        editBtn.menu.addMenuItem(copyItem);
        
        let pasteItem = new PopupMenu.PopupMenuItem(_t('paste'));
        pasteItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_v);
        });
        editBtn.menu.addMenuItem(pasteItem);
        
        this._menuButtons.push(editBtn);
    }
    
    // --- Go Menu Button ---
    // --- Botón de menú de Ir ---
    _createGoMenu() {
        let goBtn = new GlobalMenuButton(_t('go'));
        goBtn.uuid_suffix = "go";
        
        let backItem = new PopupMenu.PopupMenuItem(_t('back'));
        backItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Alt_L], Clutter.KEY_Left);
        });
        goBtn.menu.addMenuItem(backItem);
        
        goBtn.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        let homeItem = new PopupMenu.PopupMenuItem(_t('home'));
        homeItem.connect('activate', () => {
            this._openUri("file://" + GLib.get_home_dir());
        });
        goBtn.menu.addMenuItem(homeItem);
        
        let docsItem = new PopupMenu.PopupMenuItem(_t('documents'));
        docsItem.connect('activate', () => {
            let docsPath = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOCUMENTS) || (GLib.get_home_dir() + "/Documents");
            this._openUri("file://" + docsPath);
        });
        goBtn.menu.addMenuItem(docsItem);
        
        let downloadsItem = new PopupMenu.PopupMenuItem(_t('downloads'));
        downloadsItem.connect('activate', () => {
            let dlPath = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOWNLOAD) || (GLib.get_home_dir() + "/Downloads");
            this._openUri("file://" + dlPath);
        });
        goBtn.menu.addMenuItem(downloadsItem);
        
        let picsItem = new PopupMenu.PopupMenuItem(_t('pictures'));
        picsItem.connect('activate', () => {
            let picsPath = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_PICTURES) || (GLib.get_home_dir() + "/Pictures");
            this._openUri("file://" + picsPath);
        });
        goBtn.menu.addMenuItem(picsItem);
        
        this._menuButtons.push(goBtn);
    }
    
    // --- Window Menu Button ---
    // --- Botón de menú de Ventana ---
    _createWindowMenu() {
        let winBtn = new GlobalMenuButton(_t('window'));
        winBtn.uuid_suffix = "window";
        
        let minimizeItem = new PopupMenu.PopupMenuItem(_t('minimize'));
        minimizeItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.minimize();
            }
        });
        winBtn.menu.addMenuItem(minimizeItem);
        
        let maximizeItem = new PopupMenu.PopupMenuItem(_t('maximize'));
        maximizeItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                if (window.get_maximized()) {
                    window.unmaximize(Meta.MaximizeFlags.BOTH);
                } else {
                    window.maximize(Meta.MaximizeFlags.BOTH);
                }
            }
        });
        winBtn.menu.addMenuItem(maximizeItem);
        
        let closeItem = new PopupMenu.PopupMenuItem(_t('close'));
        closeItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.delete(global.get_current_time());
            }
        });
        winBtn.menu.addMenuItem(closeItem);
        
        this._menuButtons.push(winBtn);
    }
    
    // --- Help Menu Button ---
    // --- Botón de menú de Ayuda ---
    _createHelpMenu() {
        let helpBtn = new GlobalMenuButton(_t('help'));
        helpBtn.uuid_suffix = "help";
        
        // English: Link to Pulsar OS Wiki
        // Español: Enlace a la Wiki de Pulsar OS
        let wikiItem = new PopupMenu.PopupMenuItem("Pulsar OS Wiki");
        wikiItem.connect('activate', () => {
            this._openUri("https://github.com/Inled-Pulsar-OS/DOCS/wiki");
        });
        helpBtn.menu.addMenuItem(wikiItem);

        // English: Link to Pulsar OS Website
        // Español: Enlace al sitio web de Pulsar OS
        let pulsarWebItem = new PopupMenu.PopupMenuItem("Pulsar OS Website");
        pulsarWebItem.connect('activate', () => {
            this._openUri("https://os.inled.es");
        });
        helpBtn.menu.addMenuItem(pulsarWebItem);

        // English: Link to Pulsar OS Discord community
        // Español: Enlace al canal de Discord de Pulsar OS
        let discordItem = new PopupMenu.PopupMenuItem("Pulsar OS Discord");
        discordItem.connect('activate', () => {
            this._openUri("https://link.inled.es/discord");
        });
        helpBtn.menu.addMenuItem(discordItem);

        // English: Link to Pulsar OS Matrix community
        // Español: Enlace al canal de Matrix de Pulsar OS
        let matrixItem = new PopupMenu.PopupMenuItem("Pulsar OS Matrix");
        matrixItem.connect('activate', () => {
            this._openUri("https://matrix.inled.es");
        });
        helpBtn.menu.addMenuItem(matrixItem);

        // English: Separator between Pulsar OS and base system/partner links
        // Español: Separador entre los enlaces de Pulsar OS y los enlaces del sistema base/socios
        helpBtn.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        // English: Link to Inled company website
        // Español: Enlace al sitio web de la empresa Inled
        let inledItem = new PopupMenu.PopupMenuItem("Inled Website");
        inledItem.connect('activate', () => {
            this._openUri("https://inled.es");
        });
        helpBtn.menu.addMenuItem(inledItem);
        
        // English: Link to Debian base support documentation
        // Español: Enlace a la documentación de soporte base de Debian
        let debianHelpItem = new PopupMenu.PopupMenuItem("Debian Help");
        debianHelpItem.connect('activate', () => {
            this._openUri("https://www.debian.org/support");
        });
        helpBtn.menu.addMenuItem(debianHelpItem);
        
        this._menuButtons.push(helpBtn);
    }
    
    // --- Focus Event Handler ---
    // --- Manejador del Evento de Foco ---
    _onFocusWindowChanged() {
        let window = global.display.focus_window;
        
        // Check if the previously tracked window still exists
        // Comprobar si la ventana anteriormente rastreada todavía existe
        if (this._activeAppWindow) {
            let activeWindows = global.get_window_actors().map(a => a.meta_window || a.get_meta_window()).filter(Boolean);
            if (!activeWindows.includes(this._activeAppWindow)) {
                this._activeAppWindow = null;
            }
        }
        
        if (window) {
            // Check if this is a system/shell window that we want to ignore (like GNOME Shell panel or menu popups)
            // Comprobar si es una ventana del sistema o shell que queremos ignorar (como el panel o ventanas emergentes)
            let wmClass = window.get_wm_class();
            let isShell = wmClass && (wmClass.toLowerCase().includes('gnome-shell') || wmClass.toLowerCase().includes('gdm'));
            
            // In GNOME Shell, some utility windows or menus might also have no app or be system components
            if (!isShell) {
                this._activeAppWindow = window;
            }
        }
        
        let appName = this._getAppName(this._activeAppWindow);
        
        // Update panel text representation for the focused application
        // Actualizar la representación de texto en el panel para la aplicación enfocada
        this.appMenuButton.setText(appName);
        
        // Dynamically update the app menu items
        // Actualizar dinámicamente los elementos del menú de la aplicación
        this.aboutItem.label.set_text(_t('aboutApp', appName));
        this.hideItem.label.set_text(_t('hideApp', appName));
        this.quitItem.label.set_text(_t('quitApp', appName));
        
        // Deactivate Hide/Quit if no app is open (Finder mode)
        // Desactivar Ocultar/Salir si no hay app abierta (modo Finder)
        if (appName === "Finder") {
            this.hideItem.setSensitive(false);
            this.quitItem.setSensitive(false);
        } else {
            this.hideItem.setSensitive(true);
            this.quitItem.setSensitive(true);
        }
    }
    
    // --- Helper to open URI links ---
    // --- Utilidad para abrir enlaces URI ---
    _openUri(uri) {
        try {
            Gio.AppInfo.launch_default_for_uri(uri, null);
        } catch (e) {
            console.error(`[GlobalMenu] Failed to open URI: ${uri}`, e);
        }
    }
    
    // --- Helper to lock screen ---
    // --- Utilidad para bloquear la pantalla ---
    _lockScreen() {
        try {
            if (this._lockScreenOverlay) {
                this._lockScreenOverlay.lock();
            } else if (Main.screenShield) {
                Main.screenShield.lock(true);
            } else {
                GLib.spawn_command_line_async("loginctl lock-session");
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to lock screen:", e);
            GLib.spawn_command_line_async("loginctl lock-session");
        }
    }
    
    // --- Helper to execute power action (with optional session restore / hibernation) ---
    // --- Utilidad para ejecutar acción de energía (con restauración opcional de sesión / hibernación) ---
    _executePowerAction(actionType, restore) {
        let mode = restore ? "restore" : "normal";
        let cmd = `pkexec /usr/bin/pulsaros-power-action ${actionType} ${mode}`;
        this._runCommand(cmd);
    }

    // --- Helper to run command ---
    // --- Utilidad para ejecutar comandos ---
    _runCommand(cmd) {
        try {
            GLib.spawn_command_line_async(cmd);
        } catch (e) {
            console.error(`[GlobalMenu] Failed to run command: ${cmd}`, e);
        }
    }
    
    // --- Helper to get active application name ---
    // --- Utilidad para obtener el nombre de la app activa ---
    _getAppName(window) {
        if (!window) {
            return "Finder";
        }
        
        // Attempt to trace through window tracker app registry
        // Intentar rastrear mediante el registro de apps del WindowTracker
        let tracker = Shell.WindowTracker.get_default();
        let app = tracker.get_window_app(window);
        if (app) {
            let name = app.get_name();
            if (name) {
                return name;
            }
        }
        
        // Fallback to window WM_CLASS name (typically app-id/process type)
        // Alternativa al nombre WM_CLASS de la ventana
        let wmClass = window.get_wm_class();
        if (wmClass) {
            return wmClass.charAt(0).toUpperCase() + wmClass.slice(1);
        }
        
        // Ultimate fallback to window title
        // Última alternativa al título de la ventana
        let title = window.get_title();
        if (title) {
            return title;
        }
        
        return "Finder";
    }
    
    // --- Helper to simulate virtual keypress events ---
    // --- Utilidad para simular eventos de pulsación de teclas virtuales ---
    _sendKeyStroke(modifiers, targetKey) {
        if (!this._virtualKeyboard) {
            console.warn("[GlobalMenu] Virtual keyboard device not available");
            return;
        }
        
        let time = global.get_current_time();
        
        // 1. Press all modifiers (e.g. CTRL, ALT, SHIFT)
        // 1. Presionar todos los modificadores (p. ej., CTRL, ALT, SHIFT)
        for (let key of modifiers) {
            this._virtualKeyboard.notify_keyval(time, key, Clutter.KeyState.PRESSED);
        }
        
        // 2. Press target key
        // 2. Presionar tecla objetivo
        this._virtualKeyboard.notify_keyval(time, targetKey, Clutter.KeyState.PRESSED);
        
        // 3. Release target key
        // 3. Soltar tecla objetivo
        this._virtualKeyboard.notify_keyval(time, targetKey, Clutter.KeyState.RELEASED);
        
        // 4. Release modifiers in reverse order
        // 4. Soltar modificadores en orden inverso
        for (let i = modifiers.length - 1; i >= 0; i--) {
            this._virtualKeyboard.notify_keyval(time, modifiers[i], Clutter.KeyState.RELEASED);
        }
    }
}
