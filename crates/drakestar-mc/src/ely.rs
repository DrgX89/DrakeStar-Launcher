//! Ely.by Yggdrasil: contraseña/2FA solo en memoria; renovación con token y javaagent oficial verificado.
use crate::launch::Session;
use crate::{Dirs, McError, McResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;

const AUTH: &str = "https://authserver.ely.by/auth";
const AGENT_VERSION: &str = "1.2.8";
const AGENT_SHA256: &str = "9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10";
const AGENT_URL: &str = "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.8/authlib-injector-1.2.8.jar";

#[derive(Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub client_token: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    access_token: String,
    client_token: String,
    selected_profile: Profile,
}
#[derive(Deserialize)]
struct Profile {
    id: String,
    name: String,
}

fn auth_error(status: u16, two_factor: bool) -> McError {
    // Nunca se propaga el cuerpo de respuesta: podría contener credenciales reflejadas.
    McError::Invalid(if two_factor { "Ely.by: introduce el código de verificación de dos pasos." }
        else if status == 400 || status == 401 || status == 403 { "Ely.by: acceso rechazado o sesión caducada. Revisa las credenciales y vuelve a iniciar sesión." }
        else if status == 429 { "Ely.by: demasiados intentos. Espera antes de reintentar." }
        else { "Ely.by no está disponible. Inténtalo más tarde." }.into())
}
fn request(action: &str, body: serde_json::Value) -> McResult<Response> {
    request_at(AUTH, action, body)
}
fn request_at(base: &str, action: &str, body: serde_json::Value) -> McResult<Response> {
    let response = ureq::post(&format!("{}/{action}", base.trim_end_matches('/')))
        .set("User-Agent", "SlimefunNewHorizons/DrakeStar-Launcher")
        .timeout(std::time::Duration::from_secs(30))
        .send_json(body);
    match response {
        Ok(r) => r
            .into_json()
            .map_err(|_| McError::Invalid("Ely.by devolvió una respuesta no válida.".into())),
        Err(ureq::Error::Status(status, r)) => {
            let body: serde_json::Value = r.into_json().unwrap_or_default();
            Err(auth_error(
                status,
                body["errorMessage"]
                    .as_str()
                    .is_some_and(|s| s.contains("two factor")),
            ))
        }
        Err(_) => Err(McError::Net(
            "No se pudo conectar con Ely.by. Revisa tu conexión.".into(),
        )),
    }
}
fn decode(r: Response, client_token: &str) -> McResult<(Session, Token)> {
    let uuid = r.selected_profile.id.replace('-', "");
    if uuid.len() != 32
        || !uuid.bytes().all(|b| b.is_ascii_hexdigit())
        || !crate::offline::valid_name(&r.selected_profile.name)
        || r.access_token.is_empty()
        || r.client_token != client_token
    {
        return Err(McError::Invalid("Perfil Ely.by no válido.".into()));
    }
    Ok((
        Session {
            name: r.selected_profile.name,
            uuid: uuid.to_lowercase(),
            access_token: r.access_token.clone(),
            user_type: "mojang".into(),
            xuid: None,
            client_id: None,
        },
        Token {
            access_token: r.access_token,
            client_token: r.client_token,
        },
    ))
}
pub fn authenticate(
    username: &str,
    password: &str,
    totp: &str,
    client_token: &str,
) -> McResult<(Session, Token)> {
    if username.trim().is_empty()
        || password.is_empty()
        || username.len() > 320
        || password.len() > 1024
        || (!totp.is_empty() && (totp.len() != 6 || !totp.bytes().all(|c| c.is_ascii_digit())))
    {
        return Err(McError::Invalid(
            "Introduce usuario, contraseña y, si corresponde, un código 2FA de seis dígitos."
                .into(),
        ));
    }
    let password = if totp.is_empty() {
        password.to_string()
    } else {
        format!("{password}:{totp}")
    };
    decode(
        request(
            "authenticate",
            json!({"username":username.trim(), "password":password, "clientToken":client_token, "requestUser":false}),
        )?,
        client_token,
    )
}
pub fn refresh(token: &Token) -> McResult<(Session, Token)> {
    decode(
        request(
            "refresh",
            json!({"accessToken":token.access_token, "clientToken":token.client_token, "requestUser":false}),
        )?,
        &token.client_token,
    )
}
fn verified(bytes: &[u8]) -> bool {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    digest == AGENT_SHA256
}
/// Descarga separada en runtime: DrakeStar no incluye ni modifica el binario del proyecto.
pub fn ensure_agent(d: &Dirs) -> McResult<PathBuf> {
    let path = d.libraries().join(format!(
        "authlib-injector/authlib-injector-{AGENT_VERSION}.jar"
    ));
    if std::fs::read(&path).is_ok_and(|bytes| verified(&bytes)) {
        return Ok(path);
    }
    let r = ureq::get(AGENT_URL)
        .set("User-Agent", "SlimefunNewHorizons/DrakeStar-Launcher")
        .timeout(std::time::Duration::from_secs(120))
        .call()
        .map_err(|_| McError::Net("No se pudo descargar authlib-injector para Ely.by.".into()))?;
    let mut bytes = vec![];
    r.into_reader()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if !verified(&bytes) {
        return Err(McError::Invalid(
            "SHA-256 de authlib-injector no coincide.".into(),
        ));
    }
    crate::write_atomic(&path, &bytes)?;
    Ok(path)
}
pub fn agent_arg(path: &std::path::Path) -> String {
    format!("-javaagent:{}=ely.by", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_identity_without_exposing_tokens() {
        let response = |id: &str| Response {
            access_token: "TEST-ACCESS".into(),
            client_token: "client".into(),
            selected_profile: Profile {
                id: id.into(),
                name: "Alex".into(),
            },
        };
        let (s, t) = decode(response("069a79f4-44e9-4726-a5be-fca90e38aaf5"), "client").unwrap();
        assert_eq!(s.uuid, "069a79f444e94726a5befca90e38aaf5");
        assert_eq!(s.user_type, "mojang");
        assert_eq!(t.access_token, "TEST-ACCESS");
        assert!(decode(response("bad"), "client").is_err());
        assert!(decode(response("069a79f444e94726a5befca90e38aaf5"), "other").is_err());
        assert!(auth_error(401, true).to_string().contains("dos pasos"));
        assert!(!auth_error(401, false).to_string().contains("TEST-ACCESS"));
        assert!(!verified(b"corrupt jar"));
        assert_eq!(
            agent_arg(std::path::Path::new("folder with spaces/agent.jar")),
            "-javaagent:folder with spaces/agent.jar=ely.by"
        );
    }

    #[test]
    fn loopback_yggdrasil_authenticate_and_refresh() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let base = format!("http://{addr}/auth");

        let server = std::thread::spawn(move || {
            for step in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..n]);
                let (status, body) = match step {
                    0 => {
                        assert!(req.starts_with("POST /auth/authenticate "));
                        assert!(req.contains("\"password\":\"secret:123456\""));
                        (
                            "200 OK",
                            r#"{"accessToken":"ELY-TOK-1","clientToken":"CLI-1","selectedProfile":{"id":"069a79f4-44e9-4726-a5be-fca90e38aaf5","name":"AlexEly"}}"#,
                        )
                    }
                    1 => {
                        assert!(req.starts_with("POST /auth/refresh "));
                        assert!(req.contains("\"accessToken\":\"ELY-TOK-1\""));
                        (
                            "200 OK",
                            r#"{"accessToken":"ELY-TOK-2","clientToken":"CLI-1","selectedProfile":{"id":"069a79f444e94726a5befca90e38aaf5","name":"AlexRenamed"}}"#,
                        )
                    }
                    _ => (
                        "401 Unauthorized",
                        r#"{"error":"ForbiddenOperationException","errorMessage":"Account protected with two factor auth.LENIENT-SECRET-DO-NOT-LEAK"}"#,
                    ),
                };
                let resp = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(resp.as_bytes()).unwrap();
            }
        });

        let (s1, t1) = decode(
            request_at(
                &base,
                "authenticate",
                json!({"username":"alex","password":"secret:123456","clientToken":"CLI-1","requestUser":false}),
            )
            .unwrap(),
            "CLI-1",
        )
        .unwrap();
        assert_eq!(s1.name, "AlexEly");
        assert_eq!(t1.access_token, "ELY-TOK-1");

        let (s2, t2) = decode(
            request_at(
                &base,
                "refresh",
                json!({"accessToken":t1.access_token,"clientToken":t1.client_token,"requestUser":false}),
            )
            .unwrap(),
            "CLI-1",
        )
        .unwrap();
        assert_eq!(s2.name, "AlexRenamed");
        assert_eq!(t2.access_token, "ELY-TOK-2");

        let err = request_at(&base, "authenticate", json!({"username":"alex"})).err().unwrap().to_string();
        assert!(err.contains("dos pasos"));
        assert!(!err.contains("LENIENT-SECRET-DO-NOT-LEAK"));

        server.join().unwrap();
    }
}
