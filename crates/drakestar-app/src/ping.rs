use crate::{AppError, AppResult};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// Ping TCP (conexion al puerto), NO el protocolo de estado de Minecraft: solo dice
/// si el puerto acepta conexiones y cuanto tarda. Puerto por defecto 25565.
pub fn tcp_ping(addr: &str, timeout: Duration) -> AppResult<u128> {
    let addr = addr.trim();
    if addr.is_empty() || addr.contains(char::is_whitespace) || addr.contains("://") {
        return Err(AppError::Invalid("direccion invalida (usa host o host:puerto)".into()));
    }
    let full = if addr.rsplit_once(':').map(|(_, p)| p.parse::<u16>().is_ok()).unwrap_or(false) {
        addr.to_string()
    } else {
        format!("{addr}:25565")
    };
    let sock = full
        .to_socket_addrs()
        .map_err(|e| AppError::Net(format!("no se resolvio '{full}': {e}")))?
        .next()
        .ok_or_else(|| AppError::Net(format!("sin direcciones para '{full}'")))?;
    let t = Instant::now();
    TcpStream::connect_timeout(&sock, timeout).map_err(|e| AppError::Net(format!("sin respuesta de {full}: {e}")))?;
    Ok(t.elapsed().as_millis())
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
        assert!(tcp_ping(&format!("127.0.0.1:{port}"), Duration::from_millis(500)).is_err());
        assert!(tcp_ping("http://x", Duration::from_millis(100)).is_err());
        assert!(tcp_ping("", Duration::from_millis(100)).is_err());
    }
}
