use std::{
    collections::HashSet,
    fmt,
    path::PathBuf,
    sync::{Arc, atomic},
};

use strum::EnumMessage;

use crate::api;

mod dispatch;

pub use dispatch::start_processing_commands;

#[derive(strum::EnumMessage, strum::AsRefStr)] // "failed .."
/// NOTE commands are received in a background task and processed asynchronously
/// events are sent to update the ui and persistence. events are not processed when
/// the app is minimized because events are (intentionally) processed in the draw
pub enum Command {
    /// web attempt api data sync
    #[strum(detailed_message = "جلب آيبيات جديدة من الإنترنت")]
    UpdateConfigFromRemote,

    /// web attempt version check
    #[strum(detailed_message = "التحقق من وجود إصدار جديد")]
    VersionCheck,

    // update the machine's firewall based on player config
    #[strum(detailed_message = "تحديث إعدادات جدار الحماية في هذا الجهاز")]
    ApplyFirewallConfig {
        blocked_servers: HashSet<api::KnownServer>,
        already_known_paths: HashSet<PathBuf>,
    },

    // ping
    Ping {
        ip: String,
    },

    // if a player hates dropship and wants to leave no trace
    // #[strum(detailed_message = "")]
    // TODO
    // Uninstall,

    //
    #[strum(detailed_message = "تحديث البرنامج")]
    ApplicationUpdate {
        binary_download: String,
        download_total_size: Arc<atomic::AtomicU64>,
        downloaded_size: Arc<atomic::AtomicU64>,
    },

    #[strum(detailed_message = "التحقق من الألعاب المفتوحة")]
    ProcessCheck {
        process_name: String,
    },

    // #[strum(detailed_message = "adding executable path to dropship")]
    AddExecutable {
        path: std::path::PathBuf,
    },

    ForceApplyFirewallRequested,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::AddExecutable { ref path } => {
                write!(
                    f,
                    "إضافة مسار ملف تنفيذي إلى dropship: \"{}\"",
                    path.display()
                )
            }
            // Self::VersionCheck => write!(f, "{}", self.get_detailed_message().unwrap_or(self.as_ref())),
            _ => write!(
                f,
                "{}",
                self.get_detailed_message().unwrap_or(self.as_ref())
            ),
        }
    }
}
