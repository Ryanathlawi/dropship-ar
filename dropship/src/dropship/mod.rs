use std::time::Duration;

mod commands;
mod events;

pub use commands::{Command, start_processing_commands};
pub use events::{Event, EventSender, process_events};

mod startup_dispatch;
pub use startup_dispatch::startup_dispatch;

// version checks are performed this often
pub const INTERVAL_VERSION_CHECK: Duration = Duration::from_secs(9000);
pub const INTERVAL_API_CHECK: Duration = Duration::from_secs(900);
pub const INTERVAL_PROCESS_CHECK: Duration = Duration::from_millis(900);

// pub const DROPSHIP_API_URL: &str = "http://127.0.0.1:9999/dropship.json";
pub const DROPSHIP_API_URL: &str = "https://stowmyy.github.io/dropship/ips.json";

// النسخة العربية تتحدث من مستودعها الخاص لا من المستودع الأصلي
pub const UPDATE_URI: &str = "https://api.github.com/repos/Ryanathlawi/dropship-ar/releases/latest";
pub const GITHUB_URI: &str = "https://github.com/Ryanathlawi/dropship-ar";
pub const UPSTREAM_GITHUB_URI: &str = "https://github.com/stowmyy/dropship";
// اسم الملف في صفحة الإصدارات يختلف بين النسختين حتى يحدّث كل واحد نفسه
pub const BINARY_NAME: &str = if cfg!(feature = "animations") {
    "dropship-ar-animated.exe"
} else {
    "dropship-ar.exe"
};

pub const DISCORD_INVITE_LINK: &str = "https://discord.gg/QYrF8CVhbC";
