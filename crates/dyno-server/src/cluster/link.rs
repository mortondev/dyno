//! The cluster link: authenticated TCP between instances. Every connection
//! starts with a mutual challenge (HMAC-SHA256 over both nonces with the
//! shared secret), so only instances that know the secret can talk. Frames
//! are a u32 length then postcard bytes. Each peer address gets an outbound
//! worker with a queue that reconnects by itself; inbound connections are
//! read by their own threads and handed to one handler.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_FRAME: usize = 256 << 20;
const QUEUE: usize = 4096;

pub fn hmac(secret: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 64];
    if secret.len() > 64 {
        key[..32].copy_from_slice(&Sha256::digest(secret));
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let inner = Sha256::new()
        .chain_update(ipad)
        .chain_update(msg)
        .finalize();
    let outer = Sha256::new()
        .chain_update(opad)
        .chain_update(inner)
        .finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&outer);
    out
}

/// A nonce that never repeats in this process (time, counter, address).
fn nonce(salt: &str) -> [u8; 32] {
    static N: AtomicU64 = AtomicU64::new(0);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let c = N.fetch_add(1, Ordering::SeqCst);
    let mut h = Sha256::new();
    h.update(t.to_le_bytes());
    h.update(c.to_le_bytes());
    h.update(std::process::id().to_le_bytes());
    h.update(salt.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&h.finalize());
    out
}

fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn write_frame(s: &mut TcpStream, bytes: &[u8]) -> std::io::Result<()> {
    s.write_all(&(bytes.len() as u32).to_be_bytes())?;
    s.write_all(bytes)
}

pub fn read_frame(s: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len = [0u8; 4];
    s.read_exact(&mut len)?;
    let n = u32::from_be_bytes(len) as usize;
    if n > MAX_FRAME {
        return Err(std::io::Error::other("frame too large"));
    }
    let mut buf = vec![0u8; n];
    s.read_exact(&mut buf)?;
    Ok(buf)
}

#[derive(Serialize, Deserialize)]
struct Hello {
    nonce: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct HelloBack {
    nonce: [u8; 32],
    proof: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct Proof {
    proof: [u8; 32],
}

fn proof(secret: &[u8], side: &str, a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut m = Vec::with_capacity(80);
    m.extend_from_slice(b"dyno-cluster/1/");
    m.extend_from_slice(side.as_bytes());
    m.extend_from_slice(a);
    m.extend_from_slice(b);
    hmac(secret, &m)
}

fn enc<T: Serialize>(v: &T) -> Vec<u8> {
    postcard::to_stdvec(v).expect("link messages encode")
}

fn dec<T: DeserializeOwned>(b: &[u8]) -> std::io::Result<T> {
    postcard::from_bytes(b).map_err(|e| std::io::Error::other(format!("decode: {e}")))
}

/// Client side of the handshake.
fn auth_out(s: &mut TcpStream, secret: &[u8]) -> std::io::Result<()> {
    let a = nonce("out");
    write_frame(s, &enc(&Hello { nonce: a }))?;
    let back: HelloBack = dec(&read_frame(s)?)?;
    if !same(&back.proof, &proof(secret, "server", &a, &back.nonce)) {
        return Err(std::io::Error::other("peer failed the cluster challenge"));
    }
    write_frame(
        s,
        &enc(&Proof {
            proof: proof(secret, "client", &a, &back.nonce),
        }),
    )
}

/// Server side of the handshake.
fn auth_in(s: &mut TcpStream, secret: &[u8]) -> std::io::Result<()> {
    let hello: Hello = dec(&read_frame(s)?)?;
    let b = nonce("in");
    write_frame(
        s,
        &enc(&HelloBack {
            nonce: b,
            proof: proof(secret, "server", &hello.nonce, &b),
        }),
    )?;
    let p: Proof = dec(&read_frame(s)?)?;
    if !same(&p.proof, &proof(secret, "client", &hello.nonce, &b)) {
        return Err(std::io::Error::other("peer failed the cluster challenge"));
    }
    Ok(())
}

/// One message on the link, with who sent it and where to answer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Envelope<M> {
    pub from: u64,
    pub reply_to: String,
    pub msg: M,
}

type Handler<M> = Arc<dyn Fn(Envelope<M>) + Send + Sync>;

pub struct Link<M> {
    secret: Arc<Vec<u8>>,
    pub addr: String,
    pub id: AtomicU64,
    peers: Mutex<HashMap<String, SyncSender<Vec<u8>>>>,
    /// test switch: drop everything in and out
    pub cut: Arc<AtomicBool>,
    /// test switch: drop traffic with these addresses
    pub blocked: Arc<Mutex<HashSet<String>>>,
    stop: Arc<AtomicBool>,
    _m: std::marker::PhantomData<fn() -> M>,
}

impl<M: Serialize + DeserializeOwned + Send + 'static> Link<M> {
    /// Listen on `bind`; `addr` is what peers dial to reach us.
    pub fn start(
        bind: SocketAddr,
        addr: String,
        secret: Vec<u8>,
        handler: Handler<M>,
    ) -> std::io::Result<Arc<Link<M>>> {
        let listener = TcpListener::bind(bind)?;
        let link = Arc::new(Link {
            secret: Arc::new(secret),
            addr,
            id: AtomicU64::new(0),
            peers: Mutex::new(HashMap::new()),
            cut: Arc::new(AtomicBool::new(false)),
            blocked: Arc::new(Mutex::new(HashSet::new())),
            stop: Arc::new(AtomicBool::new(false)),
            _m: std::marker::PhantomData,
        });
        let l = link.clone();
        std::thread::Builder::new()
            .name("cluster-accept".into())
            .spawn(move || {
                for s in listener.incoming() {
                    if l.stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(mut s) = s else { continue };
                    let (secret, h, cut, blocked, stop) = (
                        l.secret.clone(),
                        handler.clone(),
                        l.cut.clone(),
                        l.blocked.clone(),
                        l.stop.clone(),
                    );
                    let _ =
                        std::thread::Builder::new()
                            .name("cluster-in".into())
                            .spawn(move || {
                                s.set_nodelay(true).ok();
                                s.set_read_timeout(Some(Duration::from_secs(10))).ok();
                                if auth_in(&mut s, &secret).is_err() {
                                    return;
                                }
                                s.set_read_timeout(None).ok();
                                while !stop.load(Ordering::SeqCst) {
                                    let Ok(f) = read_frame(&mut s) else { return };
                                    if cut.load(Ordering::SeqCst) {
                                        continue;
                                    }
                                    let Ok(env) = dec::<Envelope<M>>(&f) else {
                                        return;
                                    };
                                    if blocked.lock().unwrap().contains(&env.reply_to) {
                                        continue;
                                    }
                                    h(env);
                                }
                            });
                }
            })?;
        Ok(link)
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(a) = self.addr.parse::<SocketAddr>() {
            let _ = TcpStream::connect_timeout(&a, Duration::from_millis(100));
        }
        self.peers.lock().unwrap().clear();
    }

    /// Send to the instance at `addr`. Never blocks; drops if the queue is full
    /// (Raft retries, requests time out).
    pub fn send(&self, addr: &str, msg: M) {
        if self.cut.load(Ordering::SeqCst)
            || self.stop.load(Ordering::SeqCst)
            || self.blocked.lock().unwrap().contains(addr)
        {
            return;
        }
        let env = Envelope {
            from: self.id.load(Ordering::SeqCst),
            reply_to: self.addr.clone(),
            msg,
        };
        let bytes = enc(&env);
        let mut peers = self.peers.lock().unwrap();
        let tx = peers
            .entry(addr.to_string())
            .or_insert_with(|| self.worker(addr.to_string()));
        match tx.try_send(bytes) {
            Ok(()) | Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(b)) => {
                let tx = self.worker(addr.to_string());
                let _ = tx.try_send(b);
                peers.insert(addr.to_string(), tx);
            }
        }
    }

    fn worker(&self, addr: String) -> SyncSender<Vec<u8>> {
        let (tx, rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = sync_channel(QUEUE);
        let (secret, stop) = (self.secret.clone(), self.stop.clone());
        let _ = std::thread::Builder::new()
            .name("cluster-out".into())
            .spawn(move || {
                let mut conn: Option<TcpStream> = None;
                while let Ok(bytes) = rx.recv() {
                    if stop.load(Ordering::SeqCst) {
                        return;
                    }
                    for _ in 0..2 {
                        if conn.is_none() {
                            conn = addr.parse::<SocketAddr>().ok().and_then(|a| {
                                let mut s =
                                    TcpStream::connect_timeout(&a, Duration::from_millis(500))
                                        .ok()?;
                                s.set_nodelay(true).ok();
                                s.set_read_timeout(Some(Duration::from_secs(5))).ok();
                                s.set_write_timeout(Some(Duration::from_secs(5))).ok();
                                auth_out(&mut s, &secret).ok()?;
                                Some(s)
                            });
                        }
                        let Some(s) = conn.as_mut() else { break };
                        if write_frame(s, &bytes).is_ok() {
                            break;
                        }
                        conn = None;
                    }
                }
            });
        tx
    }
}

/// A channel for answers to requests: send a request with an id, wait here.
pub struct Waiters<T> {
    next: AtomicU64,
    map: Mutex<HashMap<u64, Sender<T>>>,
}

impl<T> Default for Waiters<T> {
    fn default() -> Self {
        Waiters {
            next: AtomicU64::new(1),
            map: Mutex::new(HashMap::new()),
        }
    }
}

impl<T> Waiters<T> {
    pub fn open(&self) -> (u64, Receiver<T>) {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = std::sync::mpsc::channel();
        self.map.lock().unwrap().insert(id, tx);
        (id, rx)
    }

    /// Wait on an id the caller chose (unique among open waits).
    pub fn open_at(&self, id: u64) -> Receiver<T> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.map.lock().unwrap().insert(id, tx);
        rx
    }

    pub fn answer(&self, id: u64, v: T) {
        if let Some(tx) = self.map.lock().unwrap().remove(&id) {
            let _ = tx.send(v);
        }
    }

    pub fn forget(&self, id: u64) {
        self.map.lock().unwrap().remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_rfc4231_case_2() {
        let mac = hmac(b"Jefe", b"what do ya want for nothing?");
        let hex: String = mac.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn only_peers_with_the_secret_get_through() {
        let got = Arc::new(Mutex::new(Vec::<String>::new()));
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let addr = format!("127.0.0.1:{port}");
        let g = got.clone();
        let _server: Arc<Link<String>> = Link::start(
            addr.parse().unwrap(),
            addr.clone(),
            b"right".to_vec(),
            Arc::new(move |e: Envelope<String>| g.lock().unwrap().push(e.msg)),
        )
        .unwrap();
        let good: Arc<Link<String>> = Link::start(
            "127.0.0.1:0".parse().unwrap(),
            "127.0.0.1:1".into(),
            b"right".to_vec(),
            Arc::new(|_| {}),
        )
        .unwrap();
        let bad: Arc<Link<String>> = Link::start(
            "127.0.0.1:0".parse().unwrap(),
            "127.0.0.1:2".into(),
            b"wrong".to_vec(),
            Arc::new(|_| {}),
        )
        .unwrap();
        good.send(&addr, "hello".into());
        bad.send(&addr, "intruder".into());
        std::thread::sleep(Duration::from_millis(500));
        assert_eq!(*got.lock().unwrap(), vec!["hello".to_string()]);
    }
}
