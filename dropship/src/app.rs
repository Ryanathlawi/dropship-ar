use crate::{
    api::{self, KnownServer},
    assets, components,
    dropship::{self, startup_dispatch},
    firewall::{self, applications::ApplicationType},
    logger,
    overwatch::ServerSelection,
    visuals,
};
use eframe::egui;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, atomic},
};
use tokio::sync::{
    Mutex,
    mpsc::{self, UnboundedReceiver, UnboundedSender},
};

use crate::{launcher, update};

/* todo

    [ ] make sure the svgs i downloaded all have 1:1 viewbox 640x640. in chromium

    [ ] log in ui, removing mina rules etc

    [ ] browse memory region for server strings? might have ip addrs nearby
    [ ] sort by ping button

    [ ] detect windows firewall disabled or external firewall software

    [ ] make sure it works offline, managing servers etc

    [ ] automatically get dacom kr? people have been having issues

    [ ] custom rules?
    [ ] fetch gpc manually button/background task?

    [ ] flush dns cache button?
        some users report unblocking but remaining at high ping to their closest server

    [ ] rotating tip messages. (like m2 to play on a server)
    [ ] "most likely to play on" message

    [ ] log all fs paths and operations

    [ ] enforce only one instance at a time (and fix updater so it works after this change)
*/

const CACHE_KEY: &str = "cache";

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DropshipConfig {
    zoom: f32,
    welcomed: bool,
    always_show_welcome: bool,
    pub desired_blocked_servers: ServerSelection,
    pub(crate) blocked_servers: ServerSelection,
    pub known_paths: Option<HashSet<PathBuf>>,
    starting_tab: usize,
    pub(crate) theme: Option<visuals::Theme>, // none is system theme
    pub(crate) mini: bool,
    pub(crate) wfp_dynamic_session: bool, // do wfp blocks only apply when dropship is open?
    //
    pub(crate) disable_background_image: bool,
    /// هل شاهد المستخدم الجولة التعريفية؟
    toured: bool,
    /// ترتيب قائمة السيرفرات حسب البنق
    pub(crate) sort_by_ping: bool,
    /// اختصارات الحظر (زر + مفتاح اختياري)
    pub(crate) presets: Vec<launcher::Preset>,
}

impl Default for DropshipConfig {
    fn default() -> Self {
        Self {
            zoom: 1.,
            welcomed: false,
            always_show_welcome: false,
            desired_blocked_servers: ServerSelection::none(),
            blocked_servers: ServerSelection::none(),
            known_paths: None,
            starting_tab: 0,
            theme: None,
            mini: false,
            wfp_dynamic_session: false,
            //
            disable_background_image: false,
            toured: false,
            sort_by_ping: false,
            presets: launcher::default_presets(),
        }
    }
}

const TAB_LOG: usize = launcher::VIEW_LOG;

/// تلاشي + انزلاق أفقي للمحتوى منذ لحظة `since` (بثواني egui). يعيد الرسم حتى يكتمل.
/// `dx` مسافة البداية بالنقاط (موجب = يبدأ من اليمين). لا يكلّف شيئًا بعد اكتماله.
fn slide_in(ui: &mut egui::Ui, since: f64, dx: f32, add: impl FnOnce(&mut egui::Ui)) {
    if !cfg!(feature = "animations") {
        add(ui);
        return;
    }
    let t = ((ui.input(|i| i.time) - since) / 0.22).clamp(0., 1.) as f32;
    let e = 1. - (1. - t).powi(3); // cubic out
    if t < 1. {
        ui.ctx().request_repaint();
    }
    // إزاحة مرئية فقط (لا تغيّر التخطيط) حتى لا يتغيّر حجم النافذة أثناء الحركة
    let shift = egui::emath::TSTransform::from_translation(egui::vec2(dx * (1. - e), 0.));
    ui.with_visual_transform(shift, |ui| {
        ui.set_opacity(e);
        add(ui);
    });
}

/// صف أفقي من اليمين لليسار. ملفوف بـ `horizontal` لأن `with_layout` وحده يتمدد رأسيًا
/// (ويُوسّط محتواه) داخل الحاويات غير المحدودة الارتفاع مثل Area وScrollArea.
fn rtl_row<R>(
    ui: &mut egui::Ui,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add)
            .inner
    })
}

pub struct TemplateApp {
    //
    pub(crate) commands_tx: UnboundedSender<dropship::Command>,
    pub(crate) events_rx: UnboundedReceiver<dropship::Event>,
    pub(crate) logs_rx: mpsc::UnboundedReceiver<logger::Message>,
    pub(crate) config: DropshipConfig,
    pub(crate) cache: Option<ApiCache>,

    //
    download_total_size: Arc<atomic::AtomicU64>,
    downloaded_size: Arc<atomic::AtomicU64>,
    pub(crate) game_open: bool,
    pub(crate) installing_status: update::UpdatingStatus,
    // pub(crate) known_applications: Option<Vec<applications::Application>>,
    // pub(crate) known_applications: Option<HashSet<PathBuf>>,
    pub(crate) logs: Vec<logger::Message>,
    pub(crate) pings: HashMap<String, Result<f32, String>>,
    pub(crate) update_available: Option<update::AvailableUpdate>,

    //
    export_ips_modal: bool,
    pub(crate) preset_editor: Option<launcher::PresetEditor>,
    modal_manage_path: Option<PathBuf>,
    hide_update: bool,
    modal_welcome_page: Option<u8>,
    restart_requested: bool,
    /// العرض الحالي في الشريط الجانبي (انظر `launcher::VIEW_*`)
    pub(crate) tab: usize,
    pub(crate) loading: bool,
    pub(crate) pending_firewall_sync_when_game_is_closed: bool,
    pub(crate) legacy_cleanup_done: bool,
    // pub(crate) cached_lowest_ping_server: Option<KnownServer>,
    prev_system_theme: Option<egui::Theme>, //

    //
    pub(crate) suggesting_path: Option<PathBuf>,
    pub(crate) denied_paths: HashSet<PathBuf>,

    //
    wfp_connection: Arc<Mutex<Option<firewall::win::WfpConnection>>>,

    /// الجولة التعريفية: الخطوة الحالية
    pub(crate) tour: Option<usize>,

    // الواجهة الجديدة
    /// السيرفر الذي يمرّ عليه المؤشر (في الخريطة أو القائمة)
    pub(crate) hover_server: Option<u8>,
    /// موقع المستخدم التقريبي من إعدادات ويندوز
    pub(crate) user_geo: Option<launcher::Geo>,
    /// نسيج خريطة العالم المنقّطة، لكل مظهر
    pub(crate) world_tex: Option<(visuals::Theme, egui::TextureHandle)>,

    // أنميشن: لحظة آخر تغيير (بثواني egui) لكل عنصر يتلاشى/ينزلق
    prev_tab: usize,
    tab_changed_at: f64,
    prev_tour: Option<usize>,
    tour_changed_at: f64,
    prev_welcome: Option<u8>,
    welcome_changed_at: f64,
    welcome_dir: f32,
    /// مواقع العناصر التي تشير إليها الجولة، تُحدّث كل إطار
    tour_rects: HashMap<&'static str, egui::Rect>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
pub struct ApiCache {
    pub cached_api_data: Option<api::DropshipApiData>,
}

impl TemplateApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        logs_rx: mpsc::UnboundedReceiver<logger::Message>,
    ) -> Self {
        cc.egui_ctx.set_fonts(crate::visuals::fonts());

        // if let Some(c) = egui::ViewportCommand::center_on_screen(&cc.egui_ctx) {
        //     cc.egui_ctx.send_viewport_cmd(c);
        // }

        // load previous app state
        let (config, cache) = {
            if let Some(storage) = cc.storage {
                let config = if let Some(config) =
                    eframe::get_value::<DropshipConfig>(storage, eframe::APP_KEY)
                {
                    config
                } else {
                    log::warn!("فشل قراءة إعدادات dropship");
                    // log::warn!("didn't find a valid dropship config file");
                    Default::default()
                };

                // failable cache
                let cache = if let Some(value) =
                    eframe::get_value::<Option<ApiCache>>(storage, CACHE_KEY)
                {
                    value
                } else {
                    log::warn!("فشل تحميل البيانات المخزنة");
                    None
                };

                (config, cache)
            } else {
                log::warn!("ما وُجد ملف إعدادات سابق");
                Default::default()
            }
        };

        {
            let size = egui::vec2(crate::APP_WIDTH, crate::APP_HEIGHT);
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            cc.egui_ctx.set_zoom_factor(config.zoom);
        }

        let (commands_tx, commands_rx) = mpsc::unbounded_channel::<dropship::Command>();
        let (events_tx, events_rx) = mpsc::unbounded_channel::<dropship::Event>();

        let wfp_connection = Arc::new(Mutex::new(None));

        dropship::start_processing_commands(
            commands_rx,
            events_tx,
            Some(cc.egui_ctx.clone()),
            wfp_connection.clone(),
        );

        startup_dispatch(&commands_tx, &cache);

        let mut app = Self {
            //
            commands_tx,
            events_rx,
            logs_rx,
            cache,
            config,

            //
            download_total_size: Arc::new(atomic::AtomicU64::new(0)),
            downloaded_size: Arc::new(atomic::AtomicU64::new(0)),
            game_open: false,
            installing_status: update::UpdatingStatus::NotActive,
            logs: vec![],
            pings: HashMap::new(),
            update_available: None,

            //
            export_ips_modal: false,
            preset_editor: None,
            modal_manage_path: None,
            hide_update: false,
            modal_welcome_page: None,
            restart_requested: false,
            tab: 0,
            loading: false,
            pending_firewall_sync_when_game_is_closed: false,
            legacy_cleanup_done: false,
            // cached_lowest_ping_server: None,
            prev_system_theme: cc.egui_ctx.system_theme(),
            //
            suggesting_path: None,
            denied_paths: HashSet::default(),

            //
            wfp_connection,

            tour: None,
            tour_rects: HashMap::new(),
            hover_server: None,
            user_geo: launcher::user_geo(),
            world_tex: None,

            prev_tab: 0,
            tab_changed_at: 0.,
            prev_tour: None,
            tour_changed_at: 0.,
            prev_welcome: None,
            welcome_changed_at: 0.,
            welcome_dir: 1.,
        };

        {
            if !app.config.welcomed || app.config.always_show_welcome {
                app.modal_welcome_page = Some(0);
            } else if !app.config.toured {
                // مستخدم قديم (إعدادات النسخة الأصلية) لم يرَ الجولة بعد
                app.start_tour(&cc.egui_ctx);
            }

            app.tab = app.config.starting_tab.min(launcher::VIEW_OPTIONS);

            if app.config.wfp_dynamic_session {
                app.config.blocked_servers = ServerSelection::none();
            }
        }

        app.apply_theme(&cc.egui_ctx);
        app.apply_wfp_session();
        app.apply_mini_mode(&cc.egui_ctx);

        app
    }

    pub fn known_servers(&self) -> &[api::KnownServer] {
        if let Some(cache) = &self.cache {
            if let Some(data) = &cache.cached_api_data {
                return data.servers.overwatch.as_slice();
            }
        }
        &[]
    }

    fn apply_zoom(&mut self, ui: &egui::Context, zoom: f32) {
        ui.set_zoom_factor(zoom);
        self.config.zoom = zoom;

        let active = ui.zoom_factor();

        let structural_adjustment = zoom / active;
        let size = egui::vec2(
            crate::APP_WIDTH * structural_adjustment,
            crate::APP_HEIGHT * structural_adjustment,
        );
        ui.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
    }

    // prev frame values, without need for ctx
    fn _get_theme(&self) -> visuals::Theme {
        let theme = {
            if let Some(t) = self.config.theme {
                t
            } else {
                match self.prev_system_theme {
                    Some(t) => match t {
                        egui::Theme::Dark => visuals::Theme::Dark,
                        egui::Theme::Light => visuals::Theme::Light,
                    },
                    None => visuals::Theme::default(),
                }
            }
        };

        theme
    }

    pub(crate) fn get_theme(&self, ctx: &egui::Context) -> visuals::Theme {
        let theme = {
            if let Some(t) = self.config.theme {
                t
            } else {
                match ctx.system_theme() {
                    Some(t) => match t {
                        egui::Theme::Dark => visuals::Theme::Dark,
                        egui::Theme::Light => visuals::Theme::Light,
                    },
                    None => visuals::Theme::default(),
                }
            }
        };

        theme
    }

    pub(crate) fn apply_theme(&mut self, ctx: &egui::Context) {
        let theme = self.get_theme(ctx);

        ctx.all_styles_mut(move |style| crate::visuals::visuals(style, theme));
    }

    pub(crate) fn apply_wfp_session(&mut self) {
        let wfp_connection = self.wfp_connection.clone();
        let dynamic = self.config.wfp_dynamic_session;
        let commands_tx = self.commands_tx.clone();

        // tokio::task::spawn_blocking(move || {
        tokio::spawn(async move {
            //
            // wipe persistent filters for safety when in dynamic mode
            if dynamic {
                match firewall::win::WfpConnection::new(true) {
                    Ok(mut w) => {
                        match (|| -> std::io::Result<()> {
                            let transaction = wfp::Transaction::new(&mut w.handle)?;
                            firewall::win::delete_dropship_wfp(&transaction)?;
                            transaction.commit()?;
                            Ok(())
                        })() {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("فشل تنظيف بيانات WFP الدائمة، {}", e.to_string());
                            }
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "فشل الاتصال بـ WFP لتنظيف البيانات الدائمة، {}",
                            e.to_string()
                        );
                    }
                }
            }

            let mut guard = wfp_connection.lock().await;

            *guard = {
                match firewall::win::WfpConnection::new(!dynamic) {
                    Ok(w) => {
                        log::debug!("تم الاتصال بـ WFP. مؤقت: {dynamic}");
                        Some(w)
                    }
                    Err(e) => {
                        log::error!("فشل الاتصال بـ WFP ({})", e.to_string());
                        None
                    }
                }
            };

            let _ = commands_tx.send(dropship::Command::ForceApplyFirewallRequested);
        });
    }

    /// يبدأ الجولة التعريفية من أولها (يتطلب الوضع الكامل)
    fn start_tour(&mut self, ctx: &egui::Context) {
        self.config.mini = false;
        self.apply_mini_mode(ctx);
        self.tour = Some(0);
    }

    /// يسجّل موقع عنصر لتشير إليه الجولة التعريفية
    pub(crate) fn tour_mark(&mut self, key: &'static str, rect: egui::Rect) {
        self.tour_rects.insert(key, rect);
    }

    pub(crate) fn apply_mini_mode(&self, ctx: &egui::Context) {
        if self.config.mini {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                [crate::APP_MINI_WIDTH, crate::APP_HEIGHT].into(),
            ));
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                [crate::APP_WIDTH, crate::APP_HEIGHT].into(),
            ));
        }
    }
}

pub const HERO_BG_SIZE: egui::Vec2 = egui::vec2(1920.0, 885.0);

impl eframe::App for TemplateApp {
    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.config);

        // failable cache
        eframe::set_value(storage, CACHE_KEY, &self.cache);

        if self.restart_requested {
            log::info!("طُلبت إعادة التشغيل");

            if let Ok(installed_binary_path) = std::env::current_exe() {
                std::process::Command::new(installed_binary_path)
                    .spawn()
                    .map_err(|e| {
                        log::error!("{}", e);
                        e
                    })
                    .ok();
            }
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // egui::Color32::from_rgba_unmultiplied(193, 197, 209, 255).to_normalized_gamma_f32()

        visuals::palette(self._get_theme()).bg.to_normalized_gamma_f32()
    }

    // happens before every ui()
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // process events
        dropship::process_events(Some(ctx.clone()), self);

        // when using system theme, check for changes
        if self.config.theme.is_none()
            && let Some(system_theme) = ctx.system_theme()
        {
            if self.prev_system_theme != Some(system_theme) {
                log::debug!("تغيّر مظهر الجهاز");
                self.apply_theme(ctx);
            }
        }

        // cache previous frame's theme
        self.prev_system_theme = ctx.system_theme();
    }

    /// called each time the ui needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let esc_pressed: bool = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));

        // أنميشن: سجّل لحظة تغيّر التبويب/الجولة/صفحة الترحيب
        {
            let now = ui.input(|i| i.time);
            if self.tab != self.prev_tab {
                self.prev_tab = self.tab;
                self.tab_changed_at = now;
            }
            if self.tour != self.prev_tour {
                self.prev_tour = self.tour;
                self.tour_changed_at = now;
            }
            if self.modal_welcome_page != self.prev_welcome {
                // للأمام: تدخل من اليسار، للخلف: من اليمين
                self.welcome_dir = match (self.prev_welcome, self.modal_welcome_page) {
                    (Some(a), Some(b)) if b < a => 1.,
                    _ => -1.,
                };
                self.prev_welcome = self.modal_welcome_page;
                self.welcome_changed_at = now;
            }
        }

        if esc_pressed && self.tour.is_some() {
            // Esc أثناء الجولة يتخطاها فقط
            self.tour = None;
            self.config.toured = true;
        } else if esc_pressed
            && !ui.any_popup_open()
            && !self.export_ips_modal
            && self.preset_editor.is_none()
            && self.modal_welcome_page.is_none()
            && !self.should_show_update_modal()
            && self.modal_manage_path.is_none()
        {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // اختصارات لوحة المفاتيح: مفاتيح الاختصارات المحفوظة أولًا، ثم 1..4 للأخبار/السجل/المساعدة/الخيارات
        // وM للخريطة (معطّلة أثناء الجولة ونافذة الاختصار وأثناء الكتابة في حقل)
        if !ui.ctx().text_edit_focused() && self.tour.is_none() && self.preset_editor.is_none() {
            let preset = ui.ctx().input(|i| self.config.presets.iter().position(|p| p.key.is_some_and(|k| i.key_pressed(k))));
            if let Some(p) = preset {
                self.apply_preset(p);
            }
            ui.ctx().input(|i| {
                if i.key_pressed(egui::Key::Num1) {
                    self.tab = launcher::VIEW_NEWS;
                } else if i.key_pressed(egui::Key::Num2) {
                    self.tab = launcher::VIEW_LOG;
                } else if i.key_pressed(egui::Key::Num3) {
                    self.tab = launcher::VIEW_HELP;
                } else if i.key_pressed(egui::Key::Num4) {
                    self.tab = launcher::VIEW_OPTIONS;
                } else if i.key_pressed(egui::Key::M) {
                    self.tab = launcher::VIEW_MAP;
                }
            });
        }

        self.launcher_ui(ui, frame);

        if let Some(page) = self.modal_welcome_page {
            self.welcome(ui, page);
        }

        self.updater(ui);

        if self.suggesting_path.is_some() {
            self.suggest_path(ui);
        }

        if let Some(step) = self.tour {
            self.tour_overlay(ui, step);
        }

        #[cfg(debug_assertions)]
        // testing
        {
            // ui.set_debug_on_hover(true);

            // let r = egui::Rect::from_min_max(egui::pos2(563.3, 425.3), egui::pos2(641.3, 448.7));
            // ui.ctx().debug_painter().rect_stroke(
            //     r,
            //     0.0,
            //     (2.0, egui::Color32::RED),
            //     egui::StrokeKind::Middle,
            // );
        }

        #[cfg(debug_assertions)]
        if ui.button("force apply").clicked() {
            self._force_apply_blocked_servers_to_firewall();
        }
    }
}

impl TemplateApp {
    fn _apply_blocked_servers_to_firewall(
        blocked_servers: &ServerSelection,
        known_servers: &[KnownServer],
        already_known_paths: &Option<HashSet<PathBuf>>,
        commands_tx: &UnboundedSender<dropship::Command>,
    ) {
        let blocked_servers = known_servers
            .iter()
            .filter(|x| blocked_servers.has(x))
            .map(|x| x.clone())
            .collect();

        let already_known_paths = already_known_paths.clone().unwrap_or_default();

        let _ = commands_tx.send(dropship::Command::ApplyFirewallConfig {
            blocked_servers,
            already_known_paths,
        });
    }

    pub fn _force_apply_blocked_servers_to_firewall(&mut self) {
        Self::_apply_blocked_servers_to_firewall(
            &self.config.desired_blocked_servers,
            self.known_servers(),
            &self.config.known_paths,
            &self.commands_tx,
        );
    }

    pub fn apply_blocked_servers_to_firewall(&mut self) {
        if self.game_open {
            log::warn!("أغلق اللعبة لتطبيق التغييرات");
            self.pending_firewall_sync_when_game_is_closed = true;
        } else {
            self._force_apply_blocked_servers_to_firewall();
        }
    }

    pub fn force_unblock_all(&mut self) {
        self.config.desired_blocked_servers = ServerSelection::none();

        Self::_apply_blocked_servers_to_firewall(
            &self.config.desired_blocked_servers,
            self.known_servers(),
            &self.config.known_paths,
            &self.commands_tx,
        );
    }

    pub(crate) fn stat(&mut self, ui: &mut egui::Ui) {
        let mut widget = None;

        if !self.logs.is_empty() {
            let now = chrono::Local::now();
            let window = now - chrono::Duration::seconds(9);

            if let Some(error) = self
                .logs
                .iter()
                .rev()
                .find(|m| m.level == log::Level::Error && m.time >= window)
            {
                widget = Some(
                    egui::Label::new(
                        egui::RichText::new(&error.message).color(ui.visuals().error_fg_color),
                    )
                    .truncate(),
                );
            } else if let Some(warning) = self
                .logs
                .iter()
                .rev()
                .find(|m| m.level == log::Level::Warn && m.time >= window)
            {
                widget = Some(
                    egui::Label::new(
                        egui::RichText::new(&warning.message).color(ui.visuals().warn_fg_color),
                    )
                    .truncate(),
                )
            } else if let Some(message) = self.logs.iter().rev().find(|m| m.time >= window) {
                widget = Some(
                    egui::Label::new(
                        egui::RichText::new(
                            message.message.strip_prefix("[ event ] ").unwrap_or(
                                &message
                                    .message
                                    .strip_prefix("[command] ")
                                    .unwrap_or(&message.message),
                            ),
                        )
                        .color(ui.visuals().weak_text_color()),
                    )
                    .truncate(),
                )
            }
        }

        // تلاشي عند الظهور (ربع ثانية) وقبل الاختفاء (آخر ثانية من نافذة الـ 9 ثواني)
        if cfg!(feature = "animations")
            && let Some(m) = self.logs.iter().rev().find(|m| m.time >= chrono::Local::now() - chrono::Duration::seconds(9))
        {
            let age = (chrono::Local::now() - m.time).as_seconds_f32();
            let opacity = (age / 0.25).clamp(0., 1.) * ((9. - age) / 1.).clamp(0., 1.);
            ui.set_opacity(opacity);
            if age < 0.25 || age > 8. {
                ui.ctx().request_repaint();
            }
        }

        if let Some(widget) = widget {
            if self.tab != TAB_LOG {
                if ui
                    .add(widget)
                    .on_hover_cursor(
                        ui.style()
                            .visuals
                            .interact_cursor
                            .unwrap_or(egui::CursorIcon::PointingHand),
                    )
                    .clicked()
                {
                    self.tab = TAB_LOG;

                    self.config.mini = false;
                    self.apply_mini_mode(ui);
                }
            } else {
                ui.add(widget);
            }
        }

        // ui.label("=^.^=");
    }

    fn draw_path(path: &PathBuf, ui: &mut egui::Ui) -> bool {
        let mut clicked = false;

        // let p = path.to_string_lossy().to_lowercase();
        let p = path.display().to_string();
        let ty = {
            if p.contains("_retail_") {
                ApplicationType::Blizzard
            } else if p.contains("steamapps") {
                ApplicationType::Valve
            } else {
                ApplicationType::Unknown
            }
        };

        ui.horizontal(|ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.x = ui.style().spacing.item_spacing.y;
                rtl_row(ui, |ui| {
                    let image = match ty {
                        firewall::applications::ApplicationType::Blizzard => {
                            assets::COMPANY_ICON_BATTLENET
                        }
                        firewall::applications::ApplicationType::Valve => {
                            assets::COMPANY_ICON_STEAM
                        }
                        _ => assets::GAME_ICON_OVERWATCH,
                    };

                    let button = egui::Button::image_and_text(
                        egui::Image::new(image).fit_to_exact_size(egui::vec2(16.0, 16.0)),
                        &path.display().to_string(),
                        // REVIEW lower case here?
                    )
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .min_size(egui::vec2(
                        ui.available_width(),
                        ui.spacing().interact_size.y,
                    ))
                    .gap(8.);
                    clicked = ui.add(button).clicked();
                });
            });
        });

        clicked
    }

    pub fn get_most_likely_to_play_on(&self) -> Option<&KnownServer> {
        let mut lowest_ping_server = None;
        let mut lowest_ping = f32::INFINITY;

        for s in self.known_servers() {
            if self.config.blocked_servers.has(s) {
                continue;
            }

            if let Some(ping) = self.pings.get(&s.ping) {
                if let Ok(ping) = ping {
                    if *ping < lowest_ping {
                        lowest_ping = *ping;
                        lowest_ping_server = Some(s);
                    }
                }
            }
        }

        lowest_ping_server
    }

    pub(crate) fn applications(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            //

            match &self.config.known_paths {
                Some(paths) => {
                    egui::ScrollArea::vertical()
                        .content_margin(egui::Margin {
                            right: 4 + 8, // gap + width + margin
                            top: 0,
                            left: 0,
                            bottom: 0,
                        })
                        .auto_shrink([false, true])
                        .max_height(160.)
                        .show(ui, |ui| {
                            paths.into_iter().for_each(|path| {
                                if Self::draw_path(&path, ui) {
                                    self.modal_manage_path = Some(path.clone());
                                }
                            });
                        });
                }
                None => {
                    ui.spinner();
                }
            }

            let theme = self.get_theme(ui);

            // new
            ui.horizontal(|ui| {
                ui.scope(|ui| {
                    //

                    // if the list isn't empty, fade it
                    if !&self
                        .config
                        .known_paths
                        .as_ref()
                        .is_some_and(|x| x.is_empty())
                    {
                        ui.style_mut().visuals.widgets.inactive.weak_bg_fill =
                            visuals::from_theme_alpha(theme, 0);
                        ui.style_mut().visuals.widgets.active.weak_bg_fill =
                            visuals::from_theme_alpha(theme, 40);
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                            visuals::from_theme_alpha(theme, 20);

                        ui.style_mut().visuals.override_text_color =
                            Some(ui.style_mut().visuals.weak_text_color());
                    }

                    ui.spacing_mut().item_spacing.x = ui.style().spacing.item_spacing.y;
                    rtl_row(ui, |ui| {
                        // {
                        //     let icon_size = 16.;

                        //     let icon = egui::Image::new(ICON_PLUS)
                        //         .fit_to_exact_size(egui::vec2(icon_size, icon_size));

                        //     let button = egui::Button::image(icon);
                        //     if ui.add(button).clicked() {
                        //         self.display_modal = Some(DropshipModal::Options);
                        //     }
                        // }
                        {
                            let button = egui::Button::new("{{ أضف لعبة }}")
                                .min_size(egui::vec2(ui.available_width(), 24.0))
                                .gap(8.);

                            if ui.add(button).clicked() {
                                let commands_tx = self.commands_tx.clone();
                                tokio::spawn(async move {
                                    // REVIEW with windows api we can make sure it's exactly overwatch.exe
                                    // ofn.lpstrFilter = TEXT("Overwatch.exe\0Overwatch.exe\0");
                                    // i cannot with rfd it seems..

                                    let file = rfd::AsyncFileDialog::new()
                                        // .add_filter("Overwatch", &["exe"])
                                        .add_filter("Overwatch.exe", &["exe"])
                                        .set_directory("/")
                                        // .set_title("find overwatch.exe")
                                        // .set_file_name("Overwatch.exe")
                                        .pick_file()
                                        .await;

                                    if let Some(file) = file {
                                        let path = file.path().to_path_buf();
                                        let _ = commands_tx
                                            .send(dropship::Command::AddExecutable { path });
                                    }
                                });
                            }
                        }
                    });
                });
            });
        });

        // path delegator
        if let Some(path) = &self.modal_manage_path {
            let mut should_close = false;

            let modal = egui::Modal::new(egui::Id::new("export_ips")).show(ui.ctx(), |ui| {
                ui.set_max_width(600.);
                ui.set_max_height(400.);

                Self::draw_path(&path, ui);

                ui.separator();

                {
                    let button = egui::Button::new("فتح مكان الملف");
                    let button = ui.add_sized(egui::vec2(ui.available_width(), 16.0), button);

                    if button.clicked() {
                        #[cfg(target_os = "windows")]
                        if let Err(e) = std::process::Command::new("explorer")
                            .arg("/select,")
                            .arg(path)
                            .spawn()
                        {
                            log::error!("{}", e);
                        }

                        should_close = true;
                    }
                }

                if let Some(known_paths) = self.config.known_paths.as_mut() {
                    {
                        let button = egui::Button::new("نسيان هذا الملف");
                        let button = ui.add_sized(egui::vec2(ui.available_width(), 16.0), button);

                        if button.clicked() {
                            known_paths.remove(path);
                            match firewall::get_dropship_rule(path) {
                                Ok(rule) => {
                                    if let Some(rule) = rule {
                                        if let Err(e) = unsafe { rule.SetEnabled(false.into()) } {
                                            log::error!("{}", e);
                                        }
                                        let name = windows::core::BSTR::from("delete");
                                        if let Err(e) = unsafe { rule.SetName(&name) } {
                                            log::error!("{}", e);
                                        }

                                        if let Ok(rules) = unsafe { firewall::win::get_rules() } {
                                            if let Err(e) = unsafe { rules.Remove(&name) } {
                                                log::error!("{}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("{}", e);
                                }
                            }

                            should_close = true;
                        }
                    }
                }
            });

            if modal.should_close() || should_close {
                self.modal_manage_path = None;
            }
        }
    }

    pub(crate) fn notice(&mut self, ui: &mut egui::Ui) {
        if let Some(notice) = self
            .cache
            .as_ref()
            .and_then(|x| x.cached_api_data.as_ref().and_then(|x| x.notices.last()))
        {
            rtl_row(ui, |ui| {
                ui.heading(&notice.title);

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(&notice.date);
                });
            });

            ui.add_space(8.0);

            ui.label(&notice.paragraph);
        } else {
            // match self.dropship_api_data.state() {
            //     egui_async::StateWithData::Failed(error) => {
            //         ui.label(error.to_string());
            //     }

            //     _ => {
            //         ui.spinner();
            //     }
            // }
        }
    }

    pub(crate) fn log(&mut self, ui: &mut egui::Ui) {
        for record in &self.logs {
            let color = match record.level {
                log::Level::Error => ui.visuals().error_fg_color,
                log::Level::Warn => ui.visuals().warn_fg_color,
                log::Level::Info => crate::visuals::BATTLENET_BLUE,
                // log::Level::Debug => crate::visuals::HEX_BDB2FF,
                log::Level::Debug => egui::Color32::PURPLE,
                _ => ui.visuals().text_color(),
            };

            ui.horizontal(|ui| ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let level = match record.level {
                    log::Level::Error => "خطأ",
                    log::Level::Warn => "تنبيه",
                    log::Level::Info => "معلومة",
                    log::Level::Debug => "تصحيح",
                    log::Level::Trace => "تتبّع",
                };
                ui.colored_label(color, level);

                ui.add(egui::Label::new(&record.message).wrap())
                    .on_hover_ui_at_pointer(|ui| {
                        ui.label(format!("الخيط #{}", record.thread_id.as_u64().get()));
                        ui.add(
                            egui::Label::new(
                                chrono_humanize::HumanTime::from(record.time)
                                    .to_string()
                                    .to_ascii_lowercase(),
                            )
                            .wrap_mode(egui::TextWrapMode::Extend),
                        );
                    });
            }));
        }
    }

    pub(crate) fn help_wizard(&mut self, ui: &mut egui::Ui) {
        // عن النسخة
        ui.label(egui::RichText::new(format!("dropship — النسخة العربية v{}", env!("CARGO_PKG_VERSION"))).strong());
        ui.label(format!("تطوير وتصميم: {} · مبني على dropship الأصلي من stormy (GPL-3.0)", dropship::AUTHOR_AR));

        ui.separator();

        ui.label("تحتاج مساعدة أو عندك مشكلة أو اقتراح؟ تعال ديسكورد النسخة العربية");
        rtl_row(ui, |ui| {
            ui.label("•  ");
            ui.hyperlink(dropship::DISCORD_INVITE_LINK);
        });
        ui.label("أو افتح issue على GitHub");
        rtl_row(ui, |ui| {
            ui.label("•  ");
            ui.hyperlink(dropship::GITHUB_URI);
        });

        ui.separator();

        ui.label("أعجبك البرنامج؟ ادعم استمرار تطوير النسخة العربية");
        rtl_row(ui, |ui| {
            ui.label("•  ");
            ui.hyperlink_to("PayPal", dropship::PAYPAL_URI)
                .on_hover_text_at_pointer(dropship::PAYPAL_URI);
        });

        ui.separator();

        ui.label("ديسكورد البرنامج الأصلي (بالإنجليزي، لمشاكل السيرفرات نفسها)");
        rtl_row(ui, |ui| {
            ui.label("•  ");
            ui.hyperlink(dropship::UPSTREAM_DISCORD_INVITE_LINK);
        });

        ui.separator();

        if ui.button("إعادة الجولة التعريفية").clicked() {
            self.start_tour(ui.ctx());
        }
    }

    pub(crate) fn options(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        ui.vertical(|ui| {
            // egui::Sides::new().show(ui, |ui| {}, |ui| {});

            let theme = self.get_theme(ui);

            egui::Panel::left("xx")
                .frame(
                    egui::Frame::default()
                        .outer_margin(egui::Margin::ZERO)
                        .inner_margin(egui::Margin::ZERO),
                )
                .exact_size(290.)
                .resizable(false)
                .show(ui, |ui| {
                  ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    // export ips
                    {
                        if ui.button("تصدير الآيبيات المحظورة").clicked() {
                            self.export_ips_modal = true;
                        }

                        if self.export_ips_modal {
                            let modal = egui::Modal::new(egui::Id::new("export_ips")).show(
                                ui.ctx(),
                                |ui| {
                                    ui.set_max_width(400.);
                                    ui.set_max_height(400.);

                                    let ips = self
                                        .known_servers()
                                        .into_iter()
                                        .filter(|x| self.config.desired_blocked_servers.has(x))
                                        .map(|x| x.block.clone())
                                        .collect::<Vec<_>>();

                                    // ui.heading("ips");

                                    ui.horizontal(|ui| {
                                        components::server_list_item::server_list_indicators(
                                            ui,
                                            self.known_servers(),
                                            &self.config.desired_blocked_servers,
                                            self.config.blocked_servers,
                                            //
                                            theme,
                                        );
                                    });

                                    ui.separator();

                                    ui.label(match ips.len() {
                                        0 => "ما عندك سيرفرات محظورة.".to_string(),
                                        1 => "عندك سيرفر واحد محظور.".to_string(),
                                        2 => "عندك سيرفران محظوران.".to_string(),
                                        n @ 3..=10 => format!("عندك {n} سيرفرات محظورة."),
                                        n => format!("عندك {n} سيرفر محظور."),
                                    });

                                    if let Some(s) = self.known_servers().iter().find(|x| {
                                        self.config.blocked_servers.has(x)
                                            && x.token.starts_with("g")
                                    }) {
                                        ui.separator();

                                        ui.label(format!(
                                            "تنبيه: آيبيات {} تتغير كثير.",
                                            &s.token
                                        ));

                                        ui.label("مو فكرة زينة تحظر السيرفرات التالية يدويًا");
                                    }

                                    if !ips.is_empty() {
                                        ui.separator();
                                        egui::ScrollArea::vertical()
                                            .content_margin(egui::Margin {
                                                right: 4 + 8, // gap + width + margin
                                                top: 0,
                                                left: 0,
                                                bottom: 0,
                                            })
                                            .auto_shrink([false, true])
                                            .show(ui, |ui| {
                                                ui.label(ips.join(","));
                                            });
                                    } else {
                                        // ui.separator();
                                        // ui.colored_label(ui.visuals().weak_text_color(), "none");
                                    }

                                    ui.separator();

                                    ui.scope(|ui| {
                                        if ips.is_empty() {
                                            ui.disable();
                                        }
                                        ui.vertical_centered(|ui| {
                                            let button = egui::Button::new("نسخ");
                                            let button = ui.add_sized(
                                                egui::vec2(ui.available_width(), 16.0),
                                                button,
                                            );

                                            if button.clicked() {
                                                ui.copy_text(ips.join(","));
                                            }
                                        });
                                    })
                                },
                            );

                            if modal.should_close() {
                                self.export_ips_modal = false;
                            }
                        }
                    }

                    ui.separator();

                    // persistence
                    {
                        if ui.button("مسح الكاش").clicked() {
                            {
                                // let cache = self.cache.clone();
                                // *self = Self::default();
                                self.config = DropshipConfig::default();

                                // self.welcomed = true;
                                // self.cache = cache;
                                self.cache = None;
                            }

                            // this is not necessary
                            {
                                if let Some(storage) = frame.storage_mut() {
                                    if let Ok(json) = serde_json::to_string(&self.config) {
                                        storage.set_string(eframe::APP_KEY, json);
                                    }
                                    storage.set_string(CACHE_KEY, "None".into());

                                    storage.flush();
                                }
                            }

                            if let Err(e) = firewall::delete_dropship_rules() {
                                log::error!("{}", e);
                            }

                            let _ = self
                                .commands_tx
                                .send(dropship::Command::UpdateConfigFromRemote);

                            self.apply_zoom(ui, self.config.zoom);
                            self.apply_theme(ui);
                            self.apply_mini_mode(ui);

                            // ui.request_repaint();
                        }
                    }

                    ui.separator();
                    if ui
                        .link("اضغط لإعادة جدار حماية ويندوز لإعدادات المصنع")
                        .clicked()
                    {
                        match unsafe { firewall::win::reset_global_windows_firewall() } {
                            Ok(_) => {
                                log::debug!("أُعيد ضبط جدار حماية ويندوز");
                            }
                            Err(e) => {
                                log::error!("{}", e.to_string());
                            }
                        }
                    }

                    ui.separator();
                    if ui.link("اضغط لمسح DNS ويندوز").clicked() {
                        unsafe { firewall::win::flush_dns() };
                    }
                  });
                });

            // ui.separator();

            // // welcome
            // {
            //     if ui.button("show welcome page").clicked() {
            //         self.modal_welcome_page = Some(0);
            //     }

            //     ui.separator();

            //     ui.add(egui::Checkbox::new(
            //         &mut self.config.always_show_welcome,
            //         "always show welcome page when app is opened",
            //     ));
            // }

            // ui.separator();

          ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
            ui.separator();

            // window zoom

            {
                rtl_row(ui, |ui| {
                    ui.label("حجم النافذة");

                    let window_scale = ui.add(
                        egui::DragValue::new(&mut self.config.zoom)
                            // .step_by(0.1)
                            .fixed_decimals(2)
                            // .range(0.5..=2.)
                            .range(0.75..=1.5)
                            .speed(0.02)
                            .update_while_editing(false),
                    );
                    if window_scale.drag_stopped() || window_scale.lost_focus() {
                        self.apply_zoom(ui, self.config.zoom);
                    }
                });
            }

            ui.separator();

            // theme
            {
                self.theme_dropdown(ui);
            }

            ui.separator();

            // wfp dynamic
            {
                self.dynamic_wfp_dropdown(ui);
            }

            ui.separator();

            {
                let mut is_checked = self.config.starting_tab == TAB_LOG;

                if ui
                    .checkbox(&mut is_checked, "افتح السجل عند التشغيل")
                    .changed()
                {
                    self.config.starting_tab = if is_checked { TAB_LOG } else { 0 };
                }

                ui.separator();
            }

            {
                ui.checkbox(
                    &mut self.config.disable_background_image,
                    "إخفاء خريطة العالم",
                );

                ui.separator();
            }

            {
                if ui.button("إعادة الجولة التعريفية").clicked() {
                    self.start_tour(ui.ctx());
                }

                ui.separator();
            }
          });

            // ui.separator();
            // if ui.link("windowsdefender://network").clicked() {
            //     std::process::Command::new("explorer.exe")
            //         .arg("windowsdefender://network")
            //         .spawn()
            //         .ok();
            // }

            // ui.separator();
            // if ui.link("wf.msc").clicked() {
            //     std::process::Command::new("mmc.exe")
            //         .arg("wf.msc")
            //         .spawn()
            //         .ok();
            // }
        });
    }

    fn welcome(&mut self, ui: &mut egui::Ui, page: u8) {
        //
        let trapped = !self.config.welcomed;

        let modal = egui::Modal::new(egui::Id::new("welcome")).show(ui.ctx(), |ui| {
            let mut last_page = false;

            ui.set_max_width(400.);
            ui.set_max_height(400.);

            let (since, dir) = (self.welcome_changed_at, self.welcome_dir);
            slide_in(ui, since, 36. * dir, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
            match page {
                0 => {
                    ui.heading("مُحدد سيرفرات أوفرواتش");
                    ui.label(
                        "هذا البرنامج يخليك تتحكم بأي سيرفرات أوفرواتش تلعب عليها",
                    );
                    ui.label(
                        egui::RichText::new(format!("النسخة العربية · تطوير {}", dropship::AUTHOR_AR)).weak(),
                    );

                    ui.separator();
                    ui.label("هذا البرنامج *لا*:");
                    ui.indent("xd", |ui| {
                        ui.label("• يعدّل أي ملفات للعبة");
                        ui.label("• يخالف شروط استخدام بليزارد");
                    });

                }
                1 => {
                    ui.heading("كيف يشتغل");

                    ui.label("تختار السيرفر اللي تبيه بـ*حظر* السيرفرات اللي ما تبيها");
                    ui.indent("xd4", |ui| {
                        ui.label("• ما تحتاج تبقي dropship مفتوح");
                        ui.label("• الحظر يبقى لين تلغيه");
                    });
                }
                _ => {
                    // ui.heading("done");

                    ui.label("إذا شيء ما يشتغل، اطلب المساعدة في ديسكورد النسخة العربية");
                    rtl_row(ui, |ui| {
                        ui.label("•  ");
                        ui.hyperlink(dropship::DISCORD_INVITE_LINK);
                    });

                    if self.known_servers().is_empty() {
                        ui.separator();

                        rtl_row(ui, |ui| {
                            ui.label("بانتظار بيانات السيرفرات ");

                            ui.spinner();
                        });
                    }

                    ui.separator();

                    ui.label("اختر المظهر:");
                    self.theme_dropdown(ui);

                    // waiting for dropship data.
                    // do not allow continuing until there are known servers

                    last_page = true;
                }
            }
            });
            });

            ui.separator();

            let mut go_back = false;
            let enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));

            // معكوس: «رجوع/خروج» على اليمين، «التالي/ابدأ» على اليسار
            egui::Sides::new().show(
                ui,
                |ui| {
                    if !last_page {
                        if ui.button("التالي >").clicked() || enter {
                            self.modal_welcome_page = Some(page + 1);
                        }
                    } else {
                        ui.scope(|ui| {
                            if self.known_servers().is_empty() {
                                ui.disable();
                            }

                            if (ui
                                .button("ابدأ")
                                .clicked() || enter) && !self.known_servers().is_empty()
                            {
                                self.config.welcomed = true;
                                self.modal_welcome_page = None;
                                if !self.config.toured {
                                    self.start_tour(ui.ctx());
                                }
                            }
                        });
                    }
                },
                |ui| {
                    if page > 0 {
                        if ui.button("< رجوع").clicked() {
                            // self.modal_welcome_page = Some(page - 1);
                            go_back = true;
                        }
                    } else {
                        {
                            if trapped {
                                let icon_size = 12.;

                                let icon = egui::Image::new(assets::ICON_POWER_OFF)
                                    .fit_to_exact_size(egui::vec2(icon_size, icon_size))
                                    .tint(ui.visuals().text_color());

                                let button = egui::Button::image_and_text(icon, "خروج").gap(6.);
                                if ui.add(button).clicked() {
                                    ui.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            }
                        }
                    }
                },
            );

            if go_back {
                self.modal_welcome_page = Some(page - 1);
            }
        });

        if !trapped && modal.should_close() {
            self.modal_welcome_page = None;
        }
    }

    fn updater(&mut self, ui: &mut egui::Ui) {
        if self.should_show_update_modal() {
            if let Some(update) = &self.update_available {
                let theme = self.get_theme(ui);

                let modal = egui::Modal::new(egui::Id::new("update")).show(ui.ctx(), |ui| {
                    ui.set_max_width(400.);
                    ui.set_max_height(400.);

                    ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui|
                    match &self.installing_status {
                        update::UpdatingStatus::NotActive => {
                            ui.label(format!("الإصدار {} متوفر", update.version));
                            ui.colored_label(
                                ui.style().visuals.weak_text_color(),
                                format!(
                                    "{} • {:.2} م.ب • {} تنزيل",
                                    chrono_humanize::HumanTime::from(update.binary.updated_at)
                                        .to_string(),
                                    update.binary.size as f32 / 1_048_576.0,
                                    update.binary.download_count,
                                ),
                            );

                            {
                                ui.separator();
                                egui::Frame::group(ui.style())
                                    .fill(visuals::from_theme_alpha(theme, 20))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());

                                        egui::ScrollArea::vertical()
                                            .max_height(128.)
                                            .content_margin(egui::Margin {
                                                right: 4 + 8, // gap + width + margin
                                                top: 0,
                                                left: 0,
                                                bottom: 0,
                                            })
                                            .auto_shrink([false, true])
                                            .show(ui, |ui| {
                                                ui.indent("..", |ui| {
                                                    ui.label(&update.description);
                                                })
                                            });
                                    });
                            }

                            ui.separator();

                            ui.vertical_centered(|ui| {
                                // let button = egui::Button::new("download");
                                let button = egui::Button::new("تحديث");
                                let button =
                                    ui.add_sized(egui::vec2(ui.available_width(), 16.0), button);

                                if button.clicked() {
                                    // ui.copy_text(ips.join(","));

                                    // self.update_install.request(update::update(
                                    //     update.binary.browser_download_url.clone(),
                                    //     // self.update_progress.clone(),
                                    //     (&self.download_total_size, &self.downloaded_size),
                                    // ));

                                    let _ = self.commands_tx.send(
                                        dropship::Command::ApplicationUpdate {
                                            binary_download: update
                                                .binary
                                                .browser_download_url
                                                .clone(),
                                            download_total_size: self.download_total_size.clone(),
                                            downloaded_size: self.downloaded_size.clone(),
                                        },
                                    );
                                }
                            });
                        }
                        update::UpdatingStatus::Downloading => {
                            ui.label("جارٍ التنزيل..");

                            ui.separator();

                            // let progress = match self.update_progress.lock() {
                            //     Ok(g) => *g,
                            //     Err(_) => 0.,
                            // };

                            let download_total_size =
                                self.download_total_size.load(atomic::Ordering::Relaxed);
                            match download_total_size {
                                0 => {
                                    ui.add(egui::ProgressBar::new(0.).show_percentage());
                                }
                                _ => {
                                    let downloaded_size =
                                        self.downloaded_size.load(atomic::Ordering::Relaxed);

                                    let progress =
                                        downloaded_size as f32 / download_total_size as f32;

                                    ui.add(
                                        egui::ProgressBar::new(progress)
                                            // .show_percentage()
                                            .text(format!(
                                                "{:.2}% ({:.2} / {:.2} م.ب)",
                                                progress * 100.,
                                                downloaded_size as f32 / 1_048_576.0,
                                                download_total_size as f32 / 1_048_576.0
                                            )),
                                    );
                                }
                            }
                        }
                        update::UpdatingStatus::Installed => {
                            let download_total_size =
                                self.download_total_size.load(atomic::Ordering::Relaxed);

                            ui.label("اكتمل التنزيل");

                            ui.separator();

                            ui.add(
                                egui::ProgressBar::new(1.)
                                    // .show_percentage()
                                    .text(format!(
                                        "{:.2}% (نُزّل {:.2} م.ب)",
                                        100.,
                                        download_total_size as f32 / 1_048_576.0,
                                    )),
                            );

                            ui.separator();

                            ui.vertical_centered(|ui| {
                                let button =
                                    egui::Button::new(format!("تشغيل v{}", update.version));
                                let button =
                                    ui.add_sized(egui::vec2(ui.available_width(), 16.0), button);

                                if button.clicked() {
                                    self.restart_requested = true;
                                    ui.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            });
                        }
                        update::UpdatingStatus::Failed(e) => {
                            ui.label("خطأ");
                            ui.separator();
                            ui.colored_label(ui.visuals().error_fg_color, e);

                            ui.vertical_centered(|ui| {
                                let button = egui::Button::new("إغلاق");
                                let button =
                                    ui.add_sized(egui::vec2(ui.available_width(), 16.0), button);

                                if button.clicked() {
                                    self.hide_update = true;
                                }
                            });
                        }
                    });
                });

                if modal.should_close() {
                    // prevent close while downloading
                    if !matches!(self.installing_status, update::UpdatingStatus::Downloading) {
                        // self.update_bind.abort();z
                        self.hide_update = true;
                    }
                }
            }
        }
    }

    fn should_show_update_modal(&self) -> bool {
        if self.update_available.is_some() {
            !self.hide_update
        } else {
            false
        }
    }

    fn suggest_path(&mut self, ui: &mut egui::Ui) {
        let mut denied = false;

        if let Some(path) = &self.suggesting_path {
            //
            let mut should_close = false;

            let modal = egui::Modal::new(egui::Id::new("update")).show(ui.ctx(), |ui| {
                ui.set_max_width(400.);
                ui.set_max_height(400.);

                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    ui.heading("لعبة جديدة");
                    ui.label("dropship لقى لعبة مفتوحة ما أُضيفت بعد. تبي تضيفها؟");
                });

                ui.separator();

                Self::draw_path(path, ui);

                ui.separator();

                {
                    let button = egui::Button::new("إضافة إلى dropship");
                    let button = ui.add_sized(egui::vec2(ui.available_width(), 16.0), button);

                    if button.clicked() {
                        // events_tx.send(Event::AddedExecutable(path))
                        let path = path.clone();
                        let _ = self
                            .commands_tx
                            .send(dropship::Command::AddExecutable { path });

                        should_close = true;
                    }
                }

                ui.scope(|ui| {
                    {
                        let theme = self.get_theme(ui);

                        ui.style_mut().visuals.widgets.inactive.weak_bg_fill =
                            visuals::from_theme_alpha(theme, 0);
                        ui.style_mut().visuals.widgets.active.weak_bg_fill =
                            visuals::from_theme_alpha(theme, 40);
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                            visuals::from_theme_alpha(theme, 20);

                        ui.style_mut().visuals.override_text_color =
                            Some(ui.style_mut().visuals.weak_text_color());
                    }

                    let button = egui::Button::new("تجاهل");
                    let button = ui.add_sized(egui::vec2(ui.available_width(), 16.0), button);

                    if button.clicked() {
                        should_close = true;
                        denied = true;
                    }
                });
            });

            if modal.should_close() || should_close {
                if denied {
                    self.denied_paths.insert(path.to_owned());
                }
                self.suggesting_path = None;
            }
        }
    }

    fn theme_dropdown(&mut self, ui: &mut egui::Ui) {
        fn name(t: &Option<visuals::Theme>) -> &'static str {
            match t {
                Some(visuals::Theme::Light) => "فاتح",
                Some(visuals::Theme::Dark) => "داكن",
                None => "مثل الجهاز",
            }
        }

        let before = self.config.theme;
        rtl_row(ui, |ui| {
            ui.label("المظهر");
            egui::ComboBox::from_id_salt("theme")
                .selected_text(name(&self.config.theme))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.theme, None, name(&None));
                    ui.selectable_value(
                        &mut self.config.theme,
                        Some(visuals::Theme::Dark),
                        name(&Some(visuals::Theme::Dark)),
                    );
                    ui.selectable_value(
                        &mut self.config.theme,
                        Some(visuals::Theme::Light),
                        name(&Some(visuals::Theme::Light)),
                    );
                });
        });

        if self.config.theme != before {
            self.apply_theme(ui);
        }
    }

    fn dynamic_wfp_dropdown(&mut self, ui: &mut egui::Ui) {
        fn name(dynamic: bool) -> &'static str {
            if dynamic {
                "فقط والبرنامج مفتوح"
            } else {
                "دائمًا"
            }
        }

        let before = self.config.wfp_dynamic_session;
        rtl_row(ui, |ui| {
            ui.label("حظر السيرفرات");
            egui::ComboBox::from_id_salt("block_servers")
                .selected_text(name(self.config.wfp_dynamic_session))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.wfp_dynamic_session, false, name(false));
                    ui.selectable_value(&mut self.config.wfp_dynamic_session, true, name(true));
                });
        });

        if self.config.wfp_dynamic_session != before {
            self.apply_wfp_session();
        }
    }
}

// ---------------------------------------------------------------------------
// الجولة التعريفية (product tour)
// ---------------------------------------------------------------------------

/// (مفتاح العنصر، التبويب الذي يجب فتحه، العنوان، الشرح)
const TOUR_STEPS: &[(&str, Option<usize>, &str, &str)] = &[
    (
        "map",
        Some(launcher::VIEW_MAP),
        "خريطة السيرفرات",
        "كل نقطة سيرفر أوفرواتش حول العالم، والخطوط تربطك بكل سيرفر مسموح. الذهبي هو الأفضل لك (أقل بنق).\nاضغط نقطة لحظر السيرفر أو إلغاء حظره، والزر الأيمن يبقيه ويعكس الباقي.",
    ),
    (
        "servers",
        Some(launcher::VIEW_MAP),
        "قائمة السيرفرات",
        "نفس السيرفرات كقائمة: العلم والاسم والرمز، شريط البنق (أخضر ممتاز، ذهبي مقبول، أحمر ضعيف)، والمفتاح يسمح أو يحظر. زر «حسب البنق» يرتّبها من الأسرع.",
    ),
    (
        "presets",
        Some(launcher::VIEW_MAP),
        "الاختصارات",
        "مجموعات حظر جاهزة بضغطة واحدة: «أوروبا» مثلًا يحظر السيرفر السعودي فقط (أو بمفتاح F1). اضغط + لإنشاء اختصارك: اسم، مفتاح، ثم السيرفرات التي تُحظر. الزر الأيمن على أي اختصار يعدّله أو يحذفه.",
    ),
    (
        "route",
        Some(launcher::VIEW_MAP),
        "أفضل مسار",
        "السيرفر الذي بتلعب عليه على الأغلب: أقل بنق بين المسموح. ومنها تختار هل الحظر «دائم» حتى بعد إغلاق البرنامج، أو «أثناء التشغيل» فقط.",
    ),
    (
        "mini",
        Some(launcher::VIEW_MAP),
        "الوضع المصغّر",
        "هذا الزر يصغّر النافذة ويخفي الخريطة والتفاصيل (تبقى قائمة السيرفرات فقط)، والضغط مرة ثانية يرجعها.",
    ),
    (
        "disable",
        Some(launcher::VIEW_MAP),
        "ارفع كل الحظر",
        "يلغي كل الحظر فورًا. إذا فشل الاتصال بسيرفر وأنت في طابور التنافسي، اضغطه بسرعة لتتجنب الحظر.",
    ),
    (
        "stars",
        None,
        "الحالة",
        "رقاقتان في الأعلى: الفلتر (شغّال وكم سيرفر محظور؛ مرّر عليها لترى أسماءهم) واللعبة (هل أوفرواتش مفتوحة الآن). التغييرات تُطبَّق بعد إغلاق اللعبة.",
    ),
    (
        "games",
        Some(launcher::VIEW_GAMES),
        "الألعاب",
        "هنا تظهر ملفات اللعبة اللي يطبّق عليها الحظر. تُكتشف تلقائيًا وقت تفتح اللعبة، أو أضفها يدويًا بزر «أضف لعبة». اضغط على لعبة لفتح مكانها أو نسيانها.",
    ),
    (
        "tab_notices",
        Some(launcher::VIEW_NEWS),
        "الأخبار",
        "آخر إشعار من مطوّر البرنامج الأصلي (بالإنجليزية)، مثل تغيّر سيرفرات أو تحذيرات مهمة.",
    ),
    (
        "tab_log",
        Some(launcher::VIEW_LOG),
        "السجل",
        "كل ما يصير داخل البرنامج: اتصال، حظر، أخطاء. اضغط رسالة الحالة في أسفل النافذة لفتحه بسرعة.",
    ),
    (
        "tab_help",
        Some(launcher::VIEW_HELP),
        "المساعدة",
        "روابط الديسكورد وGitHub لطلب الدعم أو اقتراح ميزة، وزر لإعادة هذي الجولة.",
    ),
    (
        "tab_options",
        Some(launcher::VIEW_OPTIONS),
        "الخيارات",
        "تصدير الآيبيات المحظورة، مسح الكاش، إعادة ضبط جدار الحماية، حجم النافذة، المظهر، ومدة الحظر.",
    ),
    (
        "status",
        None,
        "شريط الحالة",
        "يعرض آخر رسالة لثوانٍ: الأحمر خطأ، والملوّن تنبيه. اضغط عليها لفتح السجل.",
    ),
    (
        "footer",
        None,
        "الاختصارات",
        "Esc لإغلاق البرنامج، M للخريطة، والأرقام 1 إلى 4 للأخبار والسجل والمساعدة والخيارات، ومفاتيح الاختصارات تطبّقها فورًا. L زر الفأرة الأيسر يبدّل السيرفر، وR الأيمن يعكس الباقي.\nالحظر يبقى شغّال حتى بعد إغلاق النافذة.",
    ),
];

impl TemplateApp {
    fn tour_overlay(&mut self, ui: &mut egui::Ui, step: usize) {
        let Some(&(key, tab, title, body)) = TOUR_STEPS.get(step) else {
            self.tour = None;
            self.config.toured = true;
            return;
        };

        // افتح التبويب الذي تشرحه هذه الخطوة
        if let Some(tab) = tab {
            self.tab = tab;
        }

        let ctx = ui.ctx().clone();
        let screen = ctx.viewport_rect();
        let target = self
            .tour_rects
            .get(key)
            .copied()
            .unwrap_or_else(|| egui::Rect::from_center_size(screen.center(), egui::Vec2::ZERO))
            .expand(6.);

        // تعتيم كل شيء ما عدا العنصر المستهدف، وحجب الضغطات
        egui::Area::new(egui::Id::new("tour_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(&ctx, |ui| {
                ui.allocate_rect(screen, egui::Sense::click());
                let dim = egui::Color32::from_black_alpha(150);
                let p = ui.painter();
                p.rect_filled(
                    egui::Rect::from_min_max(screen.min, egui::pos2(screen.max.x, target.min.y)),
                    0.,
                    dim,
                );
                p.rect_filled(
                    egui::Rect::from_min_max(egui::pos2(screen.min.x, target.max.y), screen.max),
                    0.,
                    dim,
                );
                p.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(screen.min.x, target.min.y),
                        egui::pos2(target.min.x, target.max.y),
                    ),
                    0.,
                    dim,
                );
                p.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(target.max.x, target.min.y),
                        egui::pos2(screen.max.x, target.max.y),
                    ),
                    0.,
                    dim,
                );
                // نبض هادئ حول العنصر (≈20 إطار/ث فقط أثناء الجولة)
                if cfg!(feature = "animations") {
                    let pulse = 0.5 + 0.5 * ((ui.input(|i| i.time) * 3.0).sin() as f32);
                    p.rect_stroke(
                        target.expand(2. + 4. * pulse),
                        8.,
                        egui::Stroke::new(1., visuals::SAUDI_GREEN_LIGHT.gamma_multiply(0.35 * pulse + 0.1)),
                        egui::StrokeKind::Outside,
                    );
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
                }
                p.rect_stroke(
                    target,
                    6.,
                    egui::Stroke::new(2., visuals::SAUDI_GREEN_LIGHT),
                    egui::StrokeKind::Outside,
                );
            });

        // بطاقة الشرح: تحت العنصر إن وُجدت مساحة، وإلا فوقه. egui يحصرها داخل الشاشة.
        let card_w = 340.;
        let card_h_guess = 170.;
        let x = (target.max.x - card_w).max(screen.min.x + 8.);
        let y = if target.max.y + 12. + card_h_guess < screen.max.y {
            target.max.y + 12.
        } else {
            (target.min.y - 12. - card_h_guess).max(screen.min.y + 8.)
        };

        let total = TOUR_STEPS.len();
        let last = step + 1 == total;
        let mut next = None;
        let since = self.tour_changed_at;

        // لوحة المفاتيح: Enter أو ← للتالي، → للرجوع
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
            {
                next = Some(if last { None } else { Some(step + 1) });
            } else if step > 0 && i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) {
                next = Some(Some(step - 1));
            }
        });

        egui::Area::new(egui::Id::new("tour_card"))
            .order(egui::Order::Tooltip)
            .fixed_pos(egui::pos2(x, y))
            .show(&ctx, |ui| {
                egui::Frame::window(ui.style())
                    .stroke(egui::Stroke::new(1., visuals::SAUDI_GREEN))
                    .show(ui, |ui| {
                        ui.set_width(card_w);
                        slide_in(ui, since, 0., |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                            rtl_row(ui, |ui| {
                                ui.heading(title);
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.weak(format!("{} / {}", step + 1, total));
                                    },
                                );
                            });
                            ui.add_space(4.);
                            ui.label(body);
                            ui.add_space(8.);
                            rtl_row(ui, |ui| {
                                if ui
                                    .button(if last { "إنهاء" } else { "التالي >" })
                                    .clicked()
                                {
                                    next = Some(if last { None } else { Some(step + 1) });
                                }
                                if step > 0 && ui.button("< رجوع").clicked() {
                                    next = Some(Some(step - 1));
                                }
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("تخطي الجولة").clicked() {
                                            next = Some(None);
                                        }
                                    },
                                );
                            });
                        });
                        });
                    });
            });

        if let Some(next) = next {
            self.tour = next;
            if next.is_none() {
                self.config.toured = true;
            }
        }
    }
}
