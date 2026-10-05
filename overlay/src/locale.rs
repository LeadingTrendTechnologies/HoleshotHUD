//! Windows UI language for Settings → Look → Language = System.

use mxbo_hud::config::Language;
use windows::Win32::Globalization::GetUserDefaultLocaleName;

pub fn system_language() -> Language {
    let mut buf = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(&mut buf) };
    if len <= 1 {
        return Language::En;
    }
    let end = (len as usize).saturating_sub(1).min(buf.len());
    let tag = String::from_utf16_lossy(&buf[..end]);
    Language::from_locale(&tag)
}

pub fn apply(stored: Language) {
    mxbo_hud::i18n::set_language(stored.resolved(system_language()));
}
