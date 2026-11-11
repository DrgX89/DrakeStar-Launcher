use crate::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerStatusPing {
    pub latency_ms: u128,
    pub version_name: Option<String>,
    pub protocol: Option<i32>,
    pub players_online: Option<i64>,
    pub players_max: Option<i64>,
    pub description: Option<String>,
    pub favicon: Option<String>,
    pub raw_json: Option<String>,
}

fn resolve_target(addr: &str) -> AppResult<(String, u16, SocketAddr)> {
    let addr = addr.trim();
    if addr.is_empty() || addr.contains(char::is_whitespace) || addr.contains("://") {
        return Err(AppError::Invalid("direccion invalida (usa host o host:puerto)".into()));
    }
    let (host, port, full) = if let Some((h, p)) = addr.rsplit_once(':') {
        if let Ok(port_num) = p.parse::<u16>() {
            (h.to_string(), port_num, addr.to_string())
        } else {
            (addr.to_string(), 25565, format!("{addr}:25565"))
        }
    } else {
        (addr.to_string(), 25565, format!("{addr}:25565"))
    };
    let sock = full
        .to_socket_addrs()
        .map_err(|e| AppError::Net(format!("no se resolvio '{full}': {e}")))?
        .next()
        .ok_or_else(|| AppError::Net(format!("sin direcciones para '{full}'")))?;
    Ok((host, port, sock))
}

/// Ping TCP (conexion al puerto), puerto por defecto 25565.
pub fn tcp_ping(addr: &str, timeout: Duration) -> AppResult<u128> {
    let (_host, _port, sock) = resolve_target(addr)?;
    let t = Instant::now();
    TcpStream::connect_timeout(&sock, timeout).map_err(|e| AppError::Net(format!("sin respuesta de {addr}: {e}")))?;
    Ok(t.elapsed().as_millis())
}

fn push_varint(buf: &mut Vec<u8>, mut v: i32) {
    loop {
        if (v & !0x7F) == 0 {
            buf.push(v as u8);
            return;
        }
        buf.push(((v & 0x7F) | 0x80) as u8);
        v = ((v as u32) >> 7) as i32;
    }
}

fn read_varint<R: Read>(r: &mut R) -> AppResult<i32> {
    let mut val = 0i32;
    for i in 0..5 {
        let mut b = [0u8; 1];
        r.read_exact(&mut b).map_err(|e| AppError::Net(format!("lectura varint SLP: {e}")))?;
        val |= ((b[0] & 0x7F) as i32) << (7 * i);
        if (b[0] & 0x80) == 0 {
            return Ok(val);
        }
    }
    Err(AppError::Net("varint SLP demasiado largo".into()))
}

/// Server List Ping (SLP 0x00) del protocolo de estado de Minecraft.
/// Si el servidor acepta TCP pero no habla SLP, devuelve `latency_ms` con campos opcionales en `None`.
pub fn mc_status_ping(addr: &str, timeout: Duration) -> AppResult<ServerStatusPing> {
    let (host, port, sock) = resolve_target(addr)?;
    let t = Instant::now();
    let mut stream = TcpStream::connect_timeout(&sock, timeout)
        .map_err(|e| AppError::Net(format!("sin respuesta de {addr}: {e}")))?;
    let latency_ms = t.elapsed().as_millis();
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    // Paquete Handshake (0x00): protocol_version (-1 / 769), host, port, next_state = 1 (status)
    let mut hs_body = Vec::new();
    push_varint(&mut hs_body, 0x00);
    push_varint(&mut hs_body, 769);
    let host_bytes = host.as_bytes();
    push_varint(&mut hs_body, host_bytes.len() as i32);
    hs_body.extend_from_slice(host_bytes);
    hs_body.extend_from_slice(&port.to_be_bytes());
    push_varint(&mut hs_body, 1);

    let mut pkt = Vec::new();
    push_varint(&mut pkt, hs_body.len() as i32);
    pkt.extend_from_slice(&hs_body);
    // Status Request (len=1, id=0x00)
    pkt.push(0x01);
    pkt.push(0x00);

    if stream.write_all(&pkt).is_err() || stream.flush().is_err() {
        return Ok(ServerStatusPing {
            latency_ms,
            version_name: None,
            protocol: None,
            players_online: None,
            players_max: None,
            description: None,
            favicon: None,
            raw_json: None,
        });
    }

    let Ok(_pkt_len) = read_varint(&mut stream) else {
        return Ok(ServerStatusPing {
            latency_ms,
            version_name: None,
            protocol: None,
            players_online: None,
            players_max: None,
            description: None,
            favicon: None,
            raw_json: None,
        });
    };
    let Ok(pkt_id) = read_varint(&mut stream) else {
        return Ok(ServerStatusPing {
            latency_ms,
            version_name: None,
            protocol: None,
            players_online: None,
            players_max: None,
            description: None,
            favicon: None,
            raw_json: None,
        });
    };
    if pkt_id != 0x00 {
        return Err(AppError::Net(format!("id de paquete SLP inesperado: {pkt_id}")));
    }
    let str_len = read_varint(&mut stream)?;
    if !(0..=262_144).contains(&str_len) {
        return Err(AppError::Net(format!("longitud JSON SLP invalida: {str_len}")));
    }
    let mut json_buf = vec![0u8; str_len as usize];
    stream
        .read_exact(&mut json_buf)
        .map_err(|e| AppError::Net(format!("lectura JSON SLP: {e}")))?;
    let raw_json_str = String::from_utf8_lossy(&json_buf).to_string();
    let v: serde_json::Value = serde_json::from_slice(&json_buf)
        .map_err(|e| AppError::Net(format!("JSON SLP invalido: {e}")))?;

    let version_name = v.pointer("/version/name").and_then(|x| x.as_str()).map(String::from);
    let protocol = v.pointer("/version/protocol").and_then(|x| x.as_i64()).map(|x| x as i32);
    let players_online = v.pointer("/players/online").and_then(|x| x.as_i64());
    let players_max = v.pointer("/players/max").and_then(|x| x.as_i64());
    let favicon = v.get("favicon").and_then(|x| x.as_str()).map(String::from);
    let description = match v.get("description") {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(obj) => {
            if let Some(t) = obj.get("text").and_then(|x| x.as_str()) {
                if !t.is_empty() {
                    Some(t.to_string())
                } else if let Some(extra) = obj.get("extra").and_then(|x| x.as_array()) {
                    let mut full = String::new();
                    for it in extra {
                        if let Some(txt) = it.get("text").and_then(|x| x.as_str()) {
                            full.push_str(txt);
                        }
                    }
                    Some(full)
                } else {
                    Some(t.to_string())
                }
            } else {
                None
            }
        }
        None => None,
    };

    Ok(ServerStatusPing {
        latency_ms,
        version_name,
        protocol,
        players_online,
        players_max,
        description,
        favicon,
        raw_json: Some(raw_json_str),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pings_local_listener_and_rejects_garbage() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        assert!(tcp_ping(&format!("127.0.0.1:{port}"), Duration::from_secs(2)).is_ok());
        drop(l);
        // El puerto efimero recien soltado puede reasignarse en los runners de
        // CI (y el test falla de forma intermitente): el puerto 1 nadie lo
        // escucha sin root, asi que el error es determinista.
        assert!(tcp_ping("127.0.0.1:1", Duration::from_millis(500)).is_err());
        assert!(tcp_ping("http://x", Duration::from_millis(100)).is_err());
        assert!(tcp_ping("", Duration::from_millis(100)).is_err());
    }

    #[test]
    fn mc_status_ping_parses_slp_response() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                // Lee handshake + status request
                let _hs_len = read_varint(&mut s).unwrap();
                let _hs_id = read_varint(&mut s).unwrap();
                let _proto = read_varint(&mut s).unwrap();
                let hlen = read_varint(&mut s).unwrap() as usize;
                let mut hbuf = vec![0u8; hlen + 2];
                s.read_exact(&mut hbuf).unwrap();
                let _next = read_varint(&mut s).unwrap();
                let _req_len = read_varint(&mut s).unwrap();
                let _req_id = read_varint(&mut s).unwrap();

                let payload = r#"{"version":{"name":"DrakesPurpur 26.2","protocol":770},"players":{"max":100,"online":12},"description":{"text":"Project Purple"}}"#;
                let mut body = Vec::new();
                push_varint(&mut body, 0x00);
                push_varint(&mut body, payload.len() as i32);
                body.extend_from_slice(payload.as_bytes());
                let mut pkt = Vec::new();
                push_varint(&mut pkt, body.len() as i32);
                pkt.extend_from_slice(&body);
                s.write_all(&pkt).unwrap();
            }
        });
        let st = mc_status_ping(&format!("127.0.0.1:{port}"), Duration::from_secs(2)).unwrap();
        assert_eq!(st.version_name.as_deref(), Some("DrakesPurpur 26.2"));
        assert_eq!(st.protocol, Some(770));
        assert_eq!(st.players_online, Some(12));
        assert_eq!(st.players_max, Some(100));
        assert_eq!(st.description.as_deref(), Some("Project Purple"));
    }
}
