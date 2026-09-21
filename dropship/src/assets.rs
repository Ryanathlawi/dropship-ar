#![allow(dead_code)]

use eframe::egui::{self, ImageSource};

pub const COMPANY_ICON_BATTLENET: ImageSource =
    egui::include_image!("../assets/icons/company-icon-battlenet.svg");
pub const COMPANY_ICON_STEAM: ImageSource =
    egui::include_image!("../assets/icons/company-icon-steam.svg");

pub const GAME_ICON_OVERWATCH: ImageSource =
    egui::include_image!("../assets/icons/game-icon-overwatch.png");

pub const ICON_MAPLE_LEAF: ImageSource =
    egui::include_image!("../assets/icons/icon-maple-leaf.svg");
// pub const ICON_FIRE_WALL: ImageSource = egui::include_image!("../assets/icons/icon-fire-wall.svg");
pub const ICON_POWER_OFF: ImageSource = egui::include_image!("../assets/icons/icon-power-off.svg");
// pub const ICON_PLUS: ImageSource = egui::include_image!("../assets/icons/icon-plus.svg");
pub const ICON_TERMINAL: ImageSource = egui::include_image!("../assets/icons/icon-terminal.svg");
// pub const ICON_CIRCLE_INFO: ImageSource =
//     egui::include_image!("../assets/icons/icon-circle-info.svg");
pub const ICON_STAR: ImageSource = egui::include_image!("../assets/icons/icon-star.svg");
pub const ICON_BAN: ImageSource = egui::include_image!("../assets/icons/icon-ban.svg");
pub const ICON_GEARS: ImageSource = egui::include_image!("../assets/icons/icon-gears.svg");
pub const ICON_M1: ImageSource = egui::include_image!("../assets/icons/icon-m1.svg");
pub const ICON_M2: ImageSource = egui::include_image!("../assets/icons/icon-m2.svg");
pub const ANGLES_RIGHT: ImageSource = egui::include_image!("../assets/icons/icon-angles-right.svg");

pub const ICON_HEART: ImageSource = egui::include_image!("../assets/icons/icon-heart.png");

// أيقونات الواجهة الجديدة (lucide، رخصة ISC)
pub const ICON_MAP: ImageSource = egui::include_image!("../assets/icons/lucide-map.svg");
pub const ICON_GAMEPAD: ImageSource = egui::include_image!("../assets/icons/lucide-gamepad-2.svg");
pub const ICON_NEWS: ImageSource = egui::include_image!("../assets/icons/lucide-newspaper.svg");
pub const ICON_GLOBE: ImageSource = egui::include_image!("../assets/icons/lucide-globe.svg");
pub const ICON_SUN: ImageSource = egui::include_image!("../assets/icons/lucide-sun.svg");
pub const ICON_MOON: ImageSource = egui::include_image!("../assets/icons/lucide-moon.svg");
pub const ICON_SORT: ImageSource = egui::include_image!("../assets/icons/lucide-arrow-down-up.svg");
pub const ICON_CROSSHAIR: ImageSource = egui::include_image!("../assets/icons/lucide-crosshair.svg");
pub const ICON_SHIELD: ImageSource = egui::include_image!("../assets/icons/lucide-shield-check.svg");

/// علم الدولة (رمز ISO بحرفين صغيرين) من flag-icons (رخصة MIT). `None` لدولة غير مضمّنة.
pub fn flag(iso: &str) -> Option<ImageSource<'static>> {
    Some(match iso {
        "nl" => egui::include_image!("../assets/flags/nl.svg"),
        "br" => egui::include_image!("../assets/flags/br.svg"),
        "fi" => egui::include_image!("../assets/flags/fi.svg"),
        "sa" => egui::include_image!("../assets/flags/sa.svg"),
        "sg" => egui::include_image!("../assets/flags/sg.svg"),
        "jp" => egui::include_image!("../assets/flags/jp.svg"),
        "us" => egui::include_image!("../assets/flags/us.svg"),
        "au" => egui::include_image!("../assets/flags/au.svg"),
        "tw" => egui::include_image!("../assets/flags/tw.svg"),
        "kr" => egui::include_image!("../assets/flags/kr.svg"),
        "de" => egui::include_image!("../assets/flags/de.svg"),
        "gb" => egui::include_image!("../assets/flags/gb.svg"),
        "fr" => egui::include_image!("../assets/flags/fr.svg"),
        "ca" => egui::include_image!("../assets/flags/ca.svg"),
        "mx" => egui::include_image!("../assets/flags/mx.svg"),
        "ar" => egui::include_image!("../assets/flags/ar.svg"),
        "cl" => egui::include_image!("../assets/flags/cl.svg"),
        "in" => egui::include_image!("../assets/flags/in.svg"),
        "za" => egui::include_image!("../assets/flags/za.svg"),
        "hk" => egui::include_image!("../assets/flags/hk.svg"),
        "bh" => egui::include_image!("../assets/flags/bh.svg"),
        "ae" => egui::include_image!("../assets/flags/ae.svg"),
        "kw" => egui::include_image!("../assets/flags/kw.svg"),
        "qa" => egui::include_image!("../assets/flags/qa.svg"),
        "eg" => egui::include_image!("../assets/flags/eg.svg"),
        "tr" => egui::include_image!("../assets/flags/tr.svg"),
        _ => return None,
    })
}
