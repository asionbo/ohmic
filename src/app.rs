use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result};
use tokio::sync::{broadcast, mpsc, watch};

use crate::i18n::Lang;
use crate::server::{ClientToHost, HostToClient};

pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_CLIENTS: usize = 8;

/// 投递方式。仅剪贴板是所有平台的保底路径。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryMode {
    Paste,
    ClipboardOnly,
}

impl DeliveryMode {
    pub fn label(self, lang: Lang) -> &'static str {
        let s = lang.strings();
        match self {
            DeliveryMode::Paste => s.mode_paste,
            DeliveryMode::ClipboardOnly => s.mode_clipboard_only,
        }
    }
}

/// 主线程（托盘/注入）与 tokio 线程（HTTP/WS）之间共享的状态。
#[derive(Clone)]
pub struct Shared {
    pub port: u16,
    /// 运行时可通过托盘菜单切换；手机页/二维码页按请求时值渲染。
    pub lang: Arc<std::sync::RwLock<Lang>>,
    pub token: Arc<std::sync::RwLock<String>>,
    pub lan_ip: Arc<std::sync::RwLock<Ipv4Addr>>,
    /// 当前局域网候选；网络环境变化时由 refresh_network 更新，托盘菜单据此重建。
    pub candidates: Arc<std::sync::RwLock<Vec<Ipv4Addr>>>,
    /// 用户手动选过的 IP；None = 自动选最优。refresh_network 优先保留它。
    pub manual_ip: Arc<std::sync::RwLock<Option<Ipv4Addr>>>,
    pub mode: Arc<std::sync::RwLock<DeliveryMode>>,
    /// 粘贴成功后是否追加模拟回车；托盘菜单开关，默认关闭。
    pub auto_enter: Arc<std::sync::RwLock<bool>>,
    pub auto_paste_available: Arc<std::sync::atomic::AtomicBool>,
    pub connected: Arc<AtomicUsize>,
    /// 令牌轮换 / 退出时向所有 WS 连接广播。
    pub kick: broadcast::Sender<()>,
    pub token_generation: Arc<watch::Sender<u64>>,
    /// WS 收到的待注入文本；主线程串行消费。
    pub inject_tx: mpsc::UnboundedSender<Injection>,
    pub inject_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Injection>>>,
    /// 托盘命令 -> tokio 线程。
    pub ui_tx: mpsc::UnboundedSender<UiCommand>,
    pub ui_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<UiCommand>>>,
    /// 服务端通知主线程「队列里有待注入文本」。
    pub wake: tao::event_loop::EventLoopProxy<UserEvent>,
}

/// 一条待注入的文本，携带来源连接与消息 id，便于回 ack。
#[derive(Debug)]
pub struct Injection {
    pub id: String,
    pub text: String,
    pub reply: mpsc::UnboundedSender<HostToClient>,
}

#[derive(Debug)]
pub enum UiCommand {
    RotateToken,
    Shutdown,
}

#[derive(Debug)]
pub enum UserEvent {
    /// 待注入队列里有新任务，主线程取出来执行。
    InjectionReady,
    Connected(usize),
    ShowQr,
    Status(String),
    /// 看门狗探测事件：主 runloop 收到即刷新心跳，不产生任何 UI 副作用。
    Heartbeat,
    /// 局域网候选或选中 IP 变化：主线程重建托盘「网络」子菜单。
    NetworkChanged(NetworkChange),
}

/// 手机 -> 桌面 的文本消息（JSON 由 ws.rs 定义）。
pub type _ClientMessage = ClientToHost;

pub fn new_token() -> String {
    use base64::Engine;
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// 预绑定 listener：先拿到真实端口再生成二维码，端口被占用时交给 OS 分配。
pub fn bind(preferred: u16) -> Result<TcpListener> {
    let addrs = [
        SocketAddr::from(([0, 0, 0, 0], preferred)),
        SocketAddr::from(([0, 0, 0, 0], 0)),
    ];
    for addr in addrs {
        match TcpListener::bind(addr) {
            Ok(l) => {
                l.set_nonblocking(true)
                    .context("set listener nonblocking")?;
                return Ok(l);
            }
            Err(e) if addr.port() != 0 => {
                tracing::warn!("端口 {preferred} 不可用（{e}），改用随机端口");
            }
            Err(e) => return Err(e).context("绑定监听端口"),
        }
    }
    unreachable!()
}

/// 按可信度打分选择局域网 IPv4；跳过回环、链路本地、虚拟/VPN 网卡。
pub fn lan_candidates() -> Vec<Ipv4Addr> {
    let mut scored: Vec<(u8, Ipv4Addr)> = Vec::new();
    let Ok(ifaces) = if_addrs::get_if_addrs() else {
        return vec![];
    };
    for iface in ifaces {
        let IpAddr::V4(ip) = iface.ip() else { continue };
        let name = iface.name.to_ascii_lowercase();
        if ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() {
            continue;
        }
        if [
            "docker",
            "veth",
            "br-",
            "virbr",
            "tun",
            "tap",
            "utun",
            "vmnet",
            "vboxnet",
            "tailscale",
            "wg",
            "zt",
            "ham",
            "bridge",
            "lo",
        ]
        .iter()
        .any(|p| name.starts_with(p))
        {
            continue;
        }
        let o = ip.octets();
        let score = if o[0] == 192 && o[1] == 168 {
            100
        } else if o[0] == 10 {
            80
        } else if o[0] == 172 && (16..=31).contains(&o[1]) {
            70
        } else {
            10
        };
        if !scored.iter().any(|(_, existing)| *existing == ip) {
            scored.push((score, ip));
        }
    }
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    scored.into_iter().map(|(_, ip)| ip).collect()
}

impl Shared {
    pub fn token(&self) -> String {
        self.token.read().unwrap().clone()
    }
    pub fn lang(&self) -> Lang {
        *self.lang.read().unwrap()
    }
    pub fn lan_ip(&self) -> Ipv4Addr {
        *self.lan_ip.read().unwrap()
    }
    pub fn candidates(&self) -> Vec<Ipv4Addr> {
        self.candidates.read().unwrap().clone()
    }
    /// 用户手动选过的 IP（None 表示始终自动选最优）。
    pub fn manual_ip(&self) -> Option<Ipv4Addr> {
        *self.manual_ip.read().unwrap()
    }
    pub fn set_manual_ip(&self, ip: Option<Ipv4Addr>) {
        *self.manual_ip.write().unwrap() = ip;
    }
    pub fn mode(&self) -> DeliveryMode {
        *self.mode.read().unwrap()
    }
    pub fn auto_enter(&self) -> bool {
        *self.auto_enter.read().unwrap()
    }
    pub fn bump_connected(&self, delta: isize) -> usize {
        if delta > 0 {
            self.connected.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            self.connected
                .fetch_sub(1, Ordering::SeqCst)
                .saturating_sub(1)
        }
    }
    pub fn pair_url(&self) -> String {
        format!("http://127.0.0.1:{}/pair?t={}", self.port, self.token())
    }
    pub fn mobile_url(&self) -> String {
        format!(
            "http://{}:{}/m?t={}",
            self.lan_ip(),
            self.port,
            self.token()
        )
    }
    /// 重新枚举局域网地址并按需切换。返回变化情况供托盘/日志提示。
    /// - candidates 有变（网卡增减、IP 换了）→ 列表更新，`menu_changed` 要求托盘重建菜单；
    /// - 当前选中 IP 不在新列表里（手动选的也失效）→ 自动切到评分最高的候选；
    /// - 手动选的 IP 仍在线则保留不动。
    pub fn refresh_network(&self) -> NetworkChange {
        let fresh = lan_candidates();
        let mut candidates = self.candidates.write().unwrap();
        let old_selected = *self.lan_ip.read().unwrap();
        let list_changed = *candidates != fresh;
        if list_changed {
            *candidates = fresh.clone();
        }
        drop(candidates);

        let selected = if fresh.contains(&old_selected) {
            old_selected
        } else if let Some(manual) = self.manual_ip().filter(|ip| fresh.contains(ip)) {
            manual
        } else {
            fresh
                .first()
                .copied()
                .unwrap_or(Ipv4Addr::new(127, 0, 0, 1))
        };
        let ip_changed = selected != old_selected;
        if ip_changed {
            *self.lan_ip.write().unwrap() = selected;
        }
        let change = NetworkChange {
            list_changed,
            ip_changed,
            new_ip: selected,
        };
        // 列表或选中 IP 有变时唤醒主线程：重建托盘网络菜单、刷新状态提示。
        if change.list_changed || change.ip_changed {
            let _ = self
                .wake
                .send_event(crate::app::UserEvent::NetworkChanged(change));
        }
        change
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkChange {
    /// 候选列表有变，托盘「网络」子菜单需要重建。
    pub list_changed: bool,
    /// 选中的 IP 换了，QR 链接随之改变。
    pub ip_changed: bool,
    pub new_ip: Ipv4Addr,
}
