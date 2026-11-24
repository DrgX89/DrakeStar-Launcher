//! Flujo completo del boton JUGAR: loader -> version -> Java -> mods -> cuenta -> plan de lanzamiento.

use crate::accounts::Accounts;
use crate::instances::{Instance, Loader, Target};
use crate::launch::{self, Options, Plan, Session};
use crate::progress::ProgressSink;
use crate::{fabric, install, java, mods, server, Dirs, McError, McResult};

/// Resultado de preparar una instancia: todo listo para `launch::spawn`.
pub struct Prepared {
    pub plan: Plan,
    pub session: Session,
    pub game_dir: std::path::PathBuf,
    /// Avisos para mostrar al usuario (mods omitidos, Fused no disponible para la version...).
    pub warnings: Vec<String>,
    pub java_major: u32,
    pub version_id: String,
    /// Servidor local que hay que arrancar y esperar antes del cliente (`Target::Local`).
    pub server: Option<LocalServer>,
}

pub struct LocalServer {
    pub plan: launch::Plan,
    pub port: u16,
    pub log_file: std::path::PathBuf,
}

/// Prepara la instancia sin lanzar. `client_id` es el de la app de Azure para renovar cuentas Microsoft.
pub fn prepare(d: &Dirs, inst: &Instance, account_id: &str, client_id: &str, sink: &dyn ProgressSink) -> McResult<Prepared> {
    inst.validate()?;
    let mut warnings = vec![];

    // 1) cuenta primero: si falla el login no vale la pena bajar 400 MB
    sink.stage("account", "Comprobando tu cuenta");
    // Mantener la identidad elegida aunque se retire la cuenta durante las descargas.
    let account = Accounts::new(d).get(account_id).ok_or_else(|| McError::NotFound(format!("cuenta {account_id}")))?;
    let session = match Accounts::new(d).session(account_id, client_id) {
        Ok(s) => s,
        // sin internet: se juega en solitario con el nombre y el UUID de la cuenta (los servidores no dejaran entrar)
        Err(e @ McError::Net(_)) => {
            if account.kind != crate::accounts::Kind::Microsoft { return Err(e); }
            warnings.push("Sin conexion con Microsoft: juegas en solitario con el nombre de tu cuenta; los servidores no te dejaran entrar.".into());
            Session { name: account.name.clone(), uuid: account.uuid.clone(), access_token: "0".into(), user_type: "legacy".into(), xuid: None, client_id: None }
        }
        Err(e) => return Err(e),
    };

    // 2) version (+ loader)
    let version_id = match inst.loader {
        Loader::Vanilla => inst.mc.clone(),
        Loader::Fabric => fabric::ensure_profile(d, &inst.mc, inst.loader_version.as_deref())?,
    };
    let installed = install::install(d, &version_id, sink)?;
    if sink.cancelled() {
        return Err(McError::Cancelled);
    }

    // 3) Java
    let java_bin = java::ensure(d, installed.java_major, sink)?;

    // 4) mods
    let game_dir = d.instance(&inst.id);
    std::fs::create_dir_all(&game_dir)?;
    // Primer arranque: sin la pantalla de narrador/accesibilidad de Minecraft (si ya hay options.txt no se toca).
    let options = game_dir.join("options.txt");
    if !options.exists() {
        let _ = std::fs::write(&options, "onboardAccessibility:false\n");
    }
    let mut fused_jar = None;
    if inst.loader == Loader::Fabric {
        sink.stage("mods", "Instalando mods");
        let mods_dir = game_dir.join("mods");
        let mut offline = false;
        match mods::ensure_fabric_api(&mods_dir, &inst.mc) {
            Ok(_) => {}
            // sin conexion: si Fabric API ya esta instalado se sigue con los mods que hay
            Err(McError::Net(_)) if has_jar(&mods_dir, "fabric-api-") => {
                offline = true;
                warnings.push("Sin conexion: se usan los mods ya instalados.".into());
            }
            Err(e) => return Err(e),
        }
        if inst.performance && !offline {
            let ok = mods::ensure_performance(&mods_dir, &inst.mc, &|l| sink.log(l));
            if ok.len() < mods::PERFORMANCE_SET.len() {
                warnings.push(format!("Solo {} de {} mods de rendimiento existen para Minecraft {}.", ok.len(), mods::PERFORMANCE_SET.len(), inst.mc));
            }
        }
        if !inst.fused {
            mods::remove_fused(&mods_dir);
        } else if !offline {
            match mods::ensure_fused(&mods_dir, &inst.mc, "fabric") {
                Ok(Some(j)) => fused_jar = Some(j),
                Ok(None) => warnings.push(format!("DrakeStar Fused aun no tiene una version publicada para Minecraft {}: se abre sin Fused.", inst.mc)),
                Err(e) => warnings.push(format!("No se pudo instalar DrakeStar Fused: {e}")),
            }
        }
    }

    // 5) plan
    sink.stage("launch", "Iniciando Minecraft");
    let mut o = Options { game_dir: game_dir.clone(), java_bin, xmx_mb: inst.xmx_mb, xms_mb: (inst.xmx_mb / 4).max(512), extra_jvm: inst.extra_jvm.clone(), ..Default::default() };
    match &inst.target {
        Target::Menu => {}
        Target::Server { address } => o.join_server = Some(address.clone()),
        Target::World { name } => o.open_world = Some(name.clone()),
        Target::Local { .. } => {}
    }
    // servidor local: Purpur + Slimefun, con su propio Java
    let mut local = None;
    if let Target::Local { port, lan } = &inst.target {
        let port = server::free_port(*port);
        // 26.x: el monolito DrakesPurpur (ya trae Slimefun y las suites); antes de 26: Purpur + Slimefun
        let monolith = server::is_monolith(&inst.mc);
        let jar = if monolith { server::ensure_drakespurpur(d, &inst.id, sink)? } else { server::ensure_purpur(d, &inst.id, &inst.mc, sink)? };
        server::configure(d, &inst.id, port, *lan)?;
        // paso 5b: Fused con host embebido + monolito 26.x + cliente Java 25 -> el servidor corre dentro del cliente
        if monolith && installed.java_major >= 25 && fused_jar.as_deref().is_some_and(mods::fused_embeds) {
            o.extra_jvm.push(format!("-Ddrakestar.fused.embedded.jar={}", jar.display()));
            o.extra_jvm.push(format!("-Ddrakestar.fused.embedded.server={}", server::dir(d, &inst.id).display()));
            o.xmx_mb += 3072;
            o.join_server = Some(format!("127.0.0.1:{port}"));
            let plan = launch::plan(d, &installed, &session, &with_ely(d, &account, o, sink)?)?;
            return Ok(Prepared { plan, session, game_dir, warnings, java_major: installed.java_major, version_id, server: None });
        }
        let sjava = java::ensure(d, server::java_major(&inst.mc), sink)?;
        match if monolith { Ok(Some(std::path::PathBuf::new())) } else { mods::ensure_slimefun(&server::dir(d, &inst.id).join("plugins"), &inst.mc) } {
            Ok(Some(_)) => {}
            Ok(None) => warnings.push(format!("Slimefun aun no tiene build para Minecraft {}: el servidor local abre sin Slimefun.", inst.mc)),
            Err(e) => warnings.push(format!("No se pudo instalar Slimefun: {e}")),
        }
        local = Some(LocalServer { plan: server::plan(d, &inst.id, &jar, sjava, (inst.xmx_mb / 2).max(if monolith { 3072 } else { 1536 })), port, log_file: server::dir(d, &inst.id).join("logs").join("launcher-server.log") });
        o.join_server = Some(format!("127.0.0.1:{port}"));
    }
    let plan = launch::plan(d, &installed, &session, &with_ely(d, &account, o, sink)?)?;
    Ok(Prepared { plan, session, game_dir, warnings, java_major: installed.java_major, version_id, server: local })
}

fn with_ely(d: &Dirs, account: &crate::accounts::Account, o: Options, sink: &dyn ProgressSink) -> McResult<Options> {
    with_ely_agent(account, o, sink, || crate::ely::ensure_agent(d))
}

fn with_ely_agent(
    account: &crate::accounts::Account,
    mut o: Options,
    sink: &dyn ProgressSink,
    ensure_agent: impl FnOnce() -> McResult<std::path::PathBuf>,
) -> McResult<Options> {
    if account.kind == crate::accounts::Kind::Elyby {
        sink.stage("elyby", "Preparando autenticación Ely.by");
        o.extra_jvm.push(crate::ely::agent_arg(&ensure_agent()?));
    }
    Ok(o)
}

fn has_jar(dir: &std::path::Path, prefix: &str) -> bool {
    std::fs::read_dir(dir).map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().starts_with(prefix))).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{Accounts, Kind};
    use crate::ely::Token;
    use std::sync::Mutex;

    struct RecordingSink {
        stages: Mutex<Vec<(String, String)>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self { stages: Mutex::new(Vec::new()) }
        }
    }

    impl ProgressSink for RecordingSink {
        fn stage(&self, id: &str, text: &str) {
            self.stages.lock().unwrap().push((id.to_string(), text.to_string()));
        }
        fn fraction(&self, _: f32) {}
        fn log(&self, _: &str) {}
    }

    #[test]
    fn with_ely_preserves_agent_even_if_profile_removed_during_downloads() {
        let root = std::env::temp_dir().join(format!(
            "drakestar-play-ely-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let d = Dirs::new(&root);
        let session = Session {
            name: "AlexEly".into(),
            uuid: "069a79f444e94726a5befca90e38aaf5".into(),
            access_token: "fixture-access-token".into(),
            user_type: "mojang".into(),
            xuid: None,
            client_id: None,
        };
        let token = Token {
            access_token: session.access_token.clone(),
            client_token: "fixture-client-token".into(),
        };
        let created = Accounts::new(&d).add_elyby(&session, &token).unwrap();
        // Captura al inicio de play::prepare antes de las descargas:
        let snapshot = Accounts::new(&d).get(&created.id).expect("cuenta presente al iniciar");
        assert_eq!(snapshot.kind, Kind::Elyby);

        // Durante las descargas el usuario retira la cuenta del listado en disco:
        Accounts::new(&d).remove(&created.id).unwrap();
        assert!(Accounts::new(&d).get(&created.id).is_none());

        let sink = RecordingSink::new();
        let fake_agent = d.libraries().join("authlib-injector/authlib-injector-1.2.8.jar");
        let opts = with_ely_agent(&snapshot, Options::default(), &sink, || Ok(fake_agent.clone())).unwrap();
        assert_eq!(
            opts.extra_jvm,
            vec![format!("-javaagent:{}=ely.by", fake_agent.display())]
        );
        let stages = sink.stages.lock().unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].0, "elyby");

        // Las cuentas sin premium o Microsoft no añaden authlib-injector:
        let offline = Accounts::new(&d).add_offline("Invitado").unwrap();
        let sink_off = RecordingSink::new();
        let opts_off = with_ely_agent(&offline, Options::default(), &sink_off, || {
            panic!("no debe invocar ensure_agent para cuentas no Ely.by")
        })
        .unwrap();
        assert!(opts_off.extra_jvm.is_empty());
        assert!(sink_off.stages.lock().unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }
}
