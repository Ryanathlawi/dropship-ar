use eframe::egui;

use crate::overwatch::ServerSelection;
use crate::{api, visuals};

use crate::assets;

/// draw the stars here
pub fn server_list_indicators(
    ui: &mut egui::Ui,
    servers: &[api::KnownServer],
    desired_blocked_servers: &ServerSelection,
    actually_blocked_servers: ServerSelection,
    //
    theme: visuals::Theme,
) {
    servers.into_iter().enumerate().for_each(|(i, server)| {
        let blocked = actually_blocked_servers.has(server);
        let pending = desired_blocked_servers.has(server) != actually_blocked_servers.has(server);

        let color = if pending {
            crate::visuals::color_secondary_faded(i)
        } else {
            if blocked {
                // ui.visuals().weak_text_color()
                visuals::from_theme_alpha(theme, 9)
            } else {
                crate::visuals::color_active(i)
            }
        };

        let text = if pending {
            format!("{} (بانتظار إغلاق اللعبة..)", server.token)
        } else {
            if blocked {
                format!("{} (محظور)", server.token)
            } else {
                server.token.clone()
            }
        };

        ui.add(
            egui::Image::new(assets::ICON_STAR)
                .fit_to_exact_size(egui::vec2(16., 16.))
                .tint(color),
        )
        .on_hover_text_at_pointer(text);
    });
}
