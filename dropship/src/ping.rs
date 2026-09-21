//! قياس البنق عبر ICMP بواجهة ويندوز (`IcmpSendEcho`، نفس ما يستخدمه ping.exe): بلا سوكت خام ولا صلاحيات.
//!
//! عنوان الـ ping القادم من الـ API لا يرد أحيانًا (عناوين Google Cloud كثير منها يتجاهل ICMP)،
//! فنبحث عن عنوان يرد داخل نطاقات السيرفر نفسه (`block`) — نفس شبكة سيرفرات اللعبة — ونحفظه.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
    sync::{LazyLock, Mutex},
};
use tokio::time;
use windows::Win32::NetworkManagement::IpHelper::{
    ICMP_ECHO_REPLY, IP_OPTION_INFORMATION, IP_SUCCESS, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
};

pub const PING_TIMEOUT: time::Duration = time::Duration::from_millis(1500);
const PING_INTERVAL: time::Duration = time::Duration::from_millis(700);
const N_PINGS: u16 = 3;

/// العنوان الذي ثبت أنه يرد لكل سيرفر (مفتاحه عنوان الـ ping من الـ API)
static TARGET: LazyLock<Mutex<HashMap<String, Ipv4Addr>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// `ip` عنوان الـ ping من الـ API، و`block` نطاقات السيرفر (CIDR مفصولة بفواصل) للبحث عن بديل يرد
pub async fn ping_server(ip: &String, block: &str) -> Result<f32, String> {
    // ما ثبت أنه يرد من قبل
    let known = TARGET.lock().unwrap().get(ip).copied();
    if let Some(addr) = known {
        if let Some(ms) = echo(addr, N_PINGS).await? {
            return Ok(ms);
        }
    }
    // عنوان الـ API ثم مرشّحون داخل نطاقات السيرفر، أول من يرد يُعتمد
    for addr in std::iter::once(IpAddr::from_str(ip).map_err(|e| e.to_string())?)
        .filter_map(v4)
        .chain(candidates(block))
    {
        if Some(addr) == known {
            continue;
        }
        if let Some(ms) = echo(addr, 1).await? {
            TARGET.lock().unwrap().insert(ip.clone(), addr);
            return Ok(ms);
        }
    }
    Err(format!("<{ip}> فشل قياس البنق"))
}

fn v4(a: IpAddr) -> Option<Ipv4Addr> {
    match a {
        IpAddr::V4(a) => Some(a),
        IpAddr::V6(_) => None,
    }
}

/// عناوين محتملة داخل النطاقات: أوائل كل نطاق، وأول الشبكة الفرعية التالية في النطاقات الواسعة
fn candidates(block: &str) -> impl Iterator<Item = Ipv4Addr> + '_ {
    block
        .split(',')
        .filter_map(|c| ipnet::Ipv4Net::from_str(c.trim()).ok())
        .flat_map(|net| {
            let base = u32::from(net.network());
            let size = 1u32 << (32 - net.prefix_len());
            [1u32, 2, 10, 256 + 1, 65_536 + 1]
                .into_iter()
                .filter(move |o| *o < size.saturating_sub(1))
                .map(move |o| Ipv4Addr::from(base + o))
        })
        .take(24)
}

/// يرسل `n` رسائل ويعيد متوسط ما رُدّ عليه، أو `None` لو لم يرد أحد
async fn echo(addr: Ipv4Addr, n: u16) -> Result<Option<f32>, String> {
    let (mut sum, mut replies) = (0f32, 0u16);
    for i in 0..n {
        if i > 0 {
            time::sleep(PING_INTERVAL).await;
        }
        let r = tokio::task::spawn_blocking(move || send_echo(addr))
            .await
            .map_err(|e| e.to_string())??;
        if let Some(ms) = r {
            sum += ms;
            replies += 1;
        }
    }
    Ok((replies > 0).then(|| sum / replies as f32))
}

/// رسالة واحدة (تحجب الخيط، لذلك تُستدعى من spawn_blocking)
fn send_echo(addr: Ipv4Addr) -> Result<Option<f32>, String> {
    let data = [0u8; 32];
    // الرد + البيانات + هامش لرسائل الخطأ كما توصي وثائق ويندوز
    let mut reply = vec![0u8; std::mem::size_of::<ICMP_ECHO_REPLY>() + data.len() + 8];
    let opts = IP_OPTION_INFORMATION { Ttl: 128, ..Default::default() };
    // SAFETY: استدعاءات Win32 موثّقة؛ المؤشرات إلى مصفوفات حيّة طوال الاستدعاء، ونغلق المقبض دائمًا
    unsafe {
        let handle = IcmpCreateFile().map_err(|e| e.to_string())?;
        let n = IcmpSendEcho(
            handle,
            u32::from_ne_bytes(addr.octets()),
            data.as_ptr().cast(),
            data.len() as u16,
            Some(&opts),
            reply.as_mut_ptr().cast(),
            reply.len() as u32,
            PING_TIMEOUT.as_millis() as u32,
        );
        let _ = IcmpCloseHandle(handle);
        if n == 0 {
            return Ok(None); // مهلة أو لا يمكن الوصول
        }
        let r: ICMP_ECHO_REPLY = std::ptr::read_unaligned(reply.as_ptr().cast());
        Ok((r.Status == IP_SUCCESS).then_some(r.RoundTripTime as f32))
    }
}
