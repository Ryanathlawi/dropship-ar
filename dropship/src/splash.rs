use crate::app::TemplateApp;
use crate::lang::tr;
use crate::{dropship, trf, visuals};
use eframe::egui::{self, Align2, FontFamily, FontId, Rect, Sense, pos2, vec2};
use std::f32::consts::TAU;

fn ease_out(t: f32) -> f32 {
    1. - (1. - t.clamp(0., 1.)).powi(3)
}

fn ease_back(t: f32) -> f32 {
    let t = t.clamp(0., 1.) - 1.;
    1. + 2.70158 * t.powi(3) + 1.70158 * t.powi(2)
}

fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

impl TemplateApp {
    pub(crate) fn splash(&mut self, ui: &mut egui::Ui, full: Rect) {
        let now = ui.input(|i| i.time) as f32;
        let t = now - *self.splash_start.get_or_insert(now);
        let ready = !self.known_servers().is_empty();
        if self.splash_out.is_none() && t >= 1.6 && (ready || t >= 3.5) {
            self.splash_out = Some(t);
        }
        let fade = self.splash_out.map_or(1., |o| 1. - ease_out((t - o) / 0.4));
        if fade <= 0. {
            self.splash_done = true;
            return;
        }
        ui.ctx().request_repaint();

        let pal = visuals::palette(self.get_theme(ui));
        egui::Area::new(egui::Id::new("splash"))
            .order(egui::Order::Tooltip)
            .fixed_pos(full.min)
            .show(ui.ctx(), |ui| {
                ui.set_opacity(fade);
                let (rect, _) = ui.allocate_exact_size(full.size(), Sense::click_and_drag());
                let p = ui.painter().clone();
                p.rect_filled(rect, 0., pal.bg);
                let c = rect.center() - vec2(0., 30.);

                let breath = 0.5 + 0.5 * (t * 2.4).sin();
                for k in 1..=8 {
                    let r = 70. + k as f32 * 14. + breath * 6.;
                    p.circle_filled(c, r, pal.accent.gamma_multiply(0.012 * (9 - k) as f32 / 8.));
                }

                let appear = ease_out(t / 0.6);
                let head = t * 3.2;
                for i in 0..60 {
                    let a = i as f32 / 60. * TAU;
                    let behind = (head - a).rem_euclid(TAU) / TAU;
                    let glow = (1. - behind).powi(3);
                    let at = c + vec2(a.cos(), a.sin()) * 92. * (0.85 + 0.15 * appear);
                    p.circle_filled(at, 1.6 + 1.4 * glow, pal.accent.gamma_multiply((0.15 + 0.85 * glow) * appear));
                }

                let s = ease_back(t / 0.55);
                let logo = Rect::from_center_size(c, vec2(76., 76.) * s);
                p.rect_filled(logo, 22. * s, pal.accent_deep);
                egui::Image::new(egui::include_image!("../assets/white-bolts.png")).paint_at(ui, logo.shrink(18. * s));

                let rise = |delay: f32| ease_out((t - delay) / 0.45);
                let a1 = rise(0.25);
                p.text(c + vec2(0., 120. + 10. * (1. - a1)), Align2::CENTER_CENTER, "dropship", font(30.), pal.text.gamma_multiply(a1));
                let a2 = rise(0.4);
                p.text(c + vec2(0., 152. + 10. * (1. - a2)), Align2::CENTER_CENTER, tr("النسخة العربية", "Athlawi edition"), font(15.), pal.accent.gamma_multiply(a2));

                let a3 = rise(0.55);
                let bar = Rect::from_center_size(c + vec2(0., 196.), vec2(170., 3.));
                p.rect_filled(bar, 2., pal.line.gamma_multiply(a3));
                let progress = ease_out(t / 1.6).min(if ready { 1. } else { 0.85 });
                let w = bar.width() * progress;
                let fill = if crate::lang::ar() {
                    Rect::from_min_max(pos2(bar.max.x - w, bar.min.y), bar.max)
                } else {
                    Rect::from_min_size(bar.min, vec2(w, bar.height()))
                };
                p.rect_filled(fill, 2., pal.accent.gamma_multiply(a3));
                let status = if ready { tr("جاهز", "ready") } else { tr("يجهّز السيرفرات", "getting servers ready") };
                p.text(c + vec2(0., 216.), Align2::CENTER_CENTER, status, font(12.), pal.faint.gamma_multiply(a3));

                let credit = trf!("تطوير {}", "made by {}", tr(dropship::AUTHOR_AR, dropship::AUTHOR));
                p.text(pos2(rect.center().x, rect.max.y - 24.), Align2::CENTER_CENTER, credit, font(11.), pal.faint.gamma_multiply(a2));
            });
    }
}
