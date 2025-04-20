use once_cell::sync::Lazy;
use regex::Regex;

use crate::manifest::{placeholder, Os};

pub fn normalize(path: &str, os: Option<Os>) -> String {
    let mut path = path.trim().trim_end_matches(['/', '\\']).replace('\\', "/");

    if path == "~" || path.starts_with("~/") {
        path = path.replacen('~', placeholder::HOME, 1);
    }

    static CONSECUTIVE_SLASHES: Lazy<Regex> = Lazy::new(|| Regex::new(r"/{2,}").unwrap());
    static UNNECESSARY_DOUBLE_STAR_1: Lazy<Regex> = Lazy::new(|| Regex::new(r"([^/*])\*{2,}").unwrap());
    static UNNECESSARY_DOUBLE_STAR_2: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*{2,}([^/*])").unwrap());
    static ENDING_WILDCARD: Lazy<Regex> = Lazy::new(|| Regex::new(r"(/\*)+$").unwrap());
    static ENDING_DOT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(/\.)$").unwrap());
    static INTERMEDIATE_DOT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(/\./)").unwrap());
    static BLANK_SEGMENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(/\s+/)").unwrap());
    static APP_DATA: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)%appdata%").unwrap());
    static APP_DATA_ROAMING: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)%userprofile%/AppData/Roaming").unwrap());
    static APP_DATA_LOCAL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)%localappdata%").unwrap());
    static APP_DATA_LOCAL_2: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)%userprofile%/AppData/Local/").unwrap());
    static USER_PROFILE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)%userprofile%").unwrap());
    static DOCUMENTS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)%userprofile%/Documents").unwrap());

    for (pattern, replacement) in [
        (&CONSECUTIVE_SLASHES, "/"),
        (&UNNECESSARY_DOUBLE_STAR_1, "${1}*"),
        (&UNNECESSARY_DOUBLE_STAR_2, "*${1}"),
        (&ENDING_WILDCARD, ""),
        (&ENDING_DOT, ""),
        (&INTERMEDIATE_DOT, "/"),
        (&BLANK_SEGMENT, "/"),
        (&APP_DATA, placeholder::WIN_APP_DATA),
        (&APP_DATA_ROAMING, placeholder::WIN_APP_DATA),
        (&APP_DATA_LOCAL, placeholder::WIN_LOCAL_APP_DATA),
        (&APP_DATA_LOCAL_2, &format!("{}/", placeholder::WIN_LOCAL_APP_DATA)),
        (&USER_PROFILE, placeholder::HOME),
        (&DOCUMENTS, placeholder::WIN_DOCUMENTS),
    ] {
        path = pattern.replace_all(&path, replacement).to_string();
    }

    if os == Some(Os::Windows) {
        static DOCUMENTS_2: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<home>/Documents").unwrap());

        #[allow(clippy::single_element_loop)]
        for (pattern, replacement) in [(&DOCUMENTS_2, placeholder::WIN_DOCUMENTS)] {
            path = pattern.replace_all(&path, replacement).to_string();
        }
    }

    for (pattern, replacement) in [
        ("{64BitSteamID}", placeholder::STORE_USER_ID),
        ("{Steam3AccountID}", placeholder::STORE_USER_ID),
    ] {
        path = path.replace(pattern, replacement);
    }

    path
}

fn too_broad(path: &str) -> bool {
    use placeholder::{BASE, HOME, ROOT, STORE_USER_ID, WIN_APP_DATA, WIN_DIR, WIN_DOCUMENTS, XDG_CONFIG, XDG_DATA};

    let path_lower = path.to_lowercase();

    for item in placeholder::ALL {
        if path == *item {
            return true;
        }
    }

    for item in placeholder::AVOID_WILDCARDS {
        if path.starts_with(&format!("{item}/*")) || path.starts_with(&format!("{item}/{STORE_USER_ID}")) {
            return true;
        }
    }

    // These paths are present whether or not the game is installed.
    // If possible, they should be narrowed down on the wiki.
    for item in [
        format!("{BASE}/{STORE_USER_ID}"), // because `<storeUserId>` is handled as `*`
        format!("{HOME}/Documents"),
        format!("{HOME}/Saved Games"),
        format!("{HOME}/AppData"),
        format!("{HOME}/AppData/Local"),
        format!("{HOME}/AppData/Local/Packages"),
        format!("{HOME}/AppData/LocalLow"),
        format!("{HOME}/AppData/Roaming"),
        format!("{HOME}/Documents/My Games"),
        format!("{HOME}/Library/Application Support"),
        format!("{HOME}/Library/Application Support/UserData"),
        format!("{HOME}/Library/Preferences"),
        format!("{HOME}/.renpy"),
        format!("{HOME}/.renpy/persistent"),
        format!("{HOME}/Library"),
        format!("{HOME}/Library/RenPy"),
        format!("{HOME}/Telltale Games"),
        format!("{ROOT}/config"),
        format!("{WIN_APP_DATA}/MMFApplications"),
        format!("{WIN_APP_DATA}/RenPy"),
        format!("{WIN_APP_DATA}/RenPy/persistent"),
        format!("{WIN_DIR}/win.ini"),
        format!("{WIN_DIR}/SysWOW64"),
        format!("{WIN_DOCUMENTS}/My Games"),
        format!("{WIN_DOCUMENTS}/Telltale Games"),
        format!("{XDG_CONFIG}/unity3d"),
        format!("{XDG_DATA}/unity3d"),
        "C:/Program Files".to_string(),
        "C:/Program Files (x86)".to_string(),
    ] {
        let item = item.to_lowercase();
        if path_lower == item
            || path_lower.starts_with(&format!("{item}/*"))
            || path_lower.starts_with(&format!("{item}/{}", STORE_USER_ID.to_lowercase()))
            || path_lower.starts_with(&format!("{item}/savesdir"))
        {
            return true;
        }
    }

    // Drive letters:
    static DRIVES: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z]:$").unwrap());
    if DRIVES.is_match(path) {
        return true;
    }

    // Colon not for a drive letter
    if path.get(2..).is_some_and(|path| path.contains(':')) {
        return true;
    }

    // Root:
    if path == "/" {
        return true;
    }

    // Relative path wildcard:
    if path.starts_with('*') {
        return true;
    }

    false
}

pub fn usable(path: &str) -> bool {
    static UNPRINTABLE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\p{Cc}|\p{Cf})").unwrap());

    !path.is_empty()
        && !path.contains("{{")
        && !path.starts_with("./")
        && !path.starts_with("../")
        && !too_broad(path)
        && !UNPRINTABLE.is_match(path)
}
