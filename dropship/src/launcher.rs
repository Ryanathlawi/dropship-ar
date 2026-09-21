//! الواجهة الجديدة للنسخة العربية: «اللانشر».
//! خريطة عالم منقّطة تملأ النافذة، مسارات تتدفق من موقعك إلى كل سيرفر مسموح، الأفضل عليه قفل
//! ذهبي، وألواح عائمة فوقها (قائمة السيرفرات، بطاقة أفضل مسار). المنطق القديم كما هو تمامًا
//! (الجدار الناري، البنق، الأوامر)؛ الجديد هو الرسم فقط.

use crate::{
    api::KnownServer,
    app::TemplateApp,
    assets, dropship,
    visuals::{self, Palette},
    world,
};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2, pos2, vec2};

// العروض التي يتنقل بينها الشريط الجانبي (تُخزَّن في `app.tab`)
pub const VIEW_MAP: usize = 0;
pub const VIEW_GAMES: usize = 1;
pub const VIEW_NEWS: usize = 2;
pub const VIEW_LOG: usize = 3;
pub const VIEW_HELP: usize = 4;
pub const VIEW_OPTIONS: usize = 5;

const RAIL_W: f32 = 56.;
const TOP_H: f32 = 46.;
const FOOT_H: f32 = 40.;
const GAP: f32 = 12.;
/// حجم نسيج الخريطة (١٠ نقاط لكل خلية من القناع)
const MAP_W: f32 = world::WORLD_W as f32 * 10.;
const MAP_H: f32 = world::WORLD_H as f32 * 10.;

const ARC_DASH: f32 = 6.;
const ARC_GAP: f32 = 9.;

/// موقع جغرافي (خط عرض، خط طول)
#[derive(Clone, Copy)]
pub struct Geo(pub f32, pub f32);

/// اختصار: مجموعة سيرفرات تُحظر بضغطة زر (أو مفتاح). تُخزَّن برموز السيرفرات لا بأرقامها
/// حتى تبقى صحيحة لو أعاد الـ API ترقيم السيرفرات.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Preset {
    pub name: String,
    pub key: Option<egui::Key>,
    pub blocked: Vec<String>,
}

/// الاختصار الافتراضي لأول تشغيل: «أوروبا» يحظر السيرفر السعودي فقط
pub fn default_presets() -> Vec<Preset> {
    vec![Preset { name: "أوروبا".into(), key: Some(egui::Key::F1), blocked: vec!["gmec2".into()] }]
}

/// حالة نافذة إنشاء/تعديل اختصار (لا تُحفظ)
pub struct PresetEditor {
    /// `None` لاختصار جديد، وإلا رقم الاختصار المعدَّل
    pub idx: Option<usize>,
    pub step: u8,
    pub name: String,
    pub key: Option<egui::Key>,
    /// ننتظر ضغطة مفتاح لتعيينه
    pub listening: bool,
    pub blocked: Vec<String>,
}

/// (رمز المطار في رمز السيرفر مثل ams1 → ams، الموقع، رمز العلم)
const SERVER_GEO: &[(&str, Geo, &str)] = &[
    ("ams", Geo(52.37, 4.9), "nl"),
    // رموز سيرفرات Google Cloud كما تأتي من الـ API: gbr1 ساو باولو، gen1 هامينا، gmec2 الدمام…
    ("gbr", Geo(-23.55, -46.63), "br"),
    ("gen", Geo(60.57, 27.2), "fi"),
    ("gme", Geo(26.43, 50.1), "sa"),
    ("gsg", Geo(1.35, 103.82), "sg"),
    ("gtk", Geo(35.68, 139.69), "jp"),
    ("gue", Geo(38.95, -77.45), "us"),
    ("las", Geo(36.17, -115.14), "us"),
    ("gru", Geo(-23.55, -46.63), "br"),
    ("hel", Geo(60.17, 24.94), "fi"),
    ("ruh", Geo(24.71, 46.68), "sa"),
    ("bah", Geo(26.2, 50.6), "bh"),
    ("dxb", Geo(25.2, 55.3), "ae"),
    ("sin", Geo(1.35, 103.82), "sg"),
    ("nrt", Geo(35.68, 139.69), "jp"),
    ("hnd", Geo(35.55, 139.78), "jp"),
    ("icn", Geo(37.46, 126.44), "kr"),
    ("hkg", Geo(22.31, 113.91), "hk"),
    ("tpe", Geo(25.03, 121.57), "tw"),
    ("syd", Geo(-33.87, 151.21), "au"),
    ("mel", Geo(-37.81, 144.96), "au"),
    ("ord", Geo(41.88, -87.63), "us"),
    ("lax", Geo(34.05, -118.24), "us"),
    ("dfw", Geo(32.78, -96.8), "us"),
    ("iad", Geo(38.95, -77.45), "us"),
    ("mia", Geo(25.79, -80.29), "us"),
    ("sea", Geo(47.45, -122.31), "us"),
    ("atl", Geo(33.64, -84.43), "us"),
    ("yyz", Geo(43.68, -79.63), "ca"),
    ("mex", Geo(19.43, -99.13), "mx"),
    ("eze", Geo(-34.6, -58.38), "ar"),
    ("scl", Geo(-33.45, -70.67), "cl"),
    ("fra", Geo(50.11, 8.68), "de"),
    ("lhr", Geo(51.5, -0.12), "gb"),
    ("cdg", Geo(48.86, 2.35), "fr"),
    ("bom", Geo(19.08, 72.88), "in"),
    ("jnb", Geo(-26.2, 28.05), "za"),
];

/// (كلمة في اسم السيرفر، الموقع، العلم) — احتياط عندما يكون رمز السيرفر غير معروف
const NAME_GEO: &[(&str, Geo, &str)] = &[
    ("netherlands", Geo(52.37, 4.9), "nl"),
    ("brazil", Geo(-23.55, -46.63), "br"),
    ("finland", Geo(60.17, 24.94), "fi"),
    ("saudi", Geo(24.71, 46.68), "sa"),
    ("bahrain", Geo(26.2, 50.6), "bh"),
    ("singapore", Geo(1.35, 103.82), "sg"),
    ("japan", Geo(35.68, 139.69), "jp"),
    ("korea", Geo(37.46, 126.44), "kr"),
    ("hong kong", Geo(22.31, 113.91), "hk"),
    ("taiwan", Geo(25.03, 121.57), "tw"),
    ("australia", Geo(-33.87, 151.21), "au"),
    ("usa - east", Geo(41.88, -87.63), "us"),
    ("usa - southwest", Geo(34.05, -118.24), "us"),
    ("usa - central", Geo(32.78, -96.8), "us"),
    ("usa", Geo(39.0, -98.0), "us"),
    ("canada", Geo(43.68, -79.63), "ca"),
    ("mexico", Geo(19.43, -99.13), "mx"),
    ("argentina", Geo(-34.6, -58.38), "ar"),
    ("chile", Geo(-33.45, -70.67), "cl"),
    ("germany", Geo(50.11, 8.68), "de"),
    ("france", Geo(48.86, 2.35), "fr"),
    ("india", Geo(19.08, 72.88), "in"),
    ("south africa", Geo(-26.2, 28.05), "za"),
];

/// موقع السيرفر وعلمه من رمزه (ams1 → ams) وإلا من اسمه
pub fn server_geo(s: &KnownServer) -> Option<(Geo, &'static str)> {
    let code: String = s.token.chars().take(3).collect::<String>().to_ascii_lowercase();
    if let Some((_, g, f)) = SERVER_GEO.iter().find(|(c, _, _)| *c == code) {
        return Some((*g, f));
    }
    let title = s.title.to_ascii_lowercase();
    NAME_GEO
        .iter()
        .find(|(k, _, _)| title.contains(k))
        .map(|(_, g, f)| (*g, *f))
}

/// (رمز الدولة ISO، عاصمتها تقريبًا)
const COUNTRY_GEO: &[(&str, Geo)] = &[
    ("SA", Geo(24.7, 46.7)),
    ("AE", Geo(24.4, 54.4)),
    ("KW", Geo(29.4, 48.0)),
    ("QA", Geo(25.3, 51.5)),
    ("BH", Geo(26.2, 50.6)),
    ("OM", Geo(23.6, 58.5)),
    ("EG", Geo(30.0, 31.2)),
    ("JO", Geo(31.9, 35.9)),
    ("IQ", Geo(33.3, 44.4)),
    ("LB", Geo(33.9, 35.5)),
    ("SY", Geo(33.5, 36.3)),
    ("YE", Geo(15.4, 44.2)),
    ("PS", Geo(31.9, 35.2)),
    ("MA", Geo(34.0, -6.8)),
    ("DZ", Geo(36.7, 3.1)),
    ("TN", Geo(36.8, 10.2)),
    ("LY", Geo(32.9, 13.2)),
    ("SD", Geo(15.6, 32.5)),
    ("TR", Geo(39.9, 32.9)),
    ("IR", Geo(35.7, 51.4)),
    ("PK", Geo(33.7, 73.1)),
    ("IN", Geo(28.6, 77.2)),
    ("US", Geo(39.0, -98.0)),
    ("CA", Geo(45.4, -75.7)),
    ("MX", Geo(19.4, -99.1)),
    ("BR", Geo(-15.8, -47.9)),
    ("AR", Geo(-34.6, -58.4)),
    ("CL", Geo(-33.5, -70.7)),
    ("GB", Geo(51.5, -0.1)),
    ("IE", Geo(53.3, -6.3)),
    ("FR", Geo(48.9, 2.3)),
    ("DE", Geo(52.5, 13.4)),
    ("ES", Geo(40.4, -3.7)),
    ("PT", Geo(38.7, -9.1)),
    ("IT", Geo(41.9, 12.5)),
    ("NL", Geo(52.4, 4.9)),
    ("BE", Geo(50.8, 4.4)),
    ("CH", Geo(46.9, 7.4)),
    ("AT", Geo(48.2, 16.4)),
    ("SE", Geo(59.3, 18.1)),
    ("NO", Geo(59.9, 10.8)),
    ("FI", Geo(60.2, 24.9)),
    ("DK", Geo(55.7, 12.6)),
    ("PL", Geo(52.2, 21.0)),
    ("CZ", Geo(50.1, 14.4)),
    ("GR", Geo(38.0, 23.7)),
    ("RO", Geo(44.4, 26.1)),
    ("UA", Geo(50.5, 30.5)),
    ("RU", Geo(55.8, 37.6)),
    ("JP", Geo(35.7, 139.7)),
    ("KR", Geo(37.6, 127.0)),
    ("CN", Geo(39.9, 116.4)),
    ("TW", Geo(25.0, 121.6)),
    ("HK", Geo(22.3, 114.2)),
    ("SG", Geo(1.35, 103.8)),
    ("MY", Geo(3.1, 101.7)),
    ("TH", Geo(13.8, 100.5)),
    ("VN", Geo(21.0, 105.8)),
    ("PH", Geo(14.6, 121.0)),
    ("ID", Geo(-6.2, 106.8)),
    ("AU", Geo(-33.9, 151.2)),
    ("NZ", Geo(-41.3, 174.8)),
    ("ZA", Geo(-25.7, 28.2)),
    ("NG", Geo(9.1, 7.5)),
    ("KE", Geo(-1.3, 36.8)),
];

/// موقع المستخدم التقريبي من إعدادات المنطقة في ويندوز (الدولة ← عاصمتها). `None` لو غير معروفة.
pub fn user_geo() -> Option<Geo> {
    let mut buf = [0u16; 16];
    let n = unsafe { windows::Win32::Globalization::GetUserDefaultGeoName(&mut buf) };
    if n <= 1 {
        return None;
    }
    let iso = String::from_utf16_lossy(&buf[..(n as usize - 1)]).to_ascii_uppercase();
    log::debug!("منطقة الجهاز: {iso}");
    COUNTRY_GEO.iter().find(|(c, _)| *c == iso).map(|(_, g)| *g)
}

/// إسقاط متساوي المستطيلات إلى مستطيل الخريطة
fn project(g: Geo, map: Rect) -> Pos2 {
    let x = (g.1 + 180.) / 360.;
    let y = (world::LAT_TOP - g.0) / (world::LAT_TOP - world::LAT_BOT);
    pos2(map.min.x + x * map.width(), map.min.y + y * map.height())
}

/// درجة البنق: 0 ممتاز، 1 مقبول، 2 ضعيف
fn grade(ms: f32) -> u8 {
    if ms < 50. {
        0
    } else if ms < 100. {
        1
    } else {
        2
    }
}

fn grade_color(pal: &Palette, g: u8) -> Color32 {
    match g {
        0 => pal.accent,
        1 => pal.gold,
        _ => pal.red,
    }
}

fn font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Monospace)
}

/// نسيج العالم المنقّط، يُبنى مرة واحدة لكل مظهر. النقاط أوضح قرب موقع المستخدم.
fn world_texture(ctx: &egui::Context, pal: &Palette, you: Option<Pos2>) -> egui::TextureHandle {
    let (w, h) = (MAP_W as usize, MAP_H as usize);
    let mut img = egui::ColorImage::filled([w, h], Color32::TRANSPARENT);
    let base = pal.accent;
    for (j, row) in world::WORLD.iter().enumerate() {
        for (i, ch) in row.bytes().enumerate() {
            if ch != b'1' {
                continue;
            }
            let cx = i as f32 * 10. + 5.;
            let cy = j as f32 * 10. + 5.;
            let near = you
                .map(|y| (1. - y.distance(pos2(cx, cy)) / 700.).max(0.))
                .unwrap_or(0.);
            let alpha = 0.3 + near * 0.5;
            let r = 2.6 + near * 1.2;
            let color = base.gamma_multiply(alpha);
            // دائرة صغيرة مملوءة بحواف ناعمة
            let r2 = (r + 1.).ceil() as i32;
            for dy in -r2..=r2 {
                for dx in -r2..=r2 {
                    let px = cx as i32 + dx;
                    let py = cy as i32 + dy;
                    if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                        continue;
                    }
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    let cov = (r + 0.5 - d).clamp(0., 1.);
                    if cov > 0. {
                        let idx = py as usize * w + px as usize;
                        img.pixels[idx] = color.gamma_multiply(cov);
                    }
                }
            }
        }
    }
    ctx.load_texture("world-dots", img, egui::TextureOptions::LINEAR)
}

/// سيرفر مع موقعه على الخريطة وبنقه
struct Node {
    idx: usize,
    pos: Pos2,
    flag: &'static str,
    ms: Option<f32>,
    blocked: bool,
    pending: bool,
}

impl TemplateApp {
    /// يرسم الواجهة كاملة (بدون النوافذ المنبثقة والجولة، تلك تُرسم بعده)
    pub fn launcher_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let theme = self.get_theme(ui);
        let pal = visuals::palette(theme);
        let full = ui.max_rect();
        let animated = cfg!(feature = "animations")
            && ui.input(|i| i.focused)
            && !self.config.mini
            && self.tour.is_none();

        // المؤشر فوق سيرفر؟ يُحدَّث من الخريطة والقائمة كل إطار
        self.hover_server = None;

        // خلفية + توهج ناعم
        let p = ui.painter().clone();
        p.rect_filled(full, 0., pal.bg);
        {
            let glow = |c: Pos2, r: f32, color: Color32| {
                for k in 1..=7 {
                    let t = k as f32 / 7.;
                    p.circle_filled(c, r * (1. - t * 0.9), color.gamma_multiply(0.012));
                }
            };
            glow(full.left_top() + vec2(full.width() * 0.2, 0.), 520., pal.accent);
            glow(full.right_bottom(), 420., pal.gold);
        }

        // ---------------------------------------------------------- تقسيم النافذة
        if self.config.mini {
            let panel = full.shrink2(vec2(8., 8.));
            self.panel_servers(ui, panel, &pal, true);
            self.preset_editor_ui(ui, &pal);
            return;
        }

        let top = Rect::from_min_size(full.min, vec2(full.width(), TOP_H));
        let foot = Rect::from_min_max(pos2(full.min.x, full.max.y - FOOT_H), full.max);
        // الشريط الجانبي على اليمين (بداية السطر في العربية)
        let rail = Rect::from_min_max(pos2(full.max.x - RAIL_W, top.max.y), pos2(full.max.x, foot.min.y));
        let body = Rect::from_min_max(pos2(full.min.x, top.max.y), pos2(rail.min.x, foot.min.y));

        let panel_w: f32 = match self.tab {
            VIEW_MAP => 330_f32,
            VIEW_GAMES => 430.,
            VIEW_NEWS => 400.,
            VIEW_LOG => 470.,
            VIEW_HELP => 380.,
            _ => 640.,
        }
        .min(body.width() - 2. * GAP);
        // اللوحة العائمة على اليسار (نهاية السطر)، والخريطة في المساحة المتبقية
        let panel = Rect::from_min_size(body.min + vec2(GAP, GAP), vec2(panel_w, body.height() - 2. * GAP));
        // الخريطة بحجم ثابت (كما في عرض الخريطة) مهما اتسعت اللوحة فوقها، حتى لا تقفز بين العروض
        let map_area = Rect::from_min_max(pos2(body.min.x + GAP + 330. + GAP, body.min.y), body.max);

        self.paint_map(ui, map_area, &pal, animated, self.tab == VIEW_MAP);
        self.tour_mark("map", map_area.shrink(8.));

        if self.tab == VIEW_MAP {
            let card = Rect::from_min_size(
                pos2(body.max.x - GAP - 300., body.max.y - GAP - 118.),
                vec2(300., 118.),
            );
            self.route_card(ui, card, &pal);
            self.tour_mark("route", card);
        }

        // اللوحة
        match self.tab {
            VIEW_MAP => self.panel_servers(ui, panel, &pal, false),
            _ => self.panel_view(ui, panel, &pal, frame),
        }

        self.top_bar(ui, top, &pal);
        self.rail(ui, rail, &pal);
        self.footer(ui, foot, &pal);
        self.preset_editor_ui(ui, &pal);

        if animated {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
        }
    }

    // ------------------------------------------------------------------ الخريطة

    /// `full`: عرض الخريطة نفسه (لافتات وتفاعل)، وإلا خلفية خافتة تحت لوحة أخرى
    fn paint_map(&mut self, ui: &mut egui::Ui, area: Rect, pal: &Palette, animated: bool, full: bool) {
        let theme = self.get_theme(ui);
        let painter = ui.painter().with_clip_rect(area);

        // مستطيل الخريطة: أعرض من المساحة بقليل، متمركز على خط طول ١٤° شرقًا حتى تظهر
        // كل السيرفرات من لاس فيغاس إلى سيدني، والحواف (محيطات) تُقص
        let scale = (area.width() * 1.28) / MAP_W;
        let size = vec2(MAP_W, MAP_H) * scale;
        let map = Rect::from_center_size(
            pos2(area.center().x - (14. / 360.) * size.x, area.min.y + area.height() * 0.46),
            size,
        );

        let you = self.user_geo.map(|g| project(g, map));

        if !self.config.disable_background_image {
            let need = self.world_tex.as_ref().map_or(true, |(t, _)| *t != theme);
            if need {
                let you_tex = self.user_geo.map(|g| {
                    project(g, Rect::from_min_size(Pos2::ZERO, vec2(MAP_W, MAP_H)))
                });
                self.world_tex = Some((theme, world_texture(ui.ctx(), pal, you_tex)));
            }
            if let Some((_, tex)) = &self.world_tex {
                painter.image(tex.id(), map, Rect::from_min_max(Pos2::ZERO, pos2(1., 1.)), Color32::WHITE);
            }
        }

        // العقد
        let servers = self.known_servers().to_vec();
        let nodes: Vec<Node> = servers
            .iter()
            .enumerate()
            .filter_map(|(idx, s)| {
                let (g, flag) = server_geo(s)?;
                Some(Node {
                    idx,
                    pos: project(g, map),
                    flag,
                    ms: self.pings.get(&s.ping).and_then(|r| r.as_ref().ok().copied()),
                    blocked: self.config.desired_blocked_servers.has(s),
                    pending: self.config.desired_blocked_servers.has(s) != self.config.blocked_servers.has(s),
                })
            })
            .collect();

        let best = self.get_most_likely_to_play_on().map(|s| s.bit);
        // لو موقع المستخدم غير معروف، نضع «أنت» قرب أقرب سيرفر
        let you = you.or_else(|| {
            best.and_then(|b| nodes.iter().find(|n| servers[n.idx].bit == b))
                .map(|n| n.pos + vec2(-40., 30.))
        });
        // لو كان فوق سيرفر (سعودي في السعودية مثلًا) نزيحه قليلًا حتى يظهر الاثنان
        let you = you.map(|y| if nodes.iter().any(|n| n.pos.distance(y) < 22.) { y + vec2(-16., 16.) } else { y });

        let time = ui.input(|i| i.time) as f32;

        // المسارات من «أنت» إلى كل سيرفر مسموح
        if let Some(you) = you {
            for n in nodes.iter().filter(|n| !n.blocked) {
                let is_best = best == Some(servers[n.idx].bit);
                let hot = self.hover_server == Some(servers[n.idx].bit);
                let mid = pos2(
                    (you.x + n.pos.x) / 2.,
                    (you.y + n.pos.y) / 2. - you.distance(n.pos) * 0.22,
                );
                let curve = egui::epaint::QuadraticBezierShape {
                    points: [you, mid, n.pos],
                    closed: false,
                    fill: Color32::TRANSPARENT,
                    stroke: egui::epaint::PathStroke::NONE,
                };
                let pts = curve.flatten(Some(0.5));
                let (color, width, speed) = if is_best {
                    (pal.gold, 2.4, 40.)
                } else if hot {
                    (pal.accent, 2.2, 28.)
                } else {
                    (pal.accent.gamma_multiply(0.55), 1.4, 20.)
                };
                let offset = if animated { -(time * speed) % (ARC_DASH + ARC_GAP) } else { 0. };
                for shape in egui::Shape::dashed_line_with_offset(
                    &pts,
                    egui::Stroke::new(width, color),
                    &[ARC_DASH],
                    &[ARC_GAP],
                    offset,
                ) {
                    painter.add(shape);
                }
            }

            // «أنت»
            let pulse = if animated { (time * 1.3) % 1. } else { 0.5 };
            // بلون النص حتى يظهر في المظهر الفاتح أيضًا
            painter.circle_stroke(you, 8. + 18. * pulse, egui::Stroke::new(1.5, pal.text.gamma_multiply(0.55 * (1. - pulse))));
            painter.circle_filled(you, 14., pal.text.gamma_multiply(0.22));
            painter.circle_filled(you, 7., pal.text);
            if full {
                painter.text(you + vec2(0., 24.), egui::Align2::CENTER_CENTER, "أنت", font(12.), pal.text);
            }
        }

        // العقد فوق المسارات: الأفضل والمؤشَّر أولًا حتى تأخذ لافتاتهما المكان قبل غيرها
        let mut hover = None;
        let mut clicked: Option<(usize, bool)> = None;
        let mut order: Vec<usize> = (0..nodes.len()).collect();
        order.sort_by_key(|&k| {
            let b = servers[nodes[k].idx].bit;
            if best == Some(b) || self.hover_server == Some(b) { 0 } else { 1 }
        });
        // ما احتُلّ من الخريطة: النقاط نفسها ثم اللافتات الموضوعة
        let mut taken: Vec<Rect> = nodes.iter().map(|n| Rect::from_center_size(n.pos, vec2(22., 22.))).collect();
        for k in order {
            let n = &nodes[k];
            let s = &servers[n.idx];
            let is_best = best == Some(s.bit);
            let hot = self.hover_server == Some(s.bit);
            let color = if n.blocked {
                pal.red
            } else if is_best {
                pal.gold
            } else {
                pal.accent
            };

            if is_best {
                // قفل الهدف: حلقة متقطعة تدور
                let r = 22.;
                let rot = if animated { time * 0.6 } else { 0. };
                let ring: Vec<Pos2> = (0..=48)
                    .map(|k| {
                        let a = rot + k as f32 / 48. * std::f32::consts::TAU;
                        n.pos + vec2(a.cos(), a.sin()) * r
                    })
                    .collect();
                for shape in egui::Shape::dashed_line(&ring, egui::Stroke::new(1.5, pal.gold), 8., 6.) {
                    painter.add(shape);
                }
            }

            painter.circle_filled(n.pos, if hot { 15. } else { 12. }, color.gamma_multiply(0.22));
            painter.circle_filled(n.pos, if hot { 8. } else { 6. }, color);
            if !full {
                continue;
            }

            // اللافتة: علم + اسم + بنق في كبسولة واحدة، توضع في أول جهة لا تصطدم بما قبلها
            // (يمين، يسار، تحت، فوق) وإلا تُخفى ويبقى العلم وحده قرب النقطة
            let label = match n.ms {
                Some(ms) if !n.pending => format!("{}  {:.0}", s.title, ms),
                _ if n.pending => format!("{}  …", s.title),
                _ => s.title.clone(),
            };
            let galley = painter.layout_no_wrap(label, font(11.), if n.blocked { pal.faint } else if hot { pal.text } else { pal.muted });
            let flag_w = 20.;
            let pill_size = vec2(flag_w + 6. + galley.size().x + 14., 20.);
            let candidates = [
                (vec2(13., 0.), egui::Align2::LEFT_CENTER),
                (vec2(-13., 0.), egui::Align2::RIGHT_CENTER),
                (vec2(0., 13.), egui::Align2::CENTER_TOP),
                (vec2(0., -13.), egui::Align2::CENTER_BOTTOM),
            ];
            let pill = candidates.iter().map(|(d, a)| a.anchor_size(n.pos + *d, pill_size)).find(|r| {
                area.contains_rect(*r) && !taken.iter().any(|t| t.intersects(*r))
            });
            let mut hit = Rect::from_center_size(n.pos, vec2(30., 30.));
            match pill {
                Some(r) => {
                    taken.push(r);
                    hit = hit.union(r);
                    painter.rect_filled(r, 6., pal.bg.gamma_multiply(if hot { 0.95 } else { 0.8 }));
                    painter.rect_stroke(r, 6., egui::Stroke::new(1., if hot { color.gamma_multiply(0.6) } else { pal.line }), egui::StrokeKind::Inside);
                    let flag_rect = Rect::from_min_size(r.min + vec2(5., 3.), vec2(flag_w, 14.));
                    if let Some(src) = assets::flag(n.flag) {
                        let mut img = egui::Image::new(src).fit_to_exact_size(flag_rect.size()).corner_radius(2.);
                        if n.blocked {
                            img = img.tint(Color32::from_gray(140));
                        }
                        img.paint_at(ui, flag_rect);
                    }
                    let text_pos = pos2(flag_rect.max.x + 6., r.center().y - galley.size().y / 2.);
                    painter.galley(text_pos, galley.clone(), pal.text);
                    if n.blocked {
                        let y = r.center().y;
                        painter.line_segment([pos2(text_pos.x, y), pos2(text_pos.x + galley.size().x, y)], egui::Stroke::new(1., pal.red.gamma_multiply(0.8)));
                    }
                }
                None => {
                    // مزدحم: العلم وحده فوق النقطة
                    let flag_rect = Rect::from_center_size(n.pos + vec2(0., -16.), vec2(16., 11.));
                    if let Some(src) = assets::flag(n.flag) {
                        egui::Image::new(src).fit_to_exact_size(flag_rect.size()).corner_radius(2.).paint_at(ui, flag_rect);
                    }
                    hit = hit.union(flag_rect);
                }
            }

            // التفاعل
            let resp = ui.interact(hit, egui::Id::new(("map_node", s.bit)), egui::Sense::click());
            if resp.hovered() {
                hover = Some(s.bit);
            }
            if resp.clicked() {
                clicked = Some((n.idx, false));
            } else if resp.secondary_clicked() {
                clicked = Some((n.idx, true));
            }
            let _ = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        if !full {
            // تحت لوحة أخرى: نخفّف الخريطة حتى لا تشوّش على محتواها
            painter.rect_filled(area, 0., pal.bg.gamma_multiply(0.5));
        }
        if hover.is_some() {
            self.hover_server = hover;
        }
        if let Some((idx, invert)) = clicked {
            self.toggle_server(idx, invert);
        }

        // خط مسح خفيف
        if animated {
            let y = area.min.y + ((time * 0.11) % 1.) * area.height();
            painter.rect_filled(
                Rect::from_min_max(pos2(area.min.x, y - 10.), pos2(area.max.x, y)),
                0.,
                pal.accent.gamma_multiply(0.06),
            );
        }
    }

    /// يبدّل حظر السيرفر رقم `idx` (أو يعكس الباقي مع الزر الأيمن) بنفس قواعد القائمة القديمة
    pub fn toggle_server(&mut self, idx: usize, invert: bool) {
        let servers = self.known_servers().to_vec();
        let Some(server) = servers.get(idx) else { return };
        let mut sel = self.config.desired_blocked_servers.clone();
        let remaining = servers.iter().filter(|x| !sel.has(x)).count();
        if invert {
            if !sel.has(server) && remaining == 1 {
                sel.solo(server);
            } else {
                sel.solo_invert(server);
            }
        } else if !sel.has(server) && remaining == 1 {
            log::warn!("ما يمكن حظر {} لأن كل السيرفرات بتصير محظورة", server.title.to_ascii_lowercase());
            return;
        } else {
            sel.toggle(server);
        }
        if sel.bits() != self.config.desired_blocked_servers.bits() {
            self.config.desired_blocked_servers = sel;
            self.apply_blocked_servers_to_firewall();
        }
    }

    // ------------------------------------------------------------ الاختصارات

    /// السيرفرات التي يحظرها الاختصار، بأرقام القائمة الحالية
    fn preset_selection(&self, p: &Preset) -> crate::overwatch::ServerSelection {
        let mut sel = crate::overwatch::ServerSelection::none();
        for s in self.known_servers() {
            if p.blocked.iter().any(|t| t == &s.token) {
                sel.set(s);
            }
        }
        sel
    }

    /// يطبّق الاختصار رقم `i`: يحظر سيرفراته ويسمح بالباقي (بنفس قواعد القائمة)
    pub(crate) fn apply_preset(&mut self, i: usize) {
        let Some(p) = self.config.presets.get(i).cloned() else { return };
        let sel = self.preset_selection(&p);
        let servers = self.known_servers();
        if !servers.is_empty() && servers.iter().all(|s| sel.has(s)) {
            log::warn!("اختصار «{}» يحظر كل السيرفرات، ما طُبّق", p.name);
            return;
        }
        if sel.bits() != self.config.desired_blocked_servers.bits() {
            self.config.desired_blocked_servers = sel;
            self.apply_blocked_servers_to_firewall();
        }
        log::info!("طُبّق اختصار «{}»", p.name);
    }

    pub(crate) fn open_preset_editor(&mut self, idx: Option<usize>) {
        let p = idx.and_then(|i| self.config.presets.get(i)).cloned();
        self.preset_editor = Some(PresetEditor {
            idx,
            step: 0,
            name: p.as_ref().map_or(String::new(), |p| p.name.clone()),
            key: p.as_ref().and_then(|p| p.key),
            listening: false,
            blocked: p.map_or(Vec::new(), |p| p.blocked),
        });
    }

    /// نافذة الاختصار على خطوتين: الاسم والمفتاح، ثم السيرفرات
    fn preset_editor_ui(&mut self, ui: &mut egui::Ui, pal: &Palette) {
        let Some(mut ed) = self.preset_editor.take() else { return };
        let servers = self.known_servers().to_vec();
        let was_listening = ed.listening;
        let (mut close, mut save, mut delete) = (false, false, false);
        let rtl = egui::Layout::right_to_left(egui::Align::Center);

        let modal = egui::Modal::new(egui::Id::new("preset_editor")).show(ui.ctx(), |ui| {
            ui.set_max_width(340.);
            ui.set_max_height(440.);
            ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                ui.label(egui::RichText::new(if ed.idx.is_some() { "تعديل الاختصار" } else { "اختصار جديد" }).strong().size(15.));
                ui.add_space(8.);
                if ed.step == 0 {
                    ui.label(egui::RichText::new("١. اسم الاختصار (يظهر على الزر)").color(pal.muted).size(12.));
                    let te = ui.add(egui::TextEdit::singleline(&mut ed.name).hint_text("مثال: أوروبا").horizontal_align(egui::Align::RIGHT).desired_width(f32::INFINITY));
                    if ed.idx.is_none() && ed.name.is_empty() && !ed.listening {
                        te.request_focus();
                    }
                    ui.add_space(10.);
                    ui.label(egui::RichText::new("٢. مفتاح سريع (اختياري)").color(pal.muted).size(12.));
                    ui.horizontal(|ui| ui.with_layout(rtl, |ui| {
                        let label = if ed.listening {
                            "اضغط أي مفتاح… (Esc للإلغاء)".to_owned()
                        } else {
                            ed.key.map_or("بدون مفتاح".to_owned(), |k| k.name().to_owned())
                        };
                        let b = ui.add(
                            egui::Button::new(egui::RichText::new(label).size(12.))
                                .fill(if ed.listening { pal.accent_deep } else { pal.panel_2 })
                                .corner_radius(8.)
                                .min_size(vec2(170., 28.)),
                        );
                        if b.on_hover_text_at_pointer("اضغط ثم اختر المفتاح").clicked() {
                            ed.listening = !ed.listening;
                        }
                        if ed.key.is_some() && !ed.listening {
                            if ui.add(egui::Button::new(egui::RichText::new("مسح").size(11.).color(pal.muted)).fill(Color32::TRANSPARENT).corner_radius(8.)).clicked() {
                                ed.key = None;
                            }
                        }
                    }));
                    if ed.listening {
                        let pressed = ui.input(|i| {
                            i.events.iter().find_map(|e| match e {
                                egui::Event::Key { key, pressed: true, .. } => Some(*key),
                                _ => None,
                            })
                        });
                        if let Some(k) = pressed {
                            if k != egui::Key::Escape {
                                ed.key = Some(k);
                            }
                            ed.listening = false;
                        }
                    }
                    ui.add_space(14.);
                    ui.horizontal(|ui| ui.with_layout(rtl, |ui| {
                        if ui.add_enabled(!ed.name.trim().is_empty(), egui::Button::new("التالي")).clicked() {
                            ed.step = 1;
                        }
                        if ui.button("إلغاء").clicked() {
                            close = true;
                        }
                    }));
                } else {
                    ui.label(egui::RichText::new("٣. السيرفرات التي تُحظر عند الضغط").color(pal.muted).size(12.));
                    ui.horizontal(|ui| ui.with_layout(rtl, |ui| {
                        if ui.small_button("الكل").clicked() {
                            ed.blocked = servers.iter().map(|s| s.token.clone()).collect();
                        }
                        if ui.small_button("عكس").clicked() {
                            ed.blocked = servers.iter().filter(|s| !ed.blocked.contains(&s.token)).map(|s| s.token.clone()).collect();
                        }
                        if ui.small_button("لا شيء").clicked() {
                            ed.blocked.clear();
                        }
                    }));
                    ui.add_space(4.);
                    egui::ScrollArea::vertical().max_height(300.).show(ui, |ui| {
                        for s in &servers {
                            let mut on = ed.blocked.contains(&s.token);
                            ui.horizontal(|ui| ui.with_layout(rtl, |ui| {
                                if ui.checkbox(&mut on, "").changed() {
                                    if on {
                                        ed.blocked.push(s.token.clone());
                                    } else {
                                        ed.blocked.retain(|t| t != &s.token);
                                    }
                                }
                                if let Some(src) = server_geo(s).and_then(|(_, f)| assets::flag(f)) {
                                    ui.add(egui::Image::new(src).fit_to_exact_size(vec2(20., 14.)).corner_radius(2.));
                                }
                                ui.label(egui::RichText::new(&s.title).size(13.).color(if on { pal.red } else { pal.text }));
                                ui.label(egui::RichText::new(s.token.to_ascii_uppercase()).size(9.).color(pal.faint));
                            }));
                        }
                    });
                    let n_blocked = servers.iter().filter(|s| ed.blocked.contains(&s.token)).count();
                    let all = !servers.is_empty() && n_blocked == servers.len();
                    ui.add_space(6.);
                    ui.label(
                        egui::RichText::new(if all {
                            "لا يمكن حظر كل السيرفرات".to_owned()
                        } else {
                            format!("يُحظر {} من {}", n_blocked, servers.len())
                        })
                        .size(11.)
                        .color(if all { pal.red } else { pal.muted }),
                    );
                    ui.add_space(10.);
                    ui.horizontal(|ui| ui.with_layout(rtl, |ui| {
                        if ui.add_enabled(!all, egui::Button::new("حفظ")).clicked() {
                            save = true;
                        }
                        if ui.button("رجوع").clicked() {
                            ed.step = 0;
                        }
                        if ed.idx.is_some() {
                            if ui.add(egui::Button::new(egui::RichText::new("حذف").color(pal.red)).fill(Color32::TRANSPARENT)).clicked() {
                                delete = true;
                            }
                        }
                    }));
                }
            });
        });
        // Esc أثناء انتظار المفتاح يلغي الانتظار فقط
        if !was_listening && modal.should_close() {
            close = true;
        }

        if save {
            let p = Preset { name: ed.name.trim().to_owned(), key: ed.key, blocked: ed.blocked.clone() };
            match ed.idx {
                Some(i) if i < self.config.presets.len() => self.config.presets[i] = p,
                _ => self.config.presets.push(p),
            }
            log::info!("حُفظ اختصار «{}»", ed.name.trim());
        } else if delete {
            if let Some(i) = ed.idx {
                if i < self.config.presets.len() {
                    let p = self.config.presets.remove(i);
                    log::info!("حُذف اختصار «{}»", p.name);
                }
            }
        } else if !close {
            self.preset_editor = Some(ed);
        }
    }

    // ------------------------------------------------------------ الألواح

    fn glass(ui: &mut egui::Ui, rect: Rect, pal: &Palette, gold: bool) -> egui::Ui {
        let stroke = if gold { pal.gold.gamma_multiply(0.5) } else { pal.line_2 };
        let p = ui.painter();
        p.add(egui::epaint::Shadow { offset: [0, 14], blur: 40, spread: 0, color: Color32::from_black_alpha(90) }.as_shape(rect, 16.));
        p.rect_filled(rect, 16., pal.panel);
        if gold {
            // لمسة ذهبية في الزاوية
            p.rect_filled(rect, 16., pal.gold.gamma_multiply(0.08));
        }
        p.rect_stroke(rect, 16., egui::Stroke::new(1., stroke), egui::StrokeKind::Inside);
        ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink(12.))
                .layout(egui::Layout::top_down(egui::Align::RIGHT)),
        )
    }

    fn panel_servers(&mut self, ui: &mut egui::Ui, rect: Rect, pal: &Palette, mini: bool) {
        let mut child = Self::glass(ui, rect, pal, false);
        let ui = &mut child;
        ui.spacing_mut().item_spacing = vec2(8., 4.);

        // العنوان
        let servers = self.known_servers().to_vec();
        let blocked_n = servers.iter().filter(|s| self.config.desired_blocked_servers.has(s)).count();
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("السيرفرات").strong().size(15.));
                pill(ui, &format!("{}/{}", servers.len() - blocked_n, servers.len()), pal.muted, pal.panel_2);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    // الوضع المصغّر
                    let icon = egui::Image::new(assets::ANGLES_RIGHT)
                        .fit_to_exact_size(vec2(12., 12.))
                        .tint(pal.muted)
                        .rotate(if mini { 0. } else { std::f32::consts::PI }, Vec2::splat(0.5));
                    let b = ui.add(egui::Button::image(icon).fill(pal.panel_2).corner_radius(8.));
                    self.tour_mark("mini", b.rect);
                    if b.on_hover_text_at_pointer(if mini { "إظهار التفاصيل" } else { "الوضع المصغّر" }).clicked() {
                        self.config.mini = !self.config.mini;
                        self.apply_mini_mode(ui.ctx());
                    }
                    // ترتيب حسب البنق
                    let sort = self.config.sort_by_ping;
                    let icon = egui::Image::new(assets::ICON_SORT).fit_to_exact_size(vec2(12., 12.)).tint(if sort { pal.accent } else { pal.muted });
                    let b = ui.add(egui::Button::image_and_text(icon, egui::RichText::new("حسب البنق").size(11.).color(if sort { pal.accent } else { pal.muted })).fill(if sort { pal.panel_2 } else { Color32::TRANSPARENT }).corner_radius(8.));
                    if b.clicked() {
                        self.config.sort_by_ping = !sort;
                    }
                });
            });
        });
        ui.add_space(4.);

        // الاختصارات: مجموعات حظر جاهزة، يسار يطبّق ويمين يعدّل، و+ ينشئ
        let presets_top = ui.cursor().min.y;
        let mut fire: Option<usize> = None;
        let mut edit: Option<Option<usize>> = None;
        let row = egui::Layout::right_to_left(egui::Align::Center).with_main_wrap(true);
        ui.allocate_ui_with_layout(vec2(ui.available_width(), 26.), row, |ui| {
            ui.spacing_mut().item_spacing = vec2(6., 4.);
            ui.label(egui::RichText::new("اختصارات").size(11.).color(pal.muted));
            for (i, p) in self.config.presets.iter().enumerate() {
                let active = self.preset_selection(p).bits() == self.config.desired_blocked_servers.bits();
                let text = match p.key {
                    Some(k) => format!("{}  {}", p.name, k.name()),
                    None => p.name.clone(),
                };
                let (fg, bg, line) = if active { (Color32::WHITE, pal.accent_deep, pal.accent_deep) } else { (pal.text, pal.panel_2, pal.line_2) };
                let b = ui.add(
                    egui::Button::new(egui::RichText::new(text).size(11.).color(fg))
                        .fill(bg)
                        .stroke(egui::Stroke::new(1., line))
                        .corner_radius(999.),
                );
                let b = b.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text_at_pointer(format!(
                    "يحظر: {}\nيسار: تطبيق · يمين: تعديل",
                    if p.blocked.is_empty() { "لا شيء".to_owned() } else { p.blocked.join("، ") }
                ));
                if b.clicked() {
                    fire = Some(i);
                } else if b.secondary_clicked() {
                    edit = Some(Some(i));
                }
            }
            let b = ui.add(
                egui::Button::new(egui::RichText::new("+").size(13.).color(pal.accent))
                    .fill(Color32::TRANSPARENT)
                    .stroke(egui::Stroke::new(1., pal.line_2))
                    .corner_radius(999.),
            );
            if b.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text_at_pointer("اختصار جديد").clicked() {
                edit = Some(None);
            }
        });
        self.tour_mark("presets", Rect::from_min_max(pos2(rect.min.x, presets_top), pos2(rect.max.x, ui.cursor().min.y)));
        if let Some(i) = fire {
            self.apply_preset(i);
        }
        if let Some(e) = edit {
            self.open_preset_editor(e);
        }
        ui.add_space(4.);

        // القائمة
        let list_top = ui.cursor().min.y;
        let list_h = ui.available_height() - 44.;
        let mut order: Vec<usize> = (0..servers.len()).collect();
        if self.config.sort_by_ping {
            order.sort_by(|a, b| {
                let pa = self.pings.get(&servers[*a].ping).and_then(|r| r.as_ref().ok().copied()).unwrap_or(f32::INFINITY);
                let pb = self.pings.get(&servers[*b].ping).and_then(|r| r.as_ref().ok().copied()).unwrap_or(f32::INFINITY);
                pa.total_cmp(&pb)
            });
        }
        let best = self.get_most_likely_to_play_on().map(|s| s.bit);
        let max_ms = servers
            .iter()
            .filter_map(|s| self.pings.get(&s.ping).and_then(|r| r.as_ref().ok().copied()))
            .fold(1f32, f32::max);
        let mut clicked: Option<(usize, bool)> = None;
        let mut hover = None;

        egui::ScrollArea::vertical()
            .max_height(list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if servers.is_empty() {
                    ui.label(egui::RichText::new("لا توجد سيرفرات معروفة").color(pal.muted));
                }
                for idx in order {
                    let s = &servers[idx];
                    let (r, hov) = self.server_row(ui, s, best == Some(s.bit), max_ms, pal);
                    if hov {
                        hover = Some(s.bit);
                    }
                    match r {
                        Some(false) => clicked = Some((idx, false)),
                        Some(true) => clicked = Some((idx, true)),
                        None => {}
                    }
                }
            });
        self.tour_mark("servers", Rect::from_min_max(pos2(rect.min.x, list_top), pos2(rect.max.x, list_top + list_h)));
        if hover.is_some() {
            self.hover_server = hover;
        }
        if let Some((idx, invert)) = clicked {
            self.toggle_server(idx, invert);
        }

        // ارفع كل الحظر
        ui.add_space(6.);
        let b = ui.add_sized(
            vec2(ui.available_width(), 32.),
            egui::Button::new(egui::RichText::new("ارفع كل الحظر").color(pal.muted))
                .fill(Color32::TRANSPARENT)
                .stroke(egui::Stroke::new(1., pal.line))
                .corner_radius(10.),
        );
        self.tour_mark("disable", b.rect);
        if b.on_hover_text("إذا فشل الاتصال بأي سيرفر، اضغط هذا الزر بسرعة لتتجنب حظر التنافسي").clicked() {
            self.force_unblock_all();
        }
    }

    /// صف سيرفر: علم، اسم ورمز، شريط بنق، الرقم، مفتاح. يعيد (نقر عادي/أيمن، هل المؤشر فوقه)
    fn server_row(&self, ui: &mut egui::Ui, s: &KnownServer, is_best: bool, max_ms: f32, pal: &Palette) -> (Option<bool>, bool) {
        let blocked = self.config.desired_blocked_servers.has(s);
        let pending = blocked != self.config.blocked_servers.has(s);
        let ms = self.pings.get(&s.ping).and_then(|r| r.as_ref().ok().copied());

        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.), egui::Sense::click());
        let id = egui::Id::new(("row_hover", s.bit));
        let t = ui.ctx().animate_bool_with_time(id, resp.hovered(), 0.15);
        let p = ui.painter();
        let bg = if is_best {
            pal.gold.gamma_multiply(0.10)
        } else {
            pal.panel_2.gamma_multiply(t)
        };
        p.rect_filled(rect, 10., bg);
        if is_best {
            p.rect_stroke(rect, 10., egui::Stroke::new(1., pal.gold.gamma_multiply(0.55)), egui::StrokeKind::Inside);
        }
        let alpha = if blocked { 0.5 } else { 1. };

        // من اليمين: العلم
        let flag_rect = Rect::from_center_size(pos2(rect.max.x - 8. - 14., rect.center().y), vec2(28., 21.));
        if let Some(src) = server_geo(s).and_then(|(_, f)| assets::flag(f)) {
            let mut img = egui::Image::new(src).fit_to_exact_size(flag_rect.size()).corner_radius(4.);
            if blocked {
                img = img.tint(Color32::from_gray(150));
            }
            img.paint_at(ui, flag_rect);
        } else {
            p.rect_filled(flag_rect, 4., pal.panel_2);
        }

        // المفتاح على اليسار
        let sw = Rect::from_center_size(pos2(rect.min.x + 8. + 16., rect.center().y), vec2(32., 18.));
        let on_t = ui.ctx().animate_bool_with_time(egui::Id::new(("row_switch", s.bit)), !blocked, 0.2);
        p.rect_filled(sw, 9., pal.panel_2.lerp_to_gamma(pal.accent_deep, on_t));
        p.rect_stroke(sw, 9., egui::Stroke::new(1., pal.line_2.lerp_to_gamma(pal.accent_deep, on_t)), egui::StrokeKind::Inside);
        // المفتاح يتحرك نحو نهاية السطر (اليسار) عند التفعيل
        let knob_x = sw.max.x - 9. - on_t * 14.;
        p.circle_filled(pos2(knob_x, sw.center().y), 6., pal.faint.lerp_to_gamma(Color32::WHITE, on_t));
        if pending {
            p.circle_stroke(pos2(knob_x, sw.center().y), 8., egui::Stroke::new(1.5, pal.gold));
        }

        // الرقم بجانب المفتاح
        let ms_x = sw.max.x + 10.;
        let ms_text = match ms {
            Some(v) => format!("{v:.0}"),
            None => "—".to_owned(),
        };
        let r1 = p.text(pos2(ms_x, rect.center().y - 1.), egui::Align2::LEFT_CENTER, ms_text, font(13.), pal.text.gamma_multiply(alpha));
        p.text(pos2(r1.max.x + 3., rect.center().y + 3.), egui::Align2::LEFT_CENTER, "ms", font(9.), pal.faint);

        // الاسم + الرمز + الشريط في المنتصف (محاذاة يمين)
        let name_right = flag_rect.min.x - 10.;
        let name_left = r1.max.x + 26.;
        let name = p.layout_no_wrap(s.title.clone(), font(13.), pal.text.gamma_multiply(alpha));
        let name_pos = pos2(name_right - name.size().x, rect.min.y + 7.);
        p.galley(name_pos, name.clone(), pal.text);
        if blocked {
            p.line_segment(
                [pos2(name_pos.x, name_pos.y + name.size().y / 2.), pos2(name_right, name_pos.y + name.size().y / 2.)],
                egui::Stroke::new(1., pal.red.gamma_multiply(0.8)),
            );
        }
        p.text(pos2(name_pos.x - 6., rect.min.y + 7. + name.size().y / 2.), egui::Align2::RIGHT_CENTER, s.token.to_ascii_uppercase(), font(9.), pal.faint);
        // شريط البنق
        let bar = Rect::from_min_max(pos2(name_left, rect.max.y - 11.), pos2(name_right, rect.max.y - 8.));
        p.rect_filled(bar, 2., pal.panel_2);
        if let Some(v) = ms {
            let w = bar.width() * (v / max_ms).clamp(0.05, 1.);
            p.rect_filled(Rect::from_min_max(pos2(bar.max.x - w, bar.min.y), bar.max), 2., grade_color(pal, grade(v)).gamma_multiply(alpha));
        }

        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text_at_pointer(if pending {
            format!("{} (بانتظار إغلاق اللعبة..)", s.token)
        } else if blocked {
            format!("{} (محظور)", s.token)
        } else {
            s.token.clone()
        });
        let click = if resp.clicked() {
            Some(false)
        } else if resp.secondary_clicked() {
            Some(true)
        } else {
            None
        };
        (click, resp.hovered())
    }

    fn route_card(&mut self, ui: &mut egui::Ui, rect: Rect, pal: &Palette) {
        let mut child = Self::glass(ui, rect, pal, true);
        let ui = &mut child;
        let best = self.get_most_likely_to_play_on().cloned();
        let ms = best.as_ref().and_then(|s| self.pings.get(&s.ping).and_then(|r| r.as_ref().ok().copied()));

        ui.label(egui::RichText::new("أفضل مسار").color(pal.gold).size(11.).strong());
        ui.add_space(2.);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("أنت").color(pal.muted).size(12.));
                // ثلاث نقاط تومض
                let t = ui.input(|i| i.time) as f32;
                let (r, _) = ui.allocate_exact_size(vec2(26., 10.), egui::Sense::hover());
                for k in 0..3 {
                    let a = if cfg!(feature = "animations") { 0.3 + 0.7 * ((t * 2.5 - k as f32 * 0.6).sin() * 0.5 + 0.5) } else { 1. };
                    ui.painter().circle_filled(pos2(r.max.x - 4. - k as f32 * 9., r.center().y), 3., pal.gold.gamma_multiply(a));
                }
                if let Some(b) = &best {
                    if let Some(src) = server_geo(b).and_then(|(_, f)| assets::flag(f)) {
                        ui.add(egui::Image::new(src).fit_to_exact_size(vec2(24., 18.)).corner_radius(3.));
                    }
                    ui.label(egui::RichText::new(&b.title).color(pal.gold).size(16.).strong());
                } else {
                    ui.label(egui::RichText::new("—").color(pal.faint).size(16.));
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(ms.map_or("--".to_owned(), |v| format!("{v:.0}"))).size(22.).strong());
                    ui.label(egui::RichText::new(format!("ms · {}", best.as_ref().map_or("", |b| b.token.as_str()))).color(pal.faint).size(10.));
                });
            });
        });
        ui.add_space(4.);
        // دائم / أثناء التشغيل
        let before = self.config.wfp_dynamic_session;
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                segmented(ui, pal, &[("دائم", false), ("أثناء التشغيل", true)], &mut self.config.wfp_dynamic_session);
            });
        });
        if self.config.wfp_dynamic_session != before {
            self.apply_wfp_session();
        }
    }

    /// لوحة العروض الأخرى (الألعاب، الأخبار، السجل، المساعدة، الخيارات)
    fn panel_view(&mut self, ui: &mut egui::Ui, rect: Rect, pal: &Palette, frame: &mut eframe::Frame) {
        let mut child = Self::glass(ui, rect, pal, false);
        let ui = &mut child;
        let title = match self.tab {
            VIEW_GAMES => "الألعاب",
            VIEW_NEWS => "الأخبار",
            VIEW_LOG => "السجل",
            VIEW_HELP => "المساعدة",
            _ => "الخيارات",
        };
        ui.label(egui::RichText::new(title).strong().size(15.));
        ui.add_space(6.);
        egui::ScrollArea::vertical()
            .id_salt(self.tab)
            .stick_to_bottom(self.tab == VIEW_LOG)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| match self.tab {
                    VIEW_GAMES => {
                        let games_top = ui.cursor().min;
                        ui.label(egui::RichText::new(match self.config.known_paths.as_ref().map_or(0, |x| x.len()) {
                            0 => "ما أضفت أي لعبة",
                            1 => "هذه اللعبة",
                            _ => "هذه الألعاب",
                        }).color(pal.muted));
                        self.applications(ui);
                        let r = Rect::from_min_max(games_top, pos2(ui.max_rect().max.x, ui.cursor().min.y));
                        self.tour_mark("games", r);
                    }
                    VIEW_NEWS => self.notice(ui),
                    VIEW_LOG => self.log(ui),
                    VIEW_HELP => self.help_wizard(ui),
                    _ => self.options(ui, frame),
                });
            });
    }

    // ------------------------------------------------------------ الإطار

    fn top_bar(&mut self, ui: &mut egui::Ui, rect: Rect, pal: &Palette) {
        let p = ui.painter();
        p.rect_filled(rect, 0., pal.panel);
        p.line_segment([rect.left_bottom(), rect.right_bottom()], egui::Stroke::new(1., pal.line));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(vec2(14., 0.))).layout(egui::Layout::right_to_left(egui::Align::Center)));
        let ui = &mut child;
        ui.spacing_mut().item_spacing.x = 10.;

        // الشعار
        let (lr, _) = ui.allocate_exact_size(vec2(28., 28.), egui::Sense::hover());
        ui.painter().rect_filled(lr, 9., pal.accent_deep);
        egui::Image::new(egui::include_image!("../assets/white-bolts.png")).paint_at(ui, lr.shrink(6.));
        ui.label(egui::RichText::new("dropship").strong().size(15.));
        pill(ui, &format!("v{} · النسخة العربية", env!("CARGO_PKG_VERSION")), pal.muted, Color32::TRANSPARENT);

        // الرقائق في المنتصف
        let blocked_names: Vec<String> = self
            .known_servers()
            .iter()
            .filter(|s| self.config.desired_blocked_servers.has(s))
            .map(|s| s.title.clone())
            .collect();
        let blocked_n = blocked_names.len();
        // «شغّال» فقط لو ما يريده المستخدم مطبَّق فعلًا في الجدار الناري
        let applied = self.config.desired_blocked_servers.bits() == self.config.blocked_servers.bits();
        let state = match (applied, blocked_n > 0, self.game_open) {
            (true, true, _) => "شغّال",
            (true, false, _) => "متوقف",
            (false, _, true) => "بانتظار إغلاق اللعبة",
            (false, _, false) => "لم يُطبَّق",
        };
        let filter_text = format!(
            "الفلتر · {} · {}",
            state,
            match blocked_n {
                0 => "بدون حظر".to_owned(),
                1 => "سيرفر محظور".to_owned(),
                2 => "سيرفران محظوران".to_owned(),
                n => format!("{n} محظورة"),
            }
        );
        let game_text = format!("اللعبة · {}", if self.game_open { "مكتشفة" } else { "مغلقة" });
        ui.add_space(24.);
        let c1 = chip(ui, pal, &filter_text, if !applied { Some(pal.gold) } else if blocked_n > 0 { Some(pal.accent) } else { None });
        let c2 = chip(ui, pal, &game_text, if self.game_open { Some(pal.gold) } else { None });
        self.tour_mark("stars", c1.union(c2));
        if !blocked_names.is_empty() {
            let _ = ui
                .interact(c1, egui::Id::new("chip_filter"), egui::Sense::hover())
                .on_hover_text_at_pointer(blocked_names.join("، "));
        }
    }

    fn rail(&mut self, ui: &mut egui::Ui, rect: Rect, pal: &Palette) {
        let p = ui.painter();
        p.rect_filled(rect, 0., pal.panel);
        p.line_segment([rect.left_top(), rect.left_bottom()], egui::Stroke::new(1., pal.line));

        let items: [(usize, egui::ImageSource<'static>, &str, &str); 6] = [
            (VIEW_MAP, assets::ICON_MAP, "الخريطة", "tab_map"),
            (VIEW_GAMES, assets::ICON_GAMEPAD, "الألعاب", "tab_games"),
            (VIEW_NEWS, assets::ICON_NEWS, "الأخبار", "tab_notices"),
            (VIEW_LOG, assets::ICON_TERMINAL, "السجل", "tab_log"),
            (VIEW_HELP, assets::ICON_HEART, "المساعدة", "tab_help"),
            (VIEW_OPTIONS, assets::ICON_GEARS, "الخيارات", "tab_options"),
        ];
        let mut y = rect.min.y + 12.;
        for (view, icon, name, key) in items {
            let r = Rect::from_min_size(pos2(rect.center().x - 20., y), vec2(40., 40.));
            let active = self.tab == view;
            let resp = rail_button(ui, r, pal, icon, active, name);
            self.tour_mark(key, r);
            if resp.clicked() {
                self.tab = view;
            }
            y += 46.;
        }

        // في الأسفل: المظهر
        let r = Rect::from_min_size(pos2(rect.center().x - 20., rect.max.y - 12. - 40.), vec2(40., 40.));
        let dark = self.get_theme(ui) == visuals::Theme::Dark;
        let resp = rail_button(ui, r, pal, if dark { assets::ICON_SUN } else { assets::ICON_MOON }, false, "المظهر");
        if resp.clicked() {
            self.config.theme = Some(if dark { visuals::Theme::Light } else { visuals::Theme::Dark });
            self.apply_theme(ui.ctx());
        }
    }

    fn footer(&mut self, ui: &mut egui::Ui, rect: Rect, pal: &Palette) {
        let p = ui.painter();
        p.rect_filled(rect, 0., pal.panel);
        p.line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(1., pal.line));
        self.tour_mark("footer", rect);

        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(vec2(14., 0.))).layout(egui::Layout::right_to_left(egui::Align::Center)));
        let ui = &mut child;
        ui.spacing_mut().item_spacing.x = 6.;
        ui.style_mut().visuals.hyperlink_color = pal.muted;

        // الاعتمادات (يمين)
        ui.label(egui::RichText::new("البرنامج الأصلي من").color(pal.faint).size(11.));
        ui.hyperlink_to(egui::RichText::new("stormy").size(11.), dropship::UPSTREAM_GITHUB_URI)
            .on_hover_text_at_pointer(dropship::UPSTREAM_GITHUB_URI);
        ui.label(egui::RichText::new("· النسخة العربية من").color(pal.faint).size(11.));
        ui.hyperlink_to(egui::RichText::new("Ryanathlawi").size(11.), dropship::GITHUB_URI)
            .on_hover_text_at_pointer(dropship::GITHUB_URI);

        // الاختصارات (يسار)
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            let key = |ui: &mut egui::Ui, k: &str, label: &str| {
                let (r, _) = ui.allocate_exact_size(vec2(30., 20.), egui::Sense::hover());
                ui.painter().rect_filled(r, 6., pal.panel_2);
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, k, font(10.), pal.text);
                ui.label(egui::RichText::new(label).color(pal.faint).size(11.));
                ui.add_space(6.);
            };
            key(ui, "esc", "إغلاق");
            key(ui, "L", "تبديل");
            key(ui, "R", "عكس الباقي");
            if let Some(k) = self.config.presets.iter().find_map(|p| p.key) {
                key(ui, k.name(), "اختصار");
            }

            // رسالة الحالة في المنتصف
            ui.add_space(10.);
            let r0 = ui.cursor().min;
            self.stat(ui);
            self.tour_mark("status", Rect::from_min_max(r0, pos2(ui.cursor().min.x.max(r0.x + 120.), rect.max.y)));
        });
    }
}

// ---------------------------------------------------------------- عناصر صغيرة

fn pill(ui: &mut egui::Ui, text: &str, color: Color32, fill: Color32) -> Rect {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(11.), color);
    let (r, _) = ui.allocate_exact_size(galley.size() + vec2(16., 8.), egui::Sense::hover());
    ui.painter().rect_filled(r, 999., fill);
    ui.painter().rect_stroke(r, 999., egui::Stroke::new(1., ui.visuals().widgets.noninteractive.fg_stroke.color.gamma_multiply(0.15)), egui::StrokeKind::Inside);
    ui.painter().galley(r.center() - galley.size() / 2., galley, color);
    r
}

fn chip(ui: &mut egui::Ui, pal: &Palette, text: &str, led: Option<Color32>) -> Rect {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(12.), if led.is_some() { pal.text } else { pal.muted });
    let (r, _) = ui.allocate_exact_size(galley.size() + vec2(34., 10.), egui::Sense::hover());
    ui.painter().rect_filled(r, 999., pal.panel_2);
    let stroke = led.map_or(pal.line_2, |c| c.gamma_multiply(0.5));
    ui.painter().rect_stroke(r, 999., egui::Stroke::new(1., stroke), egui::StrokeKind::Inside);
    // المؤشر على يمين النص (بداية السطر)
    let dot = pos2(r.max.x - 13., r.center().y);
    match led {
        Some(c) => {
            let t = ui.input(|i| i.time) as f32;
            let a = if cfg!(feature = "animations") { 0.55 + 0.45 * (t * 4.).sin().abs() } else { 1. };
            ui.painter().circle_filled(dot, 3.5, c.gamma_multiply(a));
            ui.painter().circle_filled(dot, 7., c.gamma_multiply(0.18 * a));
        }
        None => {
            ui.painter().circle_filled(dot, 3.5, pal.faint);
        }
    }
    ui.painter().galley(pos2(r.min.x + 10., r.center().y - galley.size().y / 2.), galley, pal.text);
    r
}

fn rail_button(ui: &mut egui::Ui, rect: Rect, pal: &Palette, icon: egui::ImageSource<'_>, active: bool, tooltip: &str) -> egui::Response {
    let resp = ui.interact(rect, egui::Id::new(("rail", tooltip)), egui::Sense::click());
    let t = ui.ctx().animate_bool_with_time(egui::Id::new(("rail_active", tooltip)), active, 0.18);
    let h = ui.ctx().animate_bool_with_time(egui::Id::new(("rail_hover", tooltip)), resp.hovered(), 0.12);
    let p = ui.painter();
    if t > 0. {
        p.add(egui::epaint::Shadow { offset: [0, 8], blur: 20, spread: 0, color: pal.accent.gamma_multiply(0.35 * t) }.as_shape(rect, 12.));
        p.rect_filled(rect, 12., pal.accent_deep.gamma_multiply(t));
    }
    if h > 0. && t < 1. {
        p.rect_filled(rect, 12., pal.panel_2.gamma_multiply(h * (1. - t)));
    }
    let color = pal.muted.lerp_to_gamma(Color32::WHITE, t.max(h * 0.6));
    egui::Image::new(icon).fit_to_exact_size(vec2(18., 18.)).tint(color).paint_at(ui, Rect::from_center_size(rect.center(), vec2(18., 18.)));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text_at_pointer(tooltip)
}

/// مفتاح مجزّأ: خيارات تُختار واحدة منها
fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, pal: &Palette, options: &[(&str, T)], value: &mut T) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 2.;
        for (label, v) in options {
            let active = *value == *v;
            let galley = ui.painter().layout_no_wrap(label.to_string(), font(11.), if active { Color32::WHITE } else { pal.muted });
            let (r, resp) = ui.allocate_exact_size(galley.size() + vec2(20., 8.), egui::Sense::click());
            let t = ui.ctx().animate_bool_with_time(egui::Id::new(("seg", *label)), active, 0.15);
            ui.painter().rect_filled(r, 7., pal.panel_2.lerp_to_gamma(pal.accent_deep, t));
            ui.painter().galley(r.center() - galley.size() / 2., galley, pal.text);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *value = *v;
            }
        }
    });
}
