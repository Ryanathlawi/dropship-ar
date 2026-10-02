//! لغة الواجهة: العربية أو الإنجليزية
//!
//! تُعرف وحدها من الجهاز، ولصاحبه أن يختار غيرها من الخيارات، والنصوص
//! مكتوبة باللغتين في مكانها بدل ملفات مفاتيح، فمن يقرأ الكود يرى الجملتين معًا

use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;

static ARABIC: AtomicBool = AtomicBool::new(true);

pub fn ar() -> bool {
    ARABIC.load(Ordering::Relaxed)
}

pub fn set(arabic: bool) {
    ARABIC.store(arabic, Ordering::Relaxed);
}

/// الجملة بلغة الواجهة
pub fn tr(ar: &'static str, en: &'static str) -> &'static str {
    if self::ar() { ar } else { en }
}

/// قالب تنسيق بلغة الواجهة، فـ `format!` لا يقبل إلا نصًّا ثابتًا
#[macro_export]
macro_rules! trf {
    ($ar:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::lang::ar() {
            format!($ar $(, $arg)*)
        } else {
            format!($en $(, $arg)*)
        }
    };
}

/// اتجاه القراءة: من اليمين بالعربية ومن اليسار بالإنجليزية
pub fn fwd(align: egui::Align) -> egui::Layout {
    if ar() {
        egui::Layout::right_to_left(align)
    } else {
        egui::Layout::left_to_right(align)
    }
}

/// عكس اتجاه القراءة، لما يُدفع إلى الطرف الآخر من الصف
pub fn back(align: egui::Align) -> egui::Layout {
    if ar() {
        egui::Layout::left_to_right(align)
    } else {
        egui::Layout::right_to_left(align)
    }
}

/// بداية السطر، لمحاذاة عمود من النصوص إليها
pub fn start() -> egui::Align {
    if ar() { egui::Align::RIGHT } else { egui::Align::LEFT }
}

/// ما اختاره صاحب الجهاز في الخيارات
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Choice {
    #[default]
    Auto,
    Arabic,
    English,
}

impl Choice {
    pub fn arabic(self) -> bool {
        match self {
            Choice::Auto => detect(),
            Choice::Arabic => true,
            Choice::English => false,
        }
    }
}

/// اللغة الأساسية في رقم اللغة الذي يعطيه ويندوز، والعربية رقمها 1
const LANG_ARABIC: u16 = 0x01;

fn is_arabic(langid: u16) -> bool {
    langid & 0x3ff == LANG_ARABIC
}

/// عربي إن كان ويندوز بالعربي أو فيه لوحة مفاتيح عربية
///
/// كثير من اللاعبين العرب ويندوزهم إنجليزي، فلغة ويندوز وحدها كانت ستفتح لهم
/// البرنامج بالإنجليزي، أما لوحة المفاتيح العربية فلا يكاد يخلو منها جهاز عربي
/// ولا يكاد يضعها غيره
#[cfg(target_os = "windows")]
pub fn detect() -> bool {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayoutList;

    if is_arabic(unsafe { GetUserDefaultUILanguage() }) {
        return true;
    }
    let mut layouts = [Default::default(); 32];
    let n = unsafe { GetKeyboardLayoutList(Some(&mut layouts)) }.max(0) as usize;
    layouts[..n.min(layouts.len())]
        .iter()
        .any(|hkl| is_arabic((hkl.0 as usize & 0xffff) as u16))
}

#[cfg(not(target_os = "windows"))]
pub fn detect() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_is_known_by_its_primary_language_whatever_the_country() {
        // السعودية، مصر، الإمارات، المغرب
        for id in [0x0401, 0x0c01, 0x3801, 0x1801] {
            assert!(is_arabic(id), "{id:#06x}");
        }
        // الإنجليزية الأمريكية والبريطانية والفرنسية والأردية
        for id in [0x0409, 0x0809, 0x040c, 0x0420] {
            assert!(!is_arabic(id), "{id:#06x}");
        }
    }

    #[test]
    fn a_sentence_comes_in_the_chosen_language() {
        set(true);
        assert_eq!(tr("عربي", "English"), "عربي");
        assert_eq!(trf!("عندك {} سيرفر", "{} servers", 3), "عندك 3 سيرفر");
        set(false);
        assert_eq!(tr("عربي", "English"), "English");
        assert_eq!(trf!("عندك {} سيرفر", "{} servers", 3), "3 servers");
        set(true);
    }
}
