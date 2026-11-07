//! Fachada de aplicacion: une almacen, procesos, servidor y releases.
//! Es lo que llaman los comandos Tauri (y los tests de integracion).

use crate::{
    instances,
    process::{EventSink, ProcessManager, ProcStatus},
    release,
    server::ServerDir,
    settings::Settings,
    store::ProfileStore,
    AppError, AppResult, Layout,
};
use drakestar_core::{launch, Mode, Profile};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Service {
    pub layout: Layout,
    pub store: ProfileStore,
    pub procs: ProcessManager,
}

impl Service {
    pub fn new(layout: Layout, sink: Arc<dyn EventSink>) -> Self {
        let _ = std::fs::create_dir_all(layout.profiles_dir());
        Self { store: ProfileStore::new(layout.clone()), procs: ProcessManager::new(sink), layout }
    }

    pub fn settings(&self) -> Settings {
        Settings::load(&self.layout)
    }

    pub fn server_dir(&self, id: &str) -> AppResult<(Profile, ServerDir)> {
        let p = self.store.get(id)?;
        if p.mode != Mode::LocalServer {
            return Err(AppError::Invalid(format!("'{id}' no es un perfil local-server")));
        }
        Ok((p, ServerDir::new(self.layout.server_dir(id))))
    }

    pub fn create_profile(&self, p: &Profile) -> AppResult<()> {
        self.store.create(p)?;
        instances::ensure_dirs(&self.layout, p)
    }

    /// Plan de arranque del servidor local (sin ejecutar).
    pub fn server_plan(&self, id: &str) -> AppResult<launch::LaunchPlan> {
        let (p, sd) = self.server_dir(id)?;
        let jar = sd
            .jar()
            .selected
            .ok_or_else(|| AppError::NotFound(format!("no hay un unico JAR de servidor en {} (coloca {} o instala una release)", sd.dir.display(), crate::server::DEFAULT_JAR)))?;
        let root = self.layout.root().to_string_lossy().to_string();
        launch::local_server_plan(&p, &self.settings().java_bin(), &jar, &root)
            .ok_or_else(|| AppError::Invalid("el perfil no es local-server".into()))
    }

    /// Arranca el servidor. Exige EULA aceptada (si no, el servidor se cierra solo y confunde).
    pub fn server_start(&self, id: &str) -> AppResult<ProcStatus> {
        let (p, sd) = self.server_dir(id)?;
        instances::ensure_dirs(&self.layout, &p)?;
        let plan = self.server_plan(id)?;
        if sd.eula() != Some(true) {
            return Err(AppError::Invalid(
                "EULA de Minecraft sin aceptar: aceptala desde la pagina Local Forge antes de arrancar".into(),
            ));
        }
        self.procs.start(id, &plan, &self.layout.logs_dir(id))
    }

    pub fn server_stop(&self, id: &str) -> AppResult<()> {
        self.server_dir(id)?;
        self.procs.stop(id, Some("stop"), Duration::from_secs(30))
    }

    pub fn server_command(&self, id: &str, line: &str) -> AppResult<()> {
        self.server_dir(id)?;
        self.procs.send_line(id, line.trim())
    }

    pub fn delete_profile(&self, id: &str, remove_data: bool) -> AppResult<()> {
        if self.procs.is_active(id) {
            return Err(AppError::Busy(format!("'{id}' esta en ejecucion: detenlo antes de borrarlo")));
        }
        self.store.delete(id, remove_data)?;
        let mut s = self.settings();
        if s.active_profile.as_deref() == Some(id) {
            s.active_profile = None;
            s.save(&self.layout)?;
        }
        Ok(())
    }

    pub fn install_server_jar(&self, id: &str) -> AppResult<u64> {
        let (p, sd) = self.server_dir(id)?;
        sd.ensure()?;
        release::install_server_jar(&self.layout, &p, release::load_manifest(&self.layout).as_ref())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::process::{ProcEvent, ProcState};
    use crate::store::tests::sample;
    use std::sync::Mutex;
    use std::time::Instant;

    #[derive(Default)]
    struct Collect(Mutex<Vec<ProcEvent>>);
    impl EventSink for Collect {
        fn emit(&self, ev: ProcEvent) {
            self.0.lock().unwrap().push(ev);
        }
    }

    fn wait_until(f: impl Fn() -> bool) {
        let t = Instant::now();
        while !f() {
            assert!(t.elapsed() < Duration::from_secs(20), "timeout");
            std::thread::sleep(Duration::from_millis(30));
        }
    }

    #[test]
    fn start_requires_jar_and_eula() {
        let svc = Service::new(Layout::new(crate::testutil::tmpdir("svc")), Arc::new(Collect::default()));
        svc.create_profile(&sample("srv", Mode::LocalServer)).unwrap();
        assert!(matches!(svc.server_start("srv"), Err(AppError::NotFound(_))));
        let sd = ServerDir::new(svc.layout.server_dir("srv"));
        std::fs::write(sd.dir.join("drakespurpur.jar"), b"x").unwrap();
        let e = svc.server_start("srv").unwrap_err();
        assert!(e.to_string().contains("EULA"));
        let plan = svc.server_plan("srv").unwrap();
        assert!(plan.args.contains(&"-jar".to_string()) && plan.args.contains(&"drakespurpur.jar".to_string()));
        assert!(svc.server_start("nope").is_err());
        let e = svc.create_profile(&sample("on", Mode::Online)).and_then(|_| svc.server_start("on")).unwrap_err();
        assert!(e.to_string().contains("local-server"));
    }

    /// Prueba de extremo a extremo con Java real: compila un mini "servidor" de
    /// juguete (NO es DrakesPurpur) que imprime, lee stdin y sale con "stop", lo
    /// empaqueta como JAR y lo ejecuta con el plan de drakestar-core.
    /// Se omite (pasa sin ejecutar) si no hay javac/jar.
    #[test]
    fn end_to_end_with_toy_jar_if_jdk_present() {
        let have = |b: &str| std::process::Command::new(b).arg("-version").output().is_ok() || b == "jar";
        if !have("javac") || std::process::Command::new("jar").arg("--version").output().is_err() {
            eprintln!("omitido: no hay JDK");
            return;
        }
        let work = crate::testutil::tmpdir("toyjar");
        std::fs::write(
            work.join("Toy.java"),
            r#"public class Toy { public static void main(String[] a) throws Exception {
  System.out.println("Toy server listo args=" + a.length);
  System.err.println("aviso en stderr");
  java.io.BufferedReader r = new java.io.BufferedReader(new java.io.InputStreamReader(System.in));
  String l; while ((l = r.readLine()) != null) { System.out.println("cmd:" + l); if (l.equals("stop")) { System.out.println("adios"); return; } }
} }"#,
        )
        .unwrap();
        std::fs::write(work.join("manifest.txt"), "Main-Class: Toy\n").unwrap();
        let ok = |c: &mut std::process::Command| c.current_dir(&work).output().map(|o| o.status.success()).unwrap_or(false);
        if !ok(std::process::Command::new("javac").arg("Toy.java"))
            || !ok(std::process::Command::new("jar").args(["cfm", "toy.jar", "manifest.txt", "Toy.class"]))
        {
            eprintln!("omitido: no se pudo compilar el JAR de prueba");
            return;
        }
        let sink = Arc::new(Collect::default());
        let svc = Service::new(Layout::new(crate::testutil::tmpdir("svc-e2e")), sink.clone());
        svc.create_profile(&sample("e2e", Mode::LocalServer)).unwrap();
        let sd = ServerDir::new(svc.layout.server_dir("e2e"));
        std::fs::copy(work.join("toy.jar"), sd.dir.join("drakespurpur.jar")).unwrap();
        sd.accept_eula().unwrap();
        svc.server_start("e2e").unwrap();
        wait_until(|| svc.procs.history("e2e").iter().any(|l| l.line.starts_with("Toy server listo")));
        svc.server_command("e2e", "list").unwrap();
        wait_until(|| svc.procs.history("e2e").iter().any(|l| l.line == "cmd:list"));
        assert!(svc.procs.history("e2e").iter().any(|l| l.line == "aviso en stderr"));
        svc.server_stop("e2e").unwrap();
        wait_until(|| svc.procs.status("e2e").state == ProcState::Stopped);
        assert!(svc.procs.history("e2e").iter().any(|l| l.line == "adios"));
        let logs = instances::list_logs(&svc.layout, &svc.store.get("e2e").unwrap());
        assert_eq!(logs.len(), 1);
        let tail = instances::read_log_tail(&svc.layout, &svc.store.get("e2e").unwrap(), "launcher", &logs[0].name, 50).unwrap();
        assert!(tail.iter().any(|l| l.contains("[in ] stop")) && tail.iter().any(|l| l.contains("[out] adios")));
        assert!(svc.delete_profile("e2e", true).is_ok());
    }
}
