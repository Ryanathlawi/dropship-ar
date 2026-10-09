use std::{collections::HashSet, path::PathBuf};

#[derive(Debug)]
pub enum ApplicationType {
    /// battle.net
    Blizzard,

    /// steam
    Valve,

    /// unknown
    Unknown,
}

// #[derive(Debug)]
// pub struct Application {
//     pub path: std::path::PathBuf,
//     pub ty: ApplicationType,
//     pub label: String,
// }

/// gets known overwatch application paths.
/// combines
///    - overwatch application rule paths
///    - dropship rule paths
///    - legacy dropship rule paths
///    - mina paths
///
/// and deduplicates them. filters out application paths that don't exist on the filesystem.
///
/// dropship rules should be created on these paths *before* any legacy rules are deleted
#[cfg(target_os = "windows")]
fn get_known_application_paths_from_firewall() -> windows::core::Result<Vec<std::path::PathBuf>> {
    let matched = [
        super::get_blizzard_overwatch_rules()?.collect(),
        super::get_dropship_rules()?.collect(),
        super::legacy::get_legacy_dropship_rules()?,
        super::legacy::get_mina_rules()?,
    ]
    .into_iter()
    .flatten()
    //
    // has an application path defined
    // .filter_map(|r| r.application_name().clone())
    // .map(std::path::PathBuf::from)
    .filter_map(|r| {
        let n = unsafe { r.ApplicationName().ok() };
        if let Some(n) = n {
            Some(super::win::bstr_to_path(&n))
        } else {
            None
        }
    })
    //
    // exe exists on the file system
    .filter(|path| path.exists())
    //
    // deduplicate
    .collect::<HashSet<_>>()
    .into_iter()
    .collect::<Vec<_>>();

    Ok(matched)
}

#[cfg(target_os = "windows")]
pub fn get_known_valid_application_paths(
    previously_known_paths: &HashSet<PathBuf>,
) -> windows::core::Result<HashSet<PathBuf>> {
    // FIXME do we need this every time?
    let paths_from_firewall = get_known_application_paths_from_firewall()?;

    let mut combined = [
        paths_from_firewall,
        installed_overwatch(),
        previously_known_paths
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
    ]
    .into_iter()
    .flatten()
    .collect::<HashSet<_>>();

    // TODO ensure files exist
    // FIXME
    // FIXME remove rules that are valid?

    combined.retain(|x| x.exists());

    Ok(combined)
}

/// أماكن أوفرواتش المثبّت، عشان الحظر يشتغل من أول قيم بدون ما ينتظر اللعبة تنفتح:
/// Battle.net يكتب مكانه في قائمة البرامج المثبّتة بويندوز، وSteam في مكتبات ألعابه
#[cfg(target_os = "windows")]
fn installed_overwatch() -> Vec<PathBuf> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use windows::core::w;

    let mut dirs = vec![];
    if let Some(dir) = reg_string(HKEY_LOCAL_MACHINE, w!(r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Overwatch"), w!("InstallLocation")) {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(steam) = reg_string(HKEY_CURRENT_USER, w!(r"Software\Valve\Steam"), w!("SteamPath")) {
        let vdf = std::fs::read_to_string(PathBuf::from(steam).join(r"steamapps\libraryfolders.vdf")).unwrap_or_default();
        dirs.extend(steam_libraries(&vdf).map(|lib| lib.join(r"steamapps\common\Overwatch")));
    }

    dirs.into_iter()
        .flat_map(|d| [d.join(r"_retail_\Overwatch.exe"), d.join("Overwatch.exe")])
        .filter(|p| p.exists())
        .collect()
}

/// مكتبات Steam من libraryfolders.vdf، كل وحدة في سطر مثل: "path"		"D:\\SteamLibrary"
fn steam_libraries(vdf: &str) -> impl Iterator<Item = PathBuf> + '_ {
    vdf.lines().filter_map(|line| match line.split('"').collect::<Vec<_>>()[..] {
        [_, "path", _, lib, ..] => Some(PathBuf::from(lib.replace(r"\\", r"\"))),
        _ => None,
    })
}

#[cfg(target_os = "windows")]
fn reg_string(root: windows::Win32::System::Registry::HKEY, key: windows::core::PCWSTR, value: windows::core::PCWSTR) -> Option<String> {
    use windows::Win32::System::Registry::{RRF_RT_REG_SZ, RegGetValueW};

    let mut buf = [0u16; 1024];
    let mut bytes = (buf.len() * 2) as u32;
    unsafe { RegGetValueW(root, key, value, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut bytes)) }
        .ok()
        .ok()?;
    // الطول بالبايت ويشمل الصفر في آخره
    let s = String::from_utf16_lossy(&buf[..(bytes as usize / 2).saturating_sub(1)]);
    (!s.is_empty()).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_library_paths() {
        // نفس شكل الملف الحقيقي: مسافات tab، وشرطات مكررة، وأرقام الألعاب تحت كل مكتبة
        let vdf = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\Program Files (x86)\Steam"
		"apps"
		{
			"2357570"		"80533892347"
		}
	}
	"1"
	{
		"path"		"S:\SteamLibrary"
	}
}"#;
        let libs: Vec<PathBuf> = steam_libraries(vdf).collect();
        assert_eq!(libs, [PathBuf::from(r"C:\Program Files (x86)\Steam"), PathBuf::from(r"S:\SteamLibrary")]);
    }

    #[test]
    #[ignore = "يقرا جهاز اللي يشغّله: cargo test -- --ignored --nocapture"]
    fn this_pc() {
        println!("{:?}", installed_overwatch());
    }
}
