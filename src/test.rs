use crate::utils::{self, Profile, ProjectKind};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_cwd() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("mux_{prefix}_{}_{}", std::process::id(), nanos))
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

#[test]
fn resolve_profiles_variants() {
    assert_eq!(
        utils::resolve_profiles(false, false).unwrap(),
        vec![Profile::Debug]
    );
    assert_eq!(
        utils::resolve_profiles(true, false).unwrap(),
        vec![Profile::Release]
    );
    assert_eq!(
        utils::resolve_profiles(false, true).unwrap(),
        vec![Profile::Debug, Profile::Release]
    );
    assert!(utils::resolve_profiles(true, true).is_err());
}

#[test]
fn ensure_project_root_detects_layouts() {
    let _guard = lock_cwd();
    let original = env::current_dir().unwrap();
    let dir = unique_temp_dir("layout");
    fs::create_dir_all(&dir).unwrap();
    env::set_current_dir(&dir).unwrap();

    assert!(utils::ensure_project_root().is_err());

    write_text(
        Path::new("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.20)\n",
    );
    write_text(Path::new("CMakePresets.json"), "{}\n");
    write_text(Path::new("src/main.c"), "int main(void){return 0;}\n");
    write_text(
        Path::new("tests/test_main.c"),
        "int main(void){return 0;}\n",
    );
    assert!(utils::ensure_project_root().is_ok());

    fs::remove_dir_all("src").unwrap();
    fs::remove_dir_all("tests").unwrap();
    write_text(Path::new("src/main.cpp"), "int main(){return 0;}\n");
    write_text(Path::new("tests/test_main.cpp"), "int main(){return 0;}\n");
    assert!(utils::ensure_project_root().is_ok());

    env::set_current_dir(original).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn init_project_c_and_cpp_layouts() {
    let _guard = lock_cwd();
    let original = env::current_dir().unwrap();
    let dir = unique_temp_dir("init");
    fs::create_dir_all(&dir).unwrap();
    env::set_current_dir(&dir).unwrap();

    utils::init_project("proj_c", ProjectKind::C).unwrap();
    assert!(Path::new("proj_c/src/main.c").exists());
    assert!(Path::new("proj_c/tests/test_main.c").exists());
    assert!(Path::new("proj_c/lib/unity/unity.c").exists());

    utils::init_project("proj_cpp", ProjectKind::Cpp).unwrap();
    assert!(Path::new("proj_cpp/src/main.cpp").exists());
    assert!(Path::new("proj_cpp/tests/test_main.cpp").exists());

    env::set_current_dir(original).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn add_and_remove_dependency_local_path() {
    let _guard = lock_cwd();
    let original = env::current_dir().unwrap();
    let dir = unique_temp_dir("add_remove");
    fs::create_dir_all(&dir).unwrap();
    env::set_current_dir(&dir).unwrap();

    utils::init_project("proj", ProjectKind::C).unwrap();

    let dep_src = dir.join("dep-lib");
    fs::create_dir_all(&dep_src).unwrap();
    write_text(&dep_src.join("dep.h"), "int dep_add(int a, int b);\n");
    write_text(
        &dep_src.join("dep.c"),
        "#include \"dep.h\"\nint dep_add(int a, int b){return a+b;}\n",
    );

    env::set_current_dir(dir.join("proj")).unwrap();
    utils::add_dependency(dep_src.to_string_lossy().as_ref(), None).unwrap();

    assert!(Path::new("lib/dep-lib/dep.c").exists());
    let cmake = fs::read_to_string("CMakeLists.txt").unwrap();
    assert!(cmake.contains("# mux:dep begin dep-lib"));
    assert!(cmake.contains("target_link_libraries(main PRIVATE dep-lib)"));
    assert!(Path::new("mux.lock").exists());

    utils::remove_dependency("dep-lib").unwrap();
    assert!(!Path::new("lib/dep-lib").exists());
    let cmake = fs::read_to_string("CMakeLists.txt").unwrap();
    assert!(!cmake.contains("# mux:dep begin dep-lib"));

    env::set_current_dir(original).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn clean_outputs_modes() {
    let _guard = lock_cwd();
    let original = env::current_dir().unwrap();
    let dir = unique_temp_dir("clean");
    fs::create_dir_all(&dir).unwrap();
    env::set_current_dir(&dir).unwrap();

    fs::create_dir_all("out/unix-debug").unwrap();
    fs::create_dir_all("out/unix-release").unwrap();

    utils::clean_outputs(false, false).unwrap();
    assert!(!Path::new("out/unix-debug").exists());
    assert!(Path::new("out/unix-release").exists());

    utils::clean_outputs(true, false).unwrap();
    assert!(!Path::new("out").exists());

    env::set_current_dir(original).unwrap();
    fs::remove_dir_all(dir).unwrap();
}
