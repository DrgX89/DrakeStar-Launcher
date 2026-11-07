//! Conexion del cliente: handshake, login offline y lectura del flujo posterior.
//!
//! Alcance real (y sus limites, para no prometer de mas):
//! - handshake + login start + login acknowledged: si, contra un servidor.
//! - servidores `online-mode=false` (como el cliente Java gestionado del launcher).
//! - NO hace cifrado (online-mode), NO hace compresion, NO decodifica los estados
//!   de configuracion/play (inventarios, GUIs de Slimefun): ahi solo registra el
//!   paquete. Esos tres son el trabajo pendiente para v1.0.
//!
//! shortcut: modo offline y sin compresion; actualizar cuando se pruebe contra un
//! DrakesPurpur 26.2 real (cifrado + zlib + estado de configuracion).

use crate::proto;
use serde_json::Value;
use std::io::{BufRead, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Protocolo de la linea 26.x del ecosistema (docs/ARCHITECTURE.md):
/// DrakesPurpur 26.2 y Horizon 26.3/protocolo 777.
pub const PROTOCOL_26_2: i32 = 776;
pub const DEFAULT_PORT: u16 = 25565;

pub const NEXT_STATE_LOGIN: i32 = 2;
pub const LOGIN_START: i32 = 0x00;
pub const LOGIN_ACKNOWLEDGED: i32 = 0x02;
pub const PKT_DISCONNECT: i32 = 0x00;
pub const PKT_ENCRYPTION_REQUEST: i32 = 0x01;
pub const PKT_LOGIN_SUCCESS: i32 = 0x02;
pub const PKT_SET_COMPRESSION: i32 = 0x03;

#[derive(Debug, Clone)]
pub struct Config {
    pub server: String,
    pub port: u16,
    pub name: String,
    pub protocol: i32,
    /// Conexion y espera de respuesta. Si el servidor no contesta al login dentro
    /// de este tiempo, se aborta en vez de quedarse colgado.
    pub timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self { server: String::new(), port: DEFAULT_PORT, name: "jugador".into(), protocol: PROTOCOL_26_2, timeout: Duration::from_secs(10) }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// El servidor pide cifrado: es `online-mode=true`.
    OnlineMode,
    Compression,
    /// El servidor no contesto al login dentro del timeout.
    NoLogin,
    Disconnected(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "error de red: {e}"),
            Self::OnlineMode => write!(
                f,
                "el servidor exige cifrado (online-mode): este cliente solo conecta en modo offline (online-mode=false)"
            ),
            Self::Compression => write!(f, "el servidor activa compresion: no soportada todavia"),
            Self::NoLogin => write!(f, "el servidor no respondio al login (timeout de {}s)", 10),
            Self::Disconnected(r) => write!(f, "el servidor nos desconecto: {r}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Texto corto de un componente de chat de Minecraft. El cuerpo del paquete es
/// `[VarInt longitud][JSON]`, no el JSON suelto.
fn chat_text(body: &[u8]) -> String {
    let raw = proto::read_string(&mut std::io::Cursor::new(body)).unwrap_or_else(|_| String::from_utf8_lossy(body).to_string());
    let Ok(v) = serde_json::from_str::<Value>(&raw) else { return raw };
    match &v {
        Value::Object(o) => {
            if let Some(Value::String(t)) = o.get("text") {
                return t.clone();
            }
            if let Some(Value::String(t)) = o.get("translate") {
                return format!("[{t}]");
            }
        }
        Value::String(t) => return t.clone(),
        _ => {}
    }
    raw.chars().take(200).collect()
}

/// Conecta, hace el login y lee el flujo hasta que el servidor cierra o el
/// usuario escribe `quit`/`stop`. Cada linea se entrega por `out` (el binario la
/// imprime; los tests la capturan).
pub fn run(cfg: &Config, out: &mut dyn FnMut(&str)) -> Result<(), Error> {
    let addr = (cfg.server.as_str(), cfg.port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "direccion no valida")))?;
    out(&format!("conectando a {addr} (protocolo {})", cfg.protocol));
    let mut sock = TcpStream::connect_timeout(&addr, cfg.timeout)?;
    sock.set_read_timeout(Some(cfg.timeout))?;

    // handshake + login start: el protocolo permite mandarlos juntos
    let mut hello = Vec::new();
    let mut hs = Vec::new();
    proto::write_varint(&mut hs, cfg.protocol);
    proto::write_string(&mut hs, &cfg.server);
    proto::write_u16(&mut hs, cfg.port);
    proto::write_varint(&mut hs, NEXT_STATE_LOGIN);
    proto::write_packet(&mut hello, 0x00, &hs);
    let mut start = Vec::new();
    proto::write_string(&mut start, &cfg.name);
    start.extend_from_slice(&[0u8; 16]); // UUID nil: solo valido en modo offline
    proto::write_packet(&mut hello, LOGIN_START, &start);
    sock.write_all(&hello)?;
    sock.flush()?;

    // fase de login
    loop {
        let (id, body) = match proto::read_packet(&mut sock) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                return Err(Error::NoLogin)
            }
            Err(e) => return Err(e.into()),
        };
        match id {
            PKT_ENCRYPTION_REQUEST => return Err(Error::OnlineMode),
            PKT_SET_COMPRESSION => return Err(Error::Compression),
            PKT_DISCONNECT => return Err(Error::Disconnected(chat_text(&body))),
            PKT_LOGIN_SUCCESS => {
                out(&format!("login correcto ({} bytes de Login Success)", body.len()));
                proto::send_packet(&mut sock, LOGIN_ACKNOWLEDGED, &[])?;
                break;
            }
            other => out(&format!("paquete de login ignorado: 0x{other:02x}")),
        }
    }

    // `quit`/`stop` desde el launcher o la terminal: avisa y corta el socket,
    // asi el bucle de lectura termina en limpio (codigo 0).
    let quit = sock.try_clone()?;
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            let cmd = line.trim();
            if cmd == "quit" || cmd == "stop" || cmd == "exit" {
                let mut body = Vec::new();
                proto::write_string(&mut body, r#"{"text":"adios"}"#);
                let _ = proto::send_packet(&mut &quit, PKT_DISCONNECT, &body);
                let _ = quit.shutdown(std::net::Shutdown::Both);
                return;
            }
        }
    });

    // flujo posterior (configuracion/play): se registra sin decodificar.
    loop {
        let (id, body) = match proto::read_packet(&mut sock) {
            Ok(p) => p,
            // el servidor cerro: es una salida limpia, no un fallo
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                continue; // servidor en silencio: no es un error
            }
            Err(e) => return Err(e.into()),
        };
        if id == PKT_DISCONNECT {
            return Err(Error::Disconnected(chat_text(&body)));
        }
        out(&format!("paquete 0x{id:02x} ({} bytes)", body.len()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::mpsc;

    fn cfg(port: u16) -> Config {
        Config { server: "127.0.0.1".into(), port, name: "tester".into(), ..Config::default() }
    }

    /// Servidor minimo offline: lee el saludo, contesta lo que diga `script`
    /// y devuelve lo observado por un canal.
    fn mini_server(script: impl FnOnce(&mut dyn Read, &mut dyn Write) + Send + 'static) -> u16 {
        let lst = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = lst.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((s, _)) = lst.accept() {
                let mut r = s.try_clone().unwrap();
                let mut w = s;
                script(&mut r, &mut w);
            }
        });
        port
    }

    fn collect(cfg: &Config, script: impl FnOnce(&mut dyn Read, &mut dyn Write) + Send + 'static) -> (Vec<String>, Result<(), Error>) {
        let port = mini_server(script);
        let (tx, rx) = mpsc::channel();
        let cfg = Config { port, ..cfg.clone() };
        let res = std::thread::spawn(move || run(&cfg, &mut |l| tx.send(l.to_string()).unwrap())).join().unwrap();
        (rx.try_iter().collect(), res)
    }

    /// Cliente que conecta y recibe flujo: el servidor contesta login correcto,
    /// dos paquetes y luego cierra -> el cliente termina con Ok.
    #[test]
    fn connects_and_logs_the_stream() {
        let (lines, res) = collect(
            &cfg(0),
            |r, w| {
                let (hs, _) = proto::read_packet(r).unwrap(); // handshake
                assert_eq!(hs, 0x00);
                let (start, body) = proto::read_packet(r).unwrap(); // login start
                assert_eq!(start, LOGIN_START);
                let name = proto::read_string(&mut std::io::Cursor::new(body)).unwrap();
                assert_eq!(name, "tester");
                let mut body = Vec::new();
                proto::write_string(&mut body, "login-ok-payload");
                proto::send_packet(w, PKT_LOGIN_SUCCESS, &body).unwrap();
                let (ack, ack_body) = proto::read_packet(r).unwrap(); // login acknowledged
                assert_eq!((ack, ack_body.len()), (LOGIN_ACKNOWLEDGED, 0));
                proto::send_packet(w, 0x57, b"a").unwrap();
                proto::send_packet(w, 0x58, b"bc").unwrap();
                w.flush().unwrap();
                // al cerrar, el cliente ve EOF y sale con Ok
            },
        );
        assert!(res.is_ok(), "{res:?}");
        assert!(lines.iter().any(|l| l.contains("login correcto")));
        assert!(lines.iter().any(|l| l.contains("0x57")));
        assert!(lines.iter().any(|l| l.contains("0x58") && l.contains("2 bytes")));
    }

    #[test]
    fn online_mode_and_compression_are_reported() {
        let (_, res) = collect(&cfg(0), |_r, w| {
            proto::send_packet(w, PKT_ENCRYPTION_REQUEST, b"x").unwrap();
            w.flush().unwrap();
        });
        assert!(matches!(res, Err(Error::OnlineMode)));

        let (_, res) = collect(&cfg(0), |_r, w| {
            let mut b = Vec::new();
            proto::write_varint(&mut b, 256);
            proto::send_packet(w, PKT_SET_COMPRESSION, &b).unwrap();
            w.flush().unwrap();
        });
        assert!(matches!(res, Err(Error::Compression)));
    }

    #[test]
    fn disconnect_reason_is_extracted_from_chat_json() {
        let (lines, res) = collect(&cfg(0), |_r, w| {
            let mut b = Vec::new();
            proto::write_string(&mut b, r#"{"text":"servidor lleno"}"#);
            proto::send_packet(w, PKT_DISCONNECT, &b).unwrap();
            w.flush().unwrap();
        });
        match res {
            Err(Error::Disconnected(r)) => assert_eq!(r, "servidor lleno"),
            other => panic!("esperaba desconexion: {other:?}"),
        }
        assert!(lines.iter().any(|l| l.starts_with("conectando a")));
    }

    #[test]
    fn chat_text_falls_back_to_raw() {
        assert_eq!(chat_text(br#"{"translate":"multiplayer.disconnect.kicked"}"#), "[multiplayer.disconnect.kicked]");
        assert_eq!(chat_text(b"\""), "\"");
        assert_eq!(chat_text(b"no json"), "no json");
    }
}
