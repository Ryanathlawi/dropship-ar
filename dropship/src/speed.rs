use crate::api::KnownServer;
use crate::app::TemplateApp;
use crate::lang::{ar, tr};
use crate::overwatch::ServerSelection;
use crate::visuals::Palette;
use crate::{assets, launcher, ping};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, epaint, pos2, vec2};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CF: &str = "https://speed.cloudflare.com";
const STREAMS: usize = 6;
const TEST: Duration = Duration::from_secs(8);
const WARMUP: Duration = Duration::from_millis(1500);
const TICKS: [f32; 9] = [0., 5., 10., 50., 100., 250., 500., 750., 1000.];
const DOWN: [Color32; 2] = [Color32::from_rgb(31, 182, 255), Color32::from_rgb(106, 247, 200)];
const UP: [Color32; 2] = [Color32::from_rgb(155, 92, 255), Color32::from_rgb(224, 123, 255)];
static ZEROS: [u8; 8 << 20] = [0; 8 << 20];

#[derive(Default, Clone, Copy, PartialEq)]
pub enum Phase {
    #[default]
    Idle,
    Ping,
    Down,
    Up,
    Done,
}

impl Phase {
    fn running(self) -> bool {
        matches!(self, Phase::Ping | Phase::Down | Phase::Up)
    }
}

#[derive(Default, Clone)]
pub struct Speed {
    pub phase: Phase,
    pub live: f32,
    pub down: Option<f32>,
    pub up: Option<f32>,
    pub ping: Option<f32>,
    pub jitter: Option<f32>,
    pub ping_down: Option<f32>,
    pub ping_up: Option<f32>,
    pub isp: Option<String>,
    pub ip: Option<String>,
    pub server: Option<String>,
    pub failed: bool,
    pub saved: bool,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Record {
    pub time: i64,
    pub down: f32,
    pub up: Option<f32>,
    pub ping: Option<f32>,
}

pub type Checks = HashMap<String, Option<Result<ping::Quality, String>>>;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    client_ip: Option<String>,
    as_organization: Option<String>,
    colo: Option<Colo>,
}

#[derive(serde::Deserialize)]
struct Colo {
    city: Option<String>,
}

pub fn start(speed: &Arc<Mutex<Speed>>, ctx: &egui::Context) {
    {
        let mut s = speed.lock().unwrap();
        if s.phase.running() {
            return;
        }
        *s = Speed { phase: Phase::Ping, ..Default::default() };
    }
    let (speed, ctx) = (speed.clone(), ctx.clone());
    tokio::spawn(async move {
        let failed = run(&speed, &ctx).await.is_err();
        {
            let mut s = speed.lock().unwrap();
            s.phase = Phase::Done;
            s.live = 0.;
            s.failed = failed || s.down.is_none();
        }
        ctx.request_repaint();
    });
}

async fn run(speed: &Arc<Mutex<Speed>>, ctx: &egui::Context) -> reqwest::Result<()> {
    let client = |v4: bool| {
        let b = reqwest::Client::builder().http1_only().timeout(Duration::from_secs(20));
        if v4 { b.local_address(IpAddr::V4(Ipv4Addr::UNSPECIFIED)) } else { b }.build()
    };
    let v4 = probe(&client(true)?).await.is_ok();
    let (light, bulk) = (client(v4)?, client(v4)?);

    if let Ok(meta) = meta(&light).await {
        let mut s = speed.lock().unwrap();
        s.ip = meta.client_ip;
        s.isp = meta.as_organization;
        s.server = meta.colo.and_then(|c| c.city);
    }
    ctx.request_repaint();

    let mut idle = Vec::new();
    for _ in 0..11 {
        idle.push(probe(&light).await?);
    }
    idle.remove(0);
    {
        let mut s = speed.lock().unwrap();
        s.jitter = Some(idle.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / (idle.len() - 1) as f32);
        s.ping = median(idle);
        s.phase = Phase::Down;
    }

    let (down, ping_down) = load(&bulk, &light, speed, ctx, true).await;
    {
        let mut s = speed.lock().unwrap();
        s.down = down;
        s.ping_down = ping_down;
        s.live = 0.;
        s.phase = Phase::Up;
    }

    let (up, ping_up) = load(&bulk, &light, speed, ctx, false).await;
    let mut s = speed.lock().unwrap();
    s.up = up;
    s.ping_up = ping_up;
    Ok(())
}

async fn meta(client: &reqwest::Client) -> reqwest::Result<Meta> {
    client
        .get(format!("{CF}/meta"))
        .header("Referer", format!("{CF}/"))
        .send()
        .await?
        .json()
        .await
}

async fn probe(client: &reqwest::Client) -> reqwest::Result<f32> {
    let t = Instant::now();
    let r = client.get(format!("{CF}/__down?bytes=0")).send().await?;
    let total = t.elapsed().as_secs_f32() * 1000.;
    let server: f32 = r
        .headers()
        .get_all("server-timing")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|p| p.split_once("dur=")?.1.trim().parse::<f32>().ok())
        .sum();
    Ok((total - server).max(1.))
}

async fn load(
    bulk: &reqwest::Client,
    light: &reqwest::Client,
    speed: &Arc<Mutex<Speed>>,
    ctx: &egui::Context,
    down: bool,
) -> (Option<f32>, Option<f32>) {
    let bytes = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    let end = start + TEST;

    let workers: Vec<_> = (0..STREAMS)
        .map(|_| tokio::spawn(stream(bulk.clone(), bytes.clone(), end, down)))
        .collect();
    let pings = tokio::spawn({
        let light = light.clone();
        async move {
            let mut v = Vec::new();
            while Instant::now() < end {
                v.extend(probe(&light).await.ok());
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            v
        }
    });

    let (mut last, mut base) = ((start, 0), None);
    while Instant::now() < end {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let now = (Instant::now(), bytes.load(Relaxed));
        let rate = mbps(now.1 - last.1, now.0 - last.0);
        last = now;
        if base.is_none() && now.0 - start >= WARMUP {
            base = Some(now);
        }
        {
            let mut s = speed.lock().unwrap();
            s.live = if s.live == 0. { rate } else { s.live * 0.7 + rate * 0.3 };
        }
        ctx.request_repaint();
    }

    workers.iter().for_each(|w| w.abort());
    let total = base
        .filter(|b: &(Instant, u64)| last.1 > b.1)
        .map(|b| mbps(last.1 - b.1, last.0 - b.0));
    (total, median(pings.await.unwrap_or_default()))
}

async fn stream(client: reqwest::Client, bytes: Arc<AtomicU64>, end: Instant, down: bool) {
    let mut size = 256 << 10;
    while Instant::now() < end {
        let started = Instant::now();
        if down {
            let Ok(mut r) = client.get(format!("{CF}/__down?bytes=25000000")).send().await else {
                return;
            };
            while let Ok(Some(chunk)) = r.chunk().await {
                bytes.fetch_add(chunk.len() as u64, Relaxed);
            }
        } else {
            let Ok(r) = client.post(format!("{CF}/__up")).body(&ZEROS[..size]).send().await else {
                return;
            };
            let _ = r.bytes().await;
            bytes.fetch_add(size as u64, Relaxed);
            if started.elapsed() < Duration::from_millis(250) {
                size = (size * 2).min(ZEROS.len());
            }
        }
    }
}

fn mbps(bytes: u64, time: Duration) -> f32 {
    bytes as f32 * 8. / time.as_secs_f32().max(0.001) / 1e6
}

fn median(mut v: Vec<f32>) -> Option<f32> {
    v.sort_by(f32::total_cmp);
    v.get(v.len() / 2).copied()
}

pub fn check_servers(checks: &Arc<Mutex<Checks>>, servers: &[KnownServer], ctx: &egui::Context) {
    let mut c = checks.lock().unwrap();
    if c.values().any(Option::is_none) {
        return;
    }
    c.clear();
    for s in servers {
        c.insert(s.ping.clone(), None);
        let (checks, ctx, ip, block) = (checks.clone(), ctx.clone(), s.ping.clone(), s.block.clone());
        tokio::spawn(async move {
            let q = ping::quality(&ip, &block).await;
            checks.lock().unwrap().insert(ip, Some(q));
            ctx.request_repaint();
        });
    }
}

pub fn suggest(servers: &[KnownServer], checks: &Checks) -> ServerSelection {
    let q = |s: &KnownServer| checks.get(&s.ping).and_then(|q| q.as_ref()?.as_ref().ok().copied());
    let best = servers.iter().filter_map(q).map(|q| q.ping).fold(f32::MAX, f32::min);
    let mut sel = ServerSelection::none();
    for s in servers {
        let keep = q(s).is_some_and(|q| q.ping <= (best * 1.5).max(best + 40.) && q.loss <= 2. && q.jitter <= 15.);
        if !keep {
            sel.set(s);
        }
    }
    if servers.iter().all(|s| sel.has(s)) { ServerSelection::none() } else { sel }
}

fn date(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%d/%m %H:%M").to_string())
        .unwrap_or_default()
}

fn dial(p: &egui::Painter, c: Pos2, radius: f32, value: f32, colors: [Color32; 2], pal: &Palette) {
    let f = frac(value);
    if f > 0.002 {
        arc(p, c, radius, 18., f, move |t| lerp(colors, t / f.max(0.01)));
    }
    for (i, t) in TICKS.iter().enumerate() {
        let at = c + dir(i as f32 / 8.) * (radius - 34.);
        let color = if *t <= value { pal.text } else { pal.faint };
        p.text(at, Align2::CENTER_CENTER, format!("{t:.0}"), mono(12.), color);
    }
    let d = dir(f);
    let n = vec2(-d.y, d.x);
    let (tip, base) = (c + d * (radius - 24.), c + d * 26.);
    p.add(egui::Shape::convex_polygon(vec![tip, base + n * 7., base - n * 7.], pal.text.gamma_multiply(0.25), Stroke::NONE));
    let mid = c + d * ((radius - 24.) * 0.55 + 26. * 0.45);
    p.add(egui::Shape::convex_polygon(vec![tip, mid + n * 3.5, mid - n * 3.5], pal.text.gamma_multiply(0.85), Stroke::NONE));
}

fn mono(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Monospace)
}

fn frac(v: f32) -> f32 {
    let v = v.clamp(0., 1000.);
    let i = TICKS.windows(2).position(|w| v <= w[1]).unwrap_or(7);
    (i as f32 + (v - TICKS[i]) / (TICKS[i + 1] - TICKS[i])) / 8.
}

fn dir(f: f32) -> egui::Vec2 {
    let a = (135. + 270. * f).to_radians();
    vec2(a.cos(), a.sin())
}

fn lerp(c: [Color32; 2], t: f32) -> Color32 {
    let t = t.clamp(0., 1.);
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color32::from_rgb(m(c[0].r(), c[1].r()), m(c[0].g(), c[1].g()), m(c[0].b(), c[1].b()))
}

fn arc(p: &egui::Painter, c: Pos2, r: f32, w: f32, to: f32, color: impl Fn(f32) -> Color32 + Send + Sync + 'static) {
    let n = (to * 120.).ceil().max(2.) as usize;
    let points = (0..=n).map(|k| c + dir(to * k as f32 / n as f32) * r).collect();
    let stroke = epaint::PathStroke::new_uv(w, move |_, pos| {
        let mut deg = (pos - c).angle().to_degrees();
        if deg < 90. {
            deg += 360.;
        }
        color(((deg - 135.) / 270.).clamp(0., 1.))
    });
    p.add(epaint::PathShape::line(points, stroke));
}

#[derive(Clone, Copy)]
enum Icon {
    Down,
    Up,
    Idle,
}

fn icon(p: &egui::Painter, c: Pos2, icon: Icon, color: Color32) {
    let s = Stroke::new(1.4, color);
    p.circle_stroke(c, 7.5, s);
    match icon {
        Icon::Idle => {
            p.line_segment([c + vec2(-3.5, -1.5), c + vec2(3.5, -1.5)], s);
            p.line_segment([c + vec2(-3.5, 1.5), c + vec2(3.5, 1.5)], s);
        }
        Icon::Down | Icon::Up => {
            let k = if matches!(icon, Icon::Down) { 1. } else { -1. };
            p.line_segment([c + vec2(0., -3.5 * k), c + vec2(0., 3.5 * k)], s);
            p.line_segment([c + vec2(-2.8, 0.8 * k), c + vec2(0., 3.5 * k)], s);
            p.line_segment([c + vec2(2.8, 0.8 * k), c + vec2(0., 3.5 * k)], s);
        }
    }
}

fn caption(p: &egui::Painter, at: Pos2, text: String, size: f32, color: Color32, mark: Option<(Icon, Color32)>) -> Rect {
    let g = p.layout_no_wrap(text, mono(size), color);
    let r = Rect::from_center_size(at, g.size());
    p.galley(r.min, g, color);
    let Some((i, c)) = mark else { return r };
    let side = if ar() { 1. } else { -1. };
    let ic = at + vec2(side * (r.width() / 2. + 14.), 0.);
    icon(p, ic, i, c);
    r.union(Rect::from_center_size(ic, vec2(16., 16.)))
}

fn ms(v: Option<f32>) -> String {
    v.map_or("—".into(), |v| format!("{v:.0}"))
}

impl TemplateApp {
    pub(crate) fn connection_view(&mut self, ui: &mut egui::Ui, pal: &Palette) {
        let shareable = self.speed_view == 0 && {
            let s = self.speed.lock().unwrap();
            s.phase == Phase::Done && s.down.is_some()
        };
        ui.horizontal(|ui| {
            ui.with_layout(crate::lang::fwd(egui::Align::Center), |ui| {
                ui.selectable_value(&mut self.speed_view, 0, tr("سرعة النت", "internet speed"));
                ui.selectable_value(&mut self.speed_view, 1, tr("السيرفرات", "servers"));
                ui.selectable_value(&mut self.speed_view, 2, tr("السجل", "history"));
                if shareable {
                    ui.with_layout(crate::lang::back(egui::Align::Center), |ui| {
                        let button = egui::Button::new(tr("شارك النتيجة", "share result"));
                        if ui.add_enabled(self.share.is_none(), button).clicked() {
                            self.share = Some(Rect::NOTHING);
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                        }
                    });
                }
            })
        });
        ui.add_space(6.);
        match self.speed_view {
            1 => self.server_checks(ui, pal),
            2 => self.speed_history(ui, pal),
            _ => self.speed_test(ui, pal),
        }
    }

    pub(crate) fn save_speed_result(&mut self) {
        let mut s = self.speed.lock().unwrap();
        if s.phase != Phase::Done || s.failed || s.saved {
            return;
        }
        s.saved = true;
        let Some(down) = s.down else { return };
        self.config.speed_history.push(Record { time: chrono::Local::now().timestamp(), down, up: s.up, ping: s.ping });
        let n = self.config.speed_history.len();
        if n > 60 {
            self.config.speed_history.drain(..n - 60);
        }
    }

    fn speed_test(&mut self, ui: &mut egui::Ui, pal: &Palette) {
        let s = self.speed.lock().unwrap().clone();
        let (outer, _) = ui.allocate_exact_size(vec2(ui.available_width(), 440.), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), vec2(outer.width().min(700.), outer.height()));
        let p = ui.painter_at(outer);
        let x = |f: f32| if ar() { rect.max.x - rect.width() * f } else { rect.min.x + rect.width() * f };
        let top = rect.min.y + 14.;

        if self.share.is_some() {
            let shot = ui.input(|i| {
                i.raw.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            match shot {
                Some(image) => {
                    ui.ctx().copy_image(image.region(&rect, Some(ui.pixels_per_point())));
                    self.share = None;
                    log::info!("{}", tr("نسخت صورة النتيجة، الصقها في الديسكورد بـ Ctrl+V", "copied your result as an image, paste it in discord with ctrl+v"));
                }
                None => {
                    self.share = Some(rect);
                    ui.ctx().request_repaint();
                }
            }
        }
        let sharing = self.share.is_some();

        for (f, label, value, colors, i) in [
            (0.3, tr("التنزيل", "download"), s.down, DOWN, Icon::Down),
            (0.7, tr("الرفع", "upload"), s.up, UP, Icon::Up),
        ] {
            caption(&p, pos2(x(f), top), format!("{label}  Mbps"), 12., pal.muted, Some((i, colors[0])));
            let v = value.map_or("— —".into(), |v| format!("{v:.2}"));
            caption(&p, pos2(x(f), top + 34.), v, 26., pal.text, None);
        }

        let row = top + 82.;
        caption(&p, pos2(x(0.2), row), tr("البنق ms", "ping ms").into(), 12., pal.muted, None);
        let jitter = s.jitter.map_or(String::new(), |j| format!("{j:.0}"));
        for (f, v, i, c, hint) in [
            (0.42, s.ping, Icon::Idle, pal.gold, format!("{}  {jitter}", tr("البنق والنت فاضي، والتذبذب", "ping while the line is idle, and jitter"))),
            (0.6, s.ping_down, Icon::Down, DOWN[0], tr("البنق وقت التنزيل", "ping while downloading").into()),
            (0.78, s.ping_up, Icon::Up, UP[0], tr("البنق وقت الرفع", "ping while uploading").into()),
        ] {
            let r = caption(&p, pos2(x(f), row), ms(v), 13., pal.text, Some((i, c)));
            let _ = ui.interact(r.expand(4.), egui::Id::new(("speed_ping", i as u8)), Sense::hover()).on_hover_text(hint);
        }

        let c = pos2(rect.center().x, top + 250.);
        let radius = 116.;
        arc(&p, c, radius, 18., 1., {
            let t = pal.faint.gamma_multiply(0.3);
            move |_| t
        });

        if s.phase.running() || sharing {
            let value = if sharing {
                s.down.unwrap_or(0.)
            } else {
                ui.ctx().animate_value_with_time(egui::Id::new("speed_live"), s.live, 0.25)
            };
            let up = s.phase == Phase::Up && !sharing;
            let colors = if up { UP } else { DOWN };
            dial(&p, c, radius, value, colors, pal);

            let below = c + vec2(0., radius * 0.8);
            if s.phase == Phase::Ping && !sharing {
                caption(&p, below, tr("يقيس البنق", "measuring ping").into(), 14., pal.muted, None);
            } else {
                caption(&p, below, format!("{value:.2}"), 34., pal.text, None);
                let i = if up { Icon::Up } else { Icon::Down };
                caption(&p, below + vec2(0., 28.), "Mbps".into(), 12., pal.faint, Some((i, colors[0])));
            }
        } else {
            let go = Rect::from_center_size(c, vec2(150., 150.));
            let resp = ui.interact(go, egui::Id::new("speed_go"), Sense::click());
            let h = ui.ctx().animate_bool_with_time(egui::Id::new("speed_go_hover"), resp.hovered(), 0.15);
            p.circle_filled(c, 72., pal.panel_2.gamma_multiply(1. + h));
            arc(&p, c, 72., 3., 1., move |t| lerp(DOWN, t));
            p.circle_stroke(c, 72., Stroke::new(3., DOWN[1].gamma_multiply(0.6)));
            let label = if s.phase == Phase::Done { tr("مرة ثانية", "again") } else { tr("ابدأ", "GO") };
            p.text(c, Align2::CENTER_CENTER, label, egui::FontId::proportional(26.), pal.text);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                start(&self.speed, ui.ctx());
            }
            if s.failed {
                caption(&p, c + vec2(0., radius * 0.8), tr("ما قدرت أوصل لسيرفر الفحص، تأكد من النت", "couldn't reach the test server, check your connection").into(), 12., pal.red, None);
            }
        }

        let bottom = rect.max.y - 26.;
        let (start_al, end_al) = if ar() {
            (Align2::RIGHT_CENTER, Align2::LEFT_CENTER)
        } else {
            (Align2::LEFT_CENTER, Align2::RIGHT_CENTER)
        };
        if sharing {
            let forward = if ar() { -1. } else { 1. };
            let logo = Rect::from_center_size(pos2(x(0.04) + forward * 13., bottom), vec2(26., 26.));
            p.rect_filled(logo, 8., pal.accent_deep);
            egui::Image::new(egui::include_image!("../assets/white-bolts.png")).paint_at(ui, logo.shrink(6.));
            let brand = tr("dropship · النسخة العربية", "dropship · Athlawi edition");
            p.text(pos2(x(0.04) + forward * 34., bottom), start_al, brand, mono(13.), pal.text);
            if let Some(isp) = &s.isp {
                let isp: String = isp.chars().take(30).collect();
                p.text(pos2(x(0.96), bottom - 9.), end_al, isp, mono(12.), pal.text);
            }
            p.text(pos2(x(0.96), bottom + 9.), end_al, chrono::Local::now().format("%Y/%m/%d").to_string(), mono(11.), pal.faint);
            return;
        }
        if let Some(isp) = &s.isp {
            let isp: String = isp.chars().take(38).collect();
            p.text(pos2(x(0.04), bottom - 9.), start_al, isp, mono(12.), pal.text);
        }
        if let Some(ip) = &s.ip {
            p.text(pos2(x(0.04), bottom + 9.), start_al, ip, mono(11.), pal.accent);
        }
        if s.isp.is_some() || s.server.is_some() {
            p.text(pos2(x(0.96), bottom - 9.), end_al, "Cloudflare", mono(12.), pal.text);
            if let Some(city) = &s.server {
                p.text(pos2(x(0.96), bottom + 9.), end_al, city, mono(11.), pal.faint);
            }
        }
    }

    fn speed_history(&mut self, ui: &mut egui::Ui, pal: &Palette) {
        let h = self.config.speed_history.clone();
        if h.is_empty() {
            ui.add_space(20.);
            ui.label(egui::RichText::new(tr("ما فيه فحوصات للحين، سوّ فحص سرعة وبتطلع نتيجته هنا", "no tests yet, run a speed test and it'll show up here")).color(pal.muted));
            return;
        }

        let (outer, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 230.), Sense::hover());
        let p = ui.painter_at(outer);
        let (start_al, end_al) = if ar() {
            (Align2::RIGHT_CENTER, Align2::LEFT_CENTER)
        } else {
            (Align2::LEFT_CENTER, Align2::RIGHT_CENTER)
        };
        let forward = if ar() { -1. } else { 1. };
        let edge = if ar() { outer.max.x } else { outer.min.x };

        let mut lx = edge;
        for (label, color) in [(tr("التنزيل", "download"), DOWN[0]), (tr("الرفع", "upload"), UP[0])] {
            p.circle_filled(pos2(lx + forward * 5., outer.min.y + 9.), 4., color);
            let r = p.text(pos2(lx + forward * 14., outer.min.y + 9.), start_al, label, mono(12.), pal.muted);
            lx += forward * (r.width() + 30.);
        }

        let (x0, x1) = if ar() { (outer.max.x - 44., outer.min.x + 10.) } else { (outer.min.x + 44., outer.max.x - 10.) };
        let (y0, y1) = (outer.max.y - 22., outer.min.y + 30.);
        let peak = h.iter().map(|r| r.down.max(r.up.unwrap_or(0.))).fold(1f32, f32::max);
        let step = [1., 2., 5., 10., 20., 25., 50., 100., 200., 250., 500., 1000.]
            .into_iter()
            .find(|s| peak / s <= 4.)
            .unwrap_or(2500.);
        let top = step * (peak / step).ceil();
        let n = h.len();
        let px = |i: usize| if n == 1 { (x0 + x1) / 2. } else { x0 + (x1 - x0) * i as f32 / (n - 1) as f32 };
        let py = |v: f32| y0 + (y1 - y0) * v / top;

        let mut t = 0.;
        while t <= top {
            p.line_segment([pos2(x0, py(t)), pos2(x1, py(t))], Stroke::new(1., pal.line));
            p.text(pos2(x0 - forward * 8., py(t)), end_al, format!("{t:.0}"), mono(10.), pal.faint);
            t += step;
        }
        p.text(pos2(x0, y0 + 13.), start_al, date(h[0].time), mono(10.), pal.faint);
        if n > 1 {
            p.text(pos2(x1, y0 + 13.), end_al, date(h[n - 1].time), mono(10.), pal.faint);
        }

        let hover = resp.hover_pos().map(|pos| (0..n).min_by(|a, b| (px(*a) - pos.x).abs().total_cmp(&(px(*b) - pos.x).abs())).unwrap_or(0));
        if let Some(i) = hover {
            p.line_segment([pos2(px(i), y0), pos2(px(i), y1)], Stroke::new(1., pal.line_2));
        }

        for (color, values) in [
            (DOWN[0], h.iter().map(|r| Some(r.down)).collect::<Vec<_>>()),
            (UP[0], h.iter().map(|r| r.up).collect::<Vec<_>>()),
        ] {
            let points: Vec<Pos2> = values.iter().enumerate().filter_map(|(i, v)| Some(pos2(px(i), py((*v)?)))).collect();
            if points.len() > 1 {
                p.add(egui::Shape::line(points, Stroke::new(2., color)));
            }
            for (i, pt) in values.iter().enumerate().filter_map(|(i, v)| v.map(|v| (i, pos2(px(i), py(v))))) {
                let r = if hover == Some(i) { 5.5 } else { 4. };
                p.circle(pt, r, color, Stroke::new(1.5, pal.panel));
            }
        }

        if let Some(i) = hover {
            let r = h[i];
            resp.on_hover_ui_at_pointer(|ui| {
                ui.label(egui::RichText::new(date(r.time)).strong());
                ui.label(format!("{}  {:.2} Mbps", tr("التنزيل", "download"), r.down));
                ui.label(format!("{}  {}", tr("الرفع", "upload"), r.up.map_or("—".into(), |v| format!("{v:.2} Mbps"))));
                ui.label(format!("{}  {} ms", tr("البنق", "ping"), ms(r.ping)));
            });
        }

        ui.add_space(14.);
        let cols = [0., 0.36, 0.6, 0.84];
        let x = |r: Rect, f: f32| if ar() { r.max.x - r.width() * f } else { r.min.x + r.width() * f };
        let row = |ui: &mut egui::Ui, h: f32| ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover()).0;
        let r = row(ui, 20.);
        let header = [tr("الوقت", "time"), tr("التنزيل Mbps", "download Mbps"), tr("الرفع Mbps", "upload Mbps"), tr("البنق ms", "ping ms")];
        for (f, t) in cols.iter().zip(header) {
            ui.painter().text(pos2(x(r, *f), r.center().y), start_al, t, mono(11.), pal.faint);
        }
        for rec in h.iter().rev().take(6) {
            let r = row(ui, 24.);
            let p = ui.painter();
            p.line_segment([r.left_top(), r.right_top()], Stroke::new(1., pal.line));
            let y = r.center().y;
            p.text(pos2(x(r, cols[0]), y), start_al, date(rec.time), mono(12.), pal.muted);
            p.text(pos2(x(r, cols[1]), y), start_al, format!("{:.2}", rec.down), mono(12.), pal.text);
            p.text(pos2(x(r, cols[2]), y), start_al, rec.up.map_or("—".into(), |v| format!("{v:.2}")), mono(12.), pal.text);
            p.text(pos2(x(r, cols[3]), y), start_al, ms(rec.ping), mono(12.), pal.text);
        }
    }

    fn server_checks(&mut self, ui: &mut egui::Ui, pal: &Palette) {
        let servers = self.known_servers().to_vec();
        let checks = self.checks.lock().unwrap().clone();
        let running = checks.values().any(Option::is_none);

        ui.horizontal(|ui| {
            ui.with_layout(crate::lang::fwd(egui::Align::Center), |ui| {
                let button = egui::Button::new(tr("افحص السيرفرات", "test the servers"));
                if ui.add_enabled(!running && !servers.is_empty(), button).clicked() {
                    check_servers(&self.checks, &servers, ui.ctx());
                }
                let button = egui::Button::new(tr("اقترح علي وش أحظر", "suggest what to block"));
                if ui.add_enabled(!servers.is_empty(), button).clicked() {
                    self.open_setup(ui.ctx());
                }
                if running {
                    ui.spinner();
                }
            })
        });
        ui.label(
            egui::RichText::new(tr(
                "يرسل 20 بنق لكل سيرفر ويقيس التذبذب وفقد الحزم، وهم اللي تحس فيهم كلاق داخل القيم",
                "sends 20 pings to every server and measures jitter and packet loss, the things you feel as lag in a match",
            ))
            .weak()
            .size(11.),
        );
        ui.add_space(8.);

        let cols = [0., 0.44, 0.58, 0.72, 0.84];
        let x = |r: Rect, f: f32| if ar() { r.max.x - r.width() * f } else { r.min.x + r.width() * f };
        let align = if ar() { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER };
        let row = |ui: &mut egui::Ui, h: f32| ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover()).0;

        let r = row(ui, 20.);
        let header = [tr("السيرفر", "server"), tr("البنق", "ping"), tr("التذبذب", "jitter"), tr("الفقد", "loss"), tr("التقييم", "rating")];
        for (f, t) in cols.iter().zip(header) {
            ui.painter().text(pos2(x(r, *f), r.center().y), align, t, mono(11.), pal.faint);
        }

        let result = |s: &KnownServer| checks.get(&s.ping).and_then(|q| q.as_ref()?.as_ref().ok().copied());
        let mut order: Vec<&KnownServer> = servers.iter().collect();
        order.sort_by(|a, b| {
            let k = |s: &KnownServer| result(s).map_or(f32::MAX, |q| q.ping);
            k(a).total_cmp(&k(b))
        });

        for s in order {
            let r = row(ui, 28.);
            let p = ui.painter().clone();
            let y = r.center().y;
            p.line_segment([r.left_top(), r.right_top()], Stroke::new(1., pal.line));

            let side = if ar() { -1. } else { 1. };
            let mut nx = x(r, 0.);
            if let Some(src) = launcher::server_geo(s).and_then(|(_, f)| assets::flag(f)) {
                let fr = Rect::from_center_size(pos2(nx + side * 10., y), vec2(20., 14.));
                egui::Image::new(src).corner_radius(2.).paint_at(ui, fr);
                nx += side * 28.;
            }
            let blocked = self.config.desired_blocked_servers.has(s);
            p.text(pos2(nx, y), align, &s.title, mono(13.), if blocked { pal.red } else { pal.text });

            let cell = |f: f32, t: String, c: Color32| {
                p.text(pos2(x(r, f), y), align, t, mono(12.), c);
            };
            match checks.get(&s.ping) {
                None => {}
                Some(None) => cell(cols[1], "…".into(), pal.faint),
                Some(Some(Err(_))) => cell(cols[4], tr("ما يرد", "no reply").into(), pal.faint),
                Some(Some(Ok(q))) => {
                    cell(cols[1], format!("{:.0}", q.ping), pal.text);
                    cell(cols[2], format!("{:.1}", q.jitter), pal.text);
                    cell(cols[3], format!("{:.0}%", q.loss), if q.loss > 0. { pal.red } else { pal.text });
                    let (t, c) = if q.loss > 2. || q.jitter > 15. || q.ping > 150. {
                        (tr("ضعيف", "poor"), pal.red)
                    } else if q.loss == 0. && q.jitter < 5. && q.ping < 60. {
                        (tr("ممتاز", "great"), pal.accent)
                    } else {
                        (tr("كويس", "good"), pal.gold)
                    };
                    cell(cols[4], t.into(), c);
                }
            }
        }
    }

    pub(crate) fn open_setup(&mut self, ctx: &egui::Context) {
        self.setup_modal = true;
        self.setup_pick = None;
        let servers = self.known_servers().to_vec();
        check_servers(&self.checks, &servers, ctx);
    }

    fn close_setup(&mut self, ctx: &egui::Context) {
        self.setup_modal = false;
        if !self.config.welcomed || self.config.toured {
            return;
        }
        self.start_tour(ctx);
    }

    pub(crate) fn setup(&mut self, ui: &mut egui::Ui) {
        let pal = crate::visuals::palette(self.get_theme(ui));
        let mut servers = self.known_servers().to_vec();
        let checks = self.checks.lock().unwrap().clone();
        let tested = checks.values().filter(|q| q.is_some()).count();
        let done = !checks.is_empty() && tested == checks.len();
        if done && self.setup_pick.is_none() {
            self.setup_pick = Some(suggest(&servers, &checks));
        }
        let ping = |s: &KnownServer| checks.get(&s.ping).and_then(|q| q.as_ref()?.as_ref().ok().copied()).map(|q| q.ping);
        servers.sort_by(|a, b| ping(a).unwrap_or(f32::MAX).total_cmp(&ping(b).unwrap_or(f32::MAX)));

        let (mut apply, mut close) = (false, false);
        let modal = egui::Modal::new(egui::Id::new("setup")).show(ui.ctx(), |ui| {
            ui.set_width(380.);
            ui.with_layout(egui::Layout::top_down(crate::lang::start()), |ui| {
                ui.heading(tr("الإعداد المقترح", "suggested setup"));
                ui.add_space(4.);
                ui.label(tr(
                    "فحصت اتصالك بكل السيرفرات، وأنصحك تفتح القريبة منك وتحظر البعيدة عشان تلعب ببنق أقل",
                    "i tested your connection to every server. keep the close ones open and block the far ones to play with lower ping",
                ));
                ui.add_space(8.);

                match &mut self.setup_pick {
                    None => {
                        ui.horizontal(|ui| {
                            ui.with_layout(crate::lang::fwd(egui::Align::Center), |ui| {
                                ui.spinner();
                                ui.label(format!("{}  {tested}/{}", tr("يفحص السيرفرات", "testing servers"), checks.len()));
                            })
                        });
                    }
                    Some(pick) => {
                        egui::ScrollArea::vertical()
                            .max_height(300.)
                            .content_margin(egui::Margin { right: 14, ..Default::default() })
                            .show(ui, |ui| {
                                for s in &servers {
                                    let open = !pick.has(s);
                                    let row = ui.horizontal(|ui| {
                                        ui.with_layout(crate::lang::fwd(egui::Align::Center), |ui| {
                                            if let Some(src) = launcher::server_geo(s).and_then(|(_, f)| assets::flag(f)) {
                                                ui.add(egui::Image::new(src).fit_to_exact_size(vec2(20., 14.)).corner_radius(2.));
                                            }
                                            ui.label(egui::RichText::new(&s.title).size(13.));
                                            ui.label(egui::RichText::new(ms(ping(s))).color(pal.muted).size(12.));
                                            ui.with_layout(crate::lang::back(egui::Align::Center), |ui| {
                                                let (t, c) = if open { (tr("مفتوح", "open"), pal.accent) } else { (tr("محظور", "blocked"), pal.red) };
                                                ui.label(egui::RichText::new(t).color(c).strong());
                                            });
                                        })
                                    });
                                    let click = ui
                                        .interact(row.response.rect, egui::Id::new(("setup_row", s.bit)), Sense::click())
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if click.clicked() {
                                        let mut next = *pick;
                                        next.toggle(s);
                                        if !servers.iter().all(|x| next.has(x)) {
                                            *pick = next;
                                        }
                                    }
                                }
                            });
                        ui.label(egui::RichText::new(tr("اضغط على أي سيرفر عشان تغيّره", "click any server to change it")).weak().size(11.));
                    }
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.with_layout(crate::lang::fwd(egui::Align::Center), |ui| {
                        if ui.add_enabled(done, egui::Button::new(tr("طبّق", "apply"))).clicked() {
                            apply = true;
                        }
                        if ui.button(tr("بختار بنفسي", "i'll choose myself")).clicked() {
                            close = true;
                        }
                    })
                });
            });
        });

        if apply && let Some(pick) = self.setup_pick {
            self.config.desired_blocked_servers = pick;
            self.apply_blocked_servers_to_firewall();
            close = true;
        }
        if close || modal.should_close() {
            self.close_setup(ui.ctx());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(ping: &str, bit: u8) -> KnownServer {
        KnownServer { title: ping.into(), token: ping.into(), block: String::new(), bit, ping: ping.into() }
    }

    fn checks(rows: &[(&str, f32, f32, f32)]) -> Checks {
        rows.iter()
            .map(|(ip, ping, jitter, loss)| (ip.to_string(), Some(Ok(ping::Quality { ping: *ping, jitter: *jitter, loss: *loss }))))
            .collect()
    }

    #[test]
    fn dial_scale() {
        assert_eq!(frac(0.), 0.);
        assert_eq!(frac(100.), 0.5);
        assert_eq!(frac(1000.), 1.);
        assert_eq!(frac(5000.), 1.);
        assert!((frac(30.) - 2.5 / 8.).abs() < 1e-6);
    }

    #[test]
    fn suggestion() {
        let servers = [server("me", 1), server("eu", 2), server("us", 3), server("far", 4)];
        let sel = suggest(&servers, &checks(&[("me", 41., 3., 0.), ("eu", 70., 4., 0.), ("us", 172., 5., 0.), ("far", 50., 30., 0.)]));
        assert!(!sel.has(&servers[0]));
        assert!(!sel.has(&servers[1]));
        assert!(sel.has(&servers[2]));
        assert!(sel.has(&servers[3]));

        let all_bad = suggest(&servers, &checks(&[("me", 41., 40., 9.), ("eu", 70., 40., 9.)]));
        assert_eq!(all_bad.bits(), 0);
    }
}
