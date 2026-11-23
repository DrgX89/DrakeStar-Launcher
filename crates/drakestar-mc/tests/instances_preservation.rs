//! Un catálogo ilegible no debe sustituirse por las plantillas iniciales.
use drakestar_mc::{
    instances::{Instance, Instances},
    Dirs,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const DAMAGED: &[u8] = b"[{\"id\":\"mi-mundo\",\"name\":\"Perfil recuperable\"";

struct Fixture(Dirs);
impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dirs = Dirs::new(std::env::temp_dir().join(format!(
            "instances-preservation-{}-{nonce}",
            std::process::id()
        )));
        fs::create_dir_all(dirs.root()).unwrap();
        Self(dirs)
    }
    fn path(&self) -> PathBuf {
        self.0.root().join("instances.json")
    }
    fn store(&self) -> Instances {
        Instances::new(&self.0)
    }
    fn instance(&self) -> Instance {
        serde_json::from_value(serde_json::json!({
            "id":"fixture", "name":"Fixture", "mc":"26.2", "loader":"vanilla",
            "target":{"kind":"menu"}, "performance":false, "fused":false,
            "xmx_mb":2048, "extra_jvm":[], "note":""
        }))
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.0.root());
    }
}

#[test]
fn listing_preserves_truncated_catalog() {
    let f = Fixture::new();
    fs::write(f.path(), DAMAGED).unwrap();
    let entries = f.store().list();
    assert!(
        fs::read(f.path()).unwrap() == DAMAGED,
        "Listar no puede destruir los bytes recuperables"
    );
    assert!(
        entries.is_empty(),
        "No fabricar plantillas para un perfil dañado"
    );
}

#[test]
fn fallible_listing_distinguishes_empty_corrupt_and_unreadable_catalogs() {
    let f = Fixture::new();
    fs::write(f.path(), b"[]").unwrap();
    assert!(f.store().try_list().unwrap().is_empty());

    fs::write(f.path(), DAMAGED).unwrap();
    assert!(f.store().try_list().is_err());
    assert_eq!(fs::read(f.path()).unwrap(), DAMAGED);

    let invalid_utf8 = [0xff, 0xfe];
    fs::write(f.path(), invalid_utf8).unwrap();
    assert!(f.store().try_list().is_err());
    assert_eq!(fs::read(f.path()).unwrap(), invalid_utf8);

    fs::remove_file(f.path()).unwrap();
    fs::create_dir(f.path()).unwrap();
    fs::write(f.path().join("sentinel"), b"keep").unwrap();
    assert!(f.store().try_list().is_err());
    assert_eq!(fs::read(f.path().join("sentinel")).unwrap(), b"keep");
}

#[test]
fn upsert_rejects_and_preserves_truncated_catalog() {
    let f = Fixture::new();
    fs::write(f.path(), DAMAGED).unwrap();
    let result = f.store().upsert(f.instance());
    assert!(
        fs::read(f.path()).unwrap() == DAMAGED,
        "Guardar no puede reemplazar un catálogo ilegible"
    );
    assert!(result.is_err());
}

#[test]
fn removal_rejects_and_preserves_truncated_catalog() {
    let f = Fixture::new();
    fs::write(f.path(), DAMAGED).unwrap();
    let result = f.store().remove("fixture");
    assert!(
        fs::read(f.path()).unwrap() == DAMAGED,
        "Retirar no puede reemplazar un catálogo ilegible"
    );
    assert!(result.is_err());
}

#[test]
fn invalid_utf8_catalog_is_preserved() {
    let f = Fixture::new();
    let bytes = [0xff, 0xfe, 0x00];
    fs::write(f.path(), bytes).unwrap();
    let _ = f.store().list();
    assert_eq!(fs::read(f.path()).unwrap(), bytes);
}

#[test]
fn absent_catalog_creates_valid_presets() {
    let f = Fixture::new();
    let entries = f.store().try_list().unwrap();
    assert_eq!(entries.len(), 3);
    for entry in &entries {
        entry.validate().unwrap();
    }
    assert_eq!(
        serde_json::from_slice::<Vec<Instance>>(&fs::read(f.path()).unwrap())
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn valid_catalog_still_supports_updates_and_removal() {
    let f = Fixture::new();
    fs::write(f.path(), b"[]").unwrap();
    let instance = f.instance();
    f.store().upsert(instance).unwrap();
    assert_eq!(f.store().get("fixture").unwrap().name, "Fixture");
    f.store().remove("fixture").unwrap();
    assert!(f.store().list().is_empty());
}

#[test]
fn get_reports_corruption_without_creating_presets() {
    let f = Fixture::new();
    fs::write(f.path(), DAMAGED).unwrap();
    assert!(f.store().get("drakescraft").is_err());
    assert!(fs::read(f.path()).unwrap() == DAMAGED);
}

#[test]
fn schema_errors_are_preserved_by_every_operation() {
    let f = Fixture::new();
    let bytes = br#"[{"id":"unknown-schema","mc":26}]"#;
    fs::write(f.path(), bytes).unwrap();
    assert!(f.store().list().is_empty());
    assert!(f.store().upsert(f.instance()).is_err());
    assert!(f.store().remove("unknown-schema").is_err());
    assert!(f.store().get("unknown-schema").is_err());
    assert!(fs::read(f.path()).unwrap() == bytes);
}

#[test]
fn io_error_keeps_existing_directory_and_rejects_mutations() {
    let f = Fixture::new();
    fs::create_dir(f.path()).unwrap();
    fs::write(f.path().join("sentinel"), b"keep").unwrap();
    assert!(f.store().list().is_empty());
    assert!(f.store().upsert(f.instance()).is_err());
    assert!(f.store().remove("fixture").is_err());
    assert_eq!(fs::read(f.path().join("sentinel")).unwrap(), b"keep");
}

#[test]
fn legacy_solo_migration_preserves_other_instances() {
    use drakestar_mc::instances::{Loader, Target};
    let f = Fixture::new();
    let mut legacy = f.instance();
    legacy.id = "slimefun-solo".into();
    legacy.loader = Loader::Fabric;
    legacy.fused = true;
    let other = f.instance();
    fs::write(f.path(), serde_json::to_vec(&vec![legacy, other]).unwrap()).unwrap();
    let entries = f.store().list();
    assert_eq!(entries.len(), 2);
    let solo = f.store().get("slimefun-solo").unwrap();
    assert!(matches!(solo.target, Target::Local { .. }));
    assert!(!solo.fused);
    assert_eq!(f.store().get("fixture").unwrap().name, "Fixture");
}
