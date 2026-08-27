use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

use crate::ootb::{self, OotbConfig};

pub struct OotbState {
    pub running: Mutex<bool>,
}

#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    progress: f64,
    status: String,
}

#[derive(Clone, serde::Serialize)]
struct LogPayload {
    message: String,
}

#[derive(Clone, serde::Serialize)]
pub struct OotbData {
    pub countries: Vec<String>,
    pub languages: Vec<String>,
    pub keymaps: Vec<String>,
    pub timezones: Vec<String>,
    pub avatars: Vec<String>,
}

#[tauri::command]
pub fn get_ootb_data() -> Result<OotbData, String> {
    let mut countries: Vec<String> = vec![
        "Argentina", "Australia", "Austria", "Belgium", "Brazil",
        "Canada", "Chile", "China", "Colombia", "Czech Republic",
        "Denmark", "Finland", "France", "Germany", "Greece",
        "Hungary", "India", "Indonesia", "Ireland", "Israel",
        "Italy", "Japan", "Malaysia", "Mexico", "Netherlands",
        "New Zealand", "Norway", "Pakistan", "Peru", "Philippines",
        "Poland", "Portugal", "Romania", "Russia", "Saudi Arabia",
        "Singapore", "South Africa", "South Korea", "Spain", "Sweden",
        "Switzerland", "Taiwan", "Thailand", "Turkey", "Ukraine",
        "United Arab Emirates", "United Kingdom", "United States", "Vietnam",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    countries.sort();

    let languages: Vec<String> = vec![
        "ar_SA.UTF-8", "bg_BG.UTF-8", "ca_ES.UTF-8", "cs_CZ.UTF-8",
        "da_DK.UTF-8", "de_DE.UTF-8", "el_GR.UTF-8", "en_GB.UTF-8",
        "en_US.UTF-8", "es_ES.UTF-8", "es_MX.UTF-8", "fi_FI.UTF-8",
        "fr_FR.UTF-8", "fr_CA.UTF-8", "he_IL.UTF-8", "hi_IN.UTF-8",
        "hr_HR.UTF-8", "hu_HU.UTF-8", "id_ID.UTF-8", "it_IT.UTF-8",
        "ja_JP.UTF-8", "ko_KR.UTF-8", "ms_MY.UTF-8", "nl_NL.UTF-8",
        "no_NO.UTF-8", "pl_PL.UTF-8", "pt_BR.UTF-8", "pt_PT.UTF-8",
        "ro_RO.UTF-8", "ru_RU.UTF-8", "sk_SK.UTF-8", "sl_SI.UTF-8",
        "sr_RS.UTF-8", "sv_SE.UTF-8", "th_TH.UTF-8", "tr_TR.UTF-8",
        "uk_UA.UTF-8", "vi_VN.UTF-8", "zh_CN.UTF-8", "zh_TW.UTF-8",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let keymaps: Vec<String> = vec![
        "al", "am", "ara", "at", "au", "az", "ba", "bd", "be", "bg",
        "br", "by", "ca", "cd", "ch", "cn", "cz", "de", "dk", "ee",
        "es", "et", "fi", "fr", "gb", "ge", "gh", "gr", "hr", "hu",
        "ie", "il", "in", "ir", "is", "it", "jp", "ke", "kg", "kh",
        "kr", "kz", "la", "lt", "lv", "ma", "md", "me", "mk", "ml",
        "mm", "mn", "mt", "mv", "ng", "nl", "no", "np", "ph", "pk",
        "pl", "pt", "ro", "rs", "ru", "sa", "se", "sg", "si", "sk",
        "sn", "sr", "sy", "tg", "th", "tj", "tm", "tr", "tw", "tz",
        "ua", "us", "uz", "vn", "za",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let timezones: Vec<String> = vec![
        "Africa/Abidjan", "Africa/Accra", "Africa/Algiers", "Africa/Cairo",
        "Africa/Casablanca", "Africa/Johannesburg", "Africa/Lagos",
        "Africa/Nairobi", "America/Anchorage", "America/Argentina/Buenos_Aires",
        "America/Bogota", "America/Caracas", "America/Chicago",
        "America/Denver", "America/Havana", "America/Lima",
        "America/Los_Angeles", "America/Mexico_City", "America/New_York",
        "America/Phoenix", "America/Sao_Paulo", "America/Toronto",
        "America/Vancouver", "Asia/Almaty", "Asia/Beirut", "Asia/Dubai",
        "Asia/Ho_Chi_Minh", "Asia/Hong_Kong", "Asia/Irkutsk",
        "Asia/Kathmandu", "Asia/Kolkata", "Asia/Kuala_Lumpur",
        "Asia/Manila", "Asia/Novosibirsk", "Asia/Riyadh", "Asia/Seoul",
        "Asia/Shanghai", "Asia/Singapore", "Asia/Taipei", "Asia/Tehran",
        "Asia/Tokyo", "Asia/Vladivostok", "Atlantic/Reykjavik",
        "Australia/Adelaide", "Australia/Brisbane", "Australia/Melbourne",
        "Australia/Perth", "Australia/Sydney", "Europe/Amsterdam",
        "Europe/Athens", "Europe/Berlin", "Europe/Bratislava",
        "Europe/Brussels", "Europe/Bucharest", "Europe/Budapest",
        "Europe/Chisinau", "Europe/Copenhagen", "Europe/Dublin",
        "Europe/Helsinki", "Europe/Istanbul", "Europe/Kiev",
        "Europe/Lisbon", "Europe/Ljubljana", "Europe/London",
        "Europe/Madrid", "Europe/Minsk", "Europe/Moscow",
        "Europe/Oslo", "Europe/Paris", "Europe/Prague",
        "Europe/Riga", "Europe/Rome", "Europe/Sarajevo",
        "Europe/Skopje", "Europe/Sofia", "Europe/Stockholm",
        "Europe/Tallinn", "Europe/Tirane", "Europe/Vienna",
        "Europe/Vilnius", "Europe/Warsaw", "Europe/Zagreb",
        "Europe/Zurich", "Pacific/Auckland", "Pacific/Fiji",
        "Pacific/Guam", "Pacific/Honolulu", "Pacific/Port_Moresby",
        "Pacific/Samoa", "UTC",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let avatars: Vec<String> = vec![
        "/usr/share/pulsaros-welcome/avatars/avatar1.png".into(),
        "/usr/share/pulsaros-welcome/avatars/avatar2.png".into(),
        "/usr/share/pulsaros-welcome/avatars/avatar3.png".into(),
        "/usr/share/pulsaros-welcome/avatars/avatar4.png".into(),
        "/usr/share/pulsaros-welcome/avatars/avatar5.png".into(),
        "/usr/share/pulsaros-welcome/avatars/avatar6.png".into(),
    ];

    Ok(OotbData {
        countries,
        languages,
        keymaps,
        timezones,
        avatars,
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn start_ootb_setup(
    app: AppHandle,
    fullname: String,
    username: String,
    password: String,
    language: String,
    keymap: String,
    timezone: String,
    avatar_path: Option<String>,
    state: State<'_, OotbState>,
) -> Result<(), String> {
    {
        let mut running = state.running.lock().map_err(|e| e.to_string())?;
        if *running {
            return Err("Setup already in progress".into());
        }
        *running = true;
    }

    let state_clone = state.inner();
    let app_clone = app.clone();

    let config = OotbConfig {
        fullname,
        username,
        password,
        language,
        keymap,
        timezone,
        avatar_path,
    };

    let result = tokio::task::spawn_blocking(move || {
        let log_fn = {
            let app = app_clone.clone();
            move |msg: String| {
                let _ = app.emit("ootb-log", LogPayload { message: msg });
            }
        };

        let progress_fn = {
            let app = app_clone.clone();
            move |pct: f64, msg: String| {
                let _ = app.emit(
                    "ootb-progress",
                    ProgressPayload {
                        progress: pct,
                        status: msg,
                    },
                );
            }
        };

        ootb::run_setup(&config, &log_fn, &progress_fn)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let mut running = state_clone.running.lock().map_err(|e| e.to_string())?;
    *running = false;

    result
}

#[tauri::command]
pub fn ootb_final_cleanup(username: String) -> Result<(), String> {
    let log_fn = |msg: String| {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/pulsar-ootb.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{}", msg)
            });
    };

    ootb::run_final_cleanup(&username, &log_fn)
}
