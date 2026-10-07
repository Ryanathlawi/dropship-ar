#[cfg(target_os = "windows")]
use crate::trf;
use crate::firewall;

const PREVIOUS_DROPSHIP_GROUP_NAMES: &[&str] = &[
    "stormy/dropship",    // v2
    "stormy.gg/dropship", // v1
];

const MINA_DEFAULT_GROUPING_NAME: &str = "_MINA Overwatch 2-Server-Selector";

/*TODO
    [ ] fork windows_firewall and have an iter filter closure so we don't have to allocate memory for all rules.
        i did this in dropship v2, that crate does not have this feature
*/

/// is there existing OUTGOING rules from a previous dropship version?
/// matches by group name only. rules may be disabled. rules may not have an application
#[cfg(target_os = "windows")]
pub fn get_legacy_dropship_rules()
-> windows::core::Result<Vec<windows::Win32::NetworkManagement::WindowsFirewall::INetFwRule>> {
    let iter = firewall::win::iter_rules()?
        .with_direction(firewall::win::Direction::Outgoing)
        .filter_map(|r| {
            let g = unsafe { r.Grouping().ok()?.to_string() };

            if PREVIOUS_DROPSHIP_GROUP_NAMES.contains(&g.as_str()) {
                Some(r)
            } else {
                None
            }
        });

    Ok(iter.collect())
}

/// is there existing INCOMING or OUTGOING firewall rules detected from mina?
/// matches by group name only. rules may be disabled. rules may not have an application
#[cfg(target_os = "windows")]
pub fn get_mina_rules()
-> windows::core::Result<Vec<windows::Win32::NetworkManagement::WindowsFirewall::INetFwRule>> {
    let iter = firewall::win::iter_rules()?
        .with_direction(firewall::win::Direction::Outgoing)
        .with_group(MINA_DEFAULT_GROUPING_NAME);

    Ok(iter.collect())
}

#[cfg(target_os = "windows")]
pub fn delete_legacy_rules() -> windows::core::Result<()> {
    // let matched = [get_legacy_dropship_rules()?, get_mina_rules()?]
    //     .into_iter()
    //     .flatten()
    //     .collect::<Vec<_>>();

    let mut matched = get_legacy_dropship_rules()?;
    matched.extend(get_mina_rules()?);

    // let enabled = matched.iter().filter(|r| *r.enabled()).collect::<Vec<_>>();
    // let disabled = matched.iter().filter(|r| !*r.enabled()).collect::<Vec<_>>();

    // if matched.len() > 0 {
    //     log::warn!("{}", {
    //         let mut t = format!(
    //             "<detected conflicting firewall entries> ({})\n",
    //             matched.len(),
    //         );

    //         t += &format!("  <enabled> ({})\n", enabled.len());
    //         for r in enabled.iter() {
    //             t += &format!(
    //                 "    .group {{ {} }}\n    .name {{ {} }}\n\n",
    //                 r.grouping().clone().unwrap_or_default(),
    //                 r.name()
    //             );
    //         }

    //         t += &format!("  <disabled> ({})\n", disabled.len());
    //         for r in disabled {
    //             t += &format!(
    //                 "    .group {{ {} }}\n    .name {{ {} }}\n\n",
    //                 r.grouping().clone().unwrap_or_default(),
    //                 r.name()
    //             );
    //         }

    //         t
    //     });

    //     log::warn!(
    //         "<حذف قواعد جدار حماية متعارضة> ({})",
    //         matched.len()
    //     );
    //     for r in matched.into_iter() {
    //         // {
    //         //     let t = format!("{:#?}", r).to_ascii_lowercase();
    //         //     log::debug!("{}", t);
    //         // }

    //         r.remove()?;
    //     }
    // }

    // if matched.len() > 0 {
    //     log::warn!("{}", {
    //         let mut t = format!(
    //             "<detected conflicting firewall entries> ({})\n",
    //             matched.len(),
    //         );
    //         for r in matched.iter() {
    //             unsafe {
    //                 t += &format!(
    //                     "    .group {{ {} }}\n    .name {{ {} }}\n\n",
    //                     r.Grouping().ok().clone().unwrap_or_default(),
    //                     r.Name().un
    //                 );
    //             }
    //         }
    //     });
    // }

    if matched.len() > 0 {
        log::warn!("{}", trf!("<حذف قواعد جدار حماية متعارضة> ({})", "<removing conflicting firewall rules> ({})",
            matched.len()
        ));
        for r in matched.into_iter() {
            // {
            //     let t = format!("{:#?}", r).to_ascii_lowercase();
            //     log::debug!("{}", t);
            // }

            if let Err(e) = firewall::win::delete_rule(r) {
                log::error!("{}", e);
            }
        }
    }

    // moved to wfp
    firewall::delete_dropship_rules()?;

    Ok(())
}

// حظر تركته برامج حظر ثانية في جدار حماية ويندوز: يقفل السيرفر عن اللعبة حتى لو هو مفتوح
// في dropship، لأن dropship ما يشيل إلا حظره هو (في WFP)

/// قاعدة من برنامج ثاني ما زالت تحظر سيرفرات أوفرواتش
pub struct ForeignBlock {
    pub rule: String,
    /// `bit` لكل سيرفر تحظره
    pub servers: Vec<u8>,
}

/// قواعد الحظر الصادرة المفعّلة من برامج ثانية، اللي توصل للعبة (كل البرامج أو أوفرواتش
/// نفسه) وتغطي شي من `servers`. قواعد dropship القديمة وMINA يحذفها `delete_legacy_rules`
/// ومع `disable` تنطفي كمان، والإطفاء يتراجع عنه صاحب الجهاز من جدار الحماية بعكس الحذف
#[cfg(target_os = "windows")]
pub fn foreign_blocks(servers: &[crate::api::KnownServer], disable: bool) -> windows::core::Result<Vec<ForeignBlock>> {
    use windows::Win32::NetworkManagement::WindowsFirewall::NET_FW_ACTION_BLOCK;

    let nets: Vec<(u8, Span)> = servers
        .iter()
        .flat_map(|s| s.block.split(',').filter_map(span).map(move |n| (s.bit, n)))
        .collect();

    let mut found = vec![];
    for r in firewall::win::iter_rules()?.with_direction(firewall::win::Direction::Outgoing) {
        let (on, group, app, remote) = unsafe {
            (
                r.Enabled().is_ok_and(|e| e.as_bool()) && r.Action().is_ok_and(|a| a == NET_FW_ACTION_BLOCK),
                r.Grouping().map(|g| g.to_string()).unwrap_or_default(),
                r.ApplicationName().map(|a| a.to_string().to_lowercase()).unwrap_or_default(),
                r.RemoteAddresses().map(|a| a.to_string()).unwrap_or_default(),
            )
        };
        let legacy = group == firewall::DROPSHIP_GROUP_NAME
            || PREVIOUS_DROPSHIP_GROUP_NAMES.contains(&group.as_str())
            || group == MINA_DEFAULT_GROUPING_NAME;
        if !on || legacy || !(app.is_empty() || app.ends_with("overwatch.exe")) {
            continue;
        }
        let hit = covered(&remote, &nets);
        if hit.is_empty() {
            continue;
        }

        let rule = unsafe { r.Name() }.map(|n| n.to_string()).unwrap_or_default();
        if disable {
            match unsafe { r.SetEnabled(false.into()) } {
                Ok(()) => {
                    log::info!("{}", trf!("شلت حظر «{}» اللي حطه برنامج ثاني", "turned off \"{}\", a block left by another program", rule));
                    continue;
                }
                Err(e) => log::error!("{}", trf!("ما قدرت أشيل حظر «{}» ({})", "couldn't turn off \"{}\" ({})", rule, e)),
            }
        }
        found.push(ForeignBlock { rule, servers: hit });
    }

    Ok(found)
}

/// مدى عناوين من الأول للأخير
type Span = (std::net::IpAddr, std::net::IpAddr);

/// عنوان واحد من عناوين قاعدة في جدار الحماية كمدى: "1.2.3.4" و"1.2.3.0/24"
/// و"1.2.3.0/255.255.255.0" و"1.2.3.4-1.2.3.9"، والكلمات مثل "*" و"LocalSubnet" ترجع `None`
fn span(entry: &str) -> Option<Span> {
    let entry = entry.trim();
    if let Some((a, b)) = entry.split_once('-') {
        return Some((a.trim().parse().ok()?, b.trim().parse().ok()?));
    }
    let (addr, bits) = entry.split_once('/').unwrap_or((entry, ""));
    let addr: std::net::IpAddr = addr.parse().ok()?;
    let bits = match bits.parse::<std::net::Ipv4Addr>() {
        Ok(mask) => u32::from(mask).leading_ones() as u8,
        Err(_) if bits.is_empty() => if addr.is_ipv4() { 32 } else { 128 },
        Err(_) => bits.parse().ok()?,
    };
    let net = ipnet::IpNet::new(addr, bits).ok()?;
    Some((net.network(), net.broadcast()))
}

/// أوسع من /8 (أو /32 في IPv6): قاعدة عامة تقفل الإنترنت كله، مثل صندوق برنامج لمستخدم
/// معيّن، مو بلوكر سيرفرات، فلا نعدّها ولا نعرض نطفيها
fn wide((a, b): Span) -> bool {
    use std::net::IpAddr::{V4, V6};
    match (a, b) {
        (V4(a), V4(b)) => u32::from(b).saturating_sub(u32::from(a)) >= 1 << 24,
        (V6(a), V6(b)) => u128::from(b).saturating_sub(u128::from(a)) >= 1 << 96,
        _ => true,
    }
}

/// السيرفرات اللي تتقاطع شبكاتها مع أي عنوان مو عام في `remote` (كل سيرفر مرة)
fn covered(remote: &str, nets: &[(u8, Span)]) -> Vec<u8> {
    let entries: Vec<Span> = remote.split(',').filter_map(span).filter(|e| !wide(*e)).collect();
    let mut bits: Vec<u8> = nets
        .iter()
        .filter(|(_, n)| entries.iter().any(|e| e.0 <= n.1 && n.0 <= e.1))
        .map(|(bit, _)| *bit)
        .collect();
    // شبكات السيرفر الواحد جنب بعض، فالتكرار متجاور
    bits.dedup();
    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> std::net::IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn spans() {
        assert_eq!(span("34.166.0.0/255.255.0.0"), Some((ip("34.166.0.0"), ip("34.166.255.255"))));
        assert_eq!(span("34.166.0.0/16"), span("34.166.0.0/255.255.0.0"));
        assert_eq!(span("34.166.0.84"), Some((ip("34.166.0.84"), ip("34.166.0.84"))));
        assert_eq!(span("34.166.0.84/255.255.255.255"), span("34.166.0.84"));
        assert_eq!(span(" 5.42.160.1-5.42.160.9 "), Some((ip("5.42.160.1"), ip("5.42.160.9"))));
        assert_eq!(span("2600:1900:4150::/44"), Some((ip("2600:1900:4150::"), ip("2600:1900:415f:ffff:ffff:ffff:ffff:ffff"))));
        for keyword in ["*", "LocalSubnet", "DNS", "", "1.2.3.0/33", "1.2.3.0/x"] {
            assert_eq!(span(keyword), None, "{keyword}");
        }
    }

    #[test]
    fn covered_servers() {
        // 4 السعودية و3 فنلندا، كما في قائمة dropship
        let nets: Vec<(u8, Span)> = [(4, "34.166.0.0/16"), (4, "8.228.192.0/19"), (3, "34.88.0.0/16"), (3, "2600:1900:4150::/44")]
            .iter()
            .map(|(bit, n)| (*bit, span(n).unwrap()))
            .collect();

        // بلوكر يحظر كم آيبي من الشرق الأوسط بصيغة ويندوز
        assert_eq!(covered("34.166.0.84/255.255.255.255,1.1.1.1", &nets), vec![4]);
        assert_eq!(covered("8.228.200.0-8.228.200.255,34.166.1.0/24", &nets), vec![4]);
        // مدى عريض فوق الاثنين، لين /8
        assert_eq!(covered("34.0.0.0-34.200.0.0", &nets), vec![4, 3]);
        assert_eq!(covered("34.0.0.0/8", &nets), vec![4, 3]);
        assert_eq!(covered("2600:1900:4150::1", &nets), vec![3]);
        // كل العناوين أو الكلمات أو شي ما له علاقة ما يُحسب
        for remote in ["*", "LocalSubnet,10.0.0.0/8", "", "34.167.0.0/16"] {
            assert!(covered(remote, &nets).is_empty(), "{remote}");
        }
        // قاعدة تقفل الإنترنت كله (إلا الجهاز نفسه) مو بلوكر، حتى لو غطّت كل شي
        let everything = "0.0.0.0-126.255.255.255,128.0.0.0-255.255.255.255,::-::,::2-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff";
        for remote in [everything, "34.0.0.0/7", "2600::/16"] {
            assert!(covered(remote, &nets).is_empty(), "{remote}");
        }
    }
}
