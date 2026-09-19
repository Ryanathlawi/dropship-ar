use std::{collections::HashSet, fmt, path::PathBuf};

use strum::EnumMessage;

use crate::{api, overwatch::ServerSelection, update};

mod dispatch;

pub use dispatch::process_events;

/// Events are (intentionally) processed in the draw pass, so sending one must also
/// request a repaint. Otherwise results such as `ApiResponse` or `ProcessOpenStatusChange`
/// wait until the user focuses the window again.
#[derive(Clone)]
pub struct EventSender {
    tx: tokio::sync::mpsc::UnboundedSender<Event>,
    ctx: Option<eframe::egui::Context>,
}

impl EventSender {
    pub fn new(
        tx: tokio::sync::mpsc::UnboundedSender<Event>,
        ctx: Option<eframe::egui::Context>,
    ) -> Self {
        Self { tx, ctx }
    }

    pub fn send(&self, event: Event) -> Result<(), tokio::sync::mpsc::error::SendError<Event>> {
        let result = self.tx.send(event);
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
        result
    }
}

#[derive(strum::EnumMessage, strum::AsRefStr)] // "got .."
/// NOTE events are received (intentionally) in the draw. no system processing
/// should depend on an event being received. events won't be processed while
/// the application is minimized

pub enum Event {
    //
    #[strum(detailed_message = "وصلت آيبيات جديدة من الإنترنت")]
    ApiResponse(api::DropshipApiData),

    // #[strum(detailed_message  = "")]
    // PlayerUninstall {},
    // #[strum(detailed_message = "an update is available")]
    UpdateAvailable(update::AvailableUpdate),

    // pong
    Pong {
        ip: String,
        pong: Result<f32, String>,
    },

    // #[strum(detailed_message = "found new ips from online")]
    ApplicationUpdateStatusChange(update::UpdatingStatus),

    ProcessOpenStatusChange {
        process_name: String,
        open: bool,
        path: Option<PathBuf>,
    },

    FoundApplicationPaths(HashSet<PathBuf>),
    FirewallConfigApplied {
        blocked_servers: ServerSelection,
    },

    /// called when we want to visually communicate a loading state change.
    /// probably only going to use this for firewall changes
    DropshipLoadingStateChange(bool),

    #[strum(detailed_message = "أُضيف ملف تنفيذي")]
    AddedExecutable(std::path::PathBuf),

    ForceApplyFirewallRequested,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::UpdateAvailable(ref data) => write!(f, "الإصدار v{} متوفر", data.version),
            Self::ProcessOpenStatusChange {
                ref process_name,
                open,
                ..
            } => write!(
                f,
                "اللعبة \"{}\" {}",
                process_name,
                if open {
                    "مفتوحة. ما يمكن حظر السيرفرات واللعبة مفتوحة"
                } else {
                    "مغلقة. حظر السيرفرات متاح الآن"
                }
            ),
            // Self::FoundApplicationPaths { ref paths } => {
            Self::FoundApplicationPaths(ref paths) => {
                write!(
                    f,
                    "مسارات تنفيذية من جدار الحماية: {:#?}",
                    paths.iter().map(|x| x.display()).collect::<Vec<_>>()
                )
            }

            _ => write!(
                f,
                "{}",
                self.get_detailed_message().unwrap_or(self.as_ref())
            ),
        }
    }
}
