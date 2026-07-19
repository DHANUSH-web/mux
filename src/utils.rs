use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    Debug,
    Release,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectKind {
    C,
    Cpp,
}

pub fn resolve_profiles(release: bool, all: bool) -> Result<Vec<Profile>> {
    if release && all {
        bail!("--release and --all cannot be used together");
    }

    if all {
        Ok(vec![Profile::Debug, Profile::Release])
    } else if release {
        Ok(vec![Profile::Release])
    } else {
        Ok(vec![Profile::Debug])
    }
}

pub fn ensure_project_root() -> Result<()> {
    for file in ["CMakeLists.txt", "CMakePresets.json"] {
        if !Path::new(file).exists() {
            bail!("current directory is not a mux project (missing {file})");
        }
    }

    let has_c_layout = Path::new("src/main.c").exists() && Path::new("tests/test_main.c").exists();
    let has_cpp_layout =
        Path::new("src/main.cpp").exists() && Path::new("tests/test_main.cpp").exists();
    if !has_c_layout && !has_cpp_layout {
        bail!(
            "current directory is not a mux project (missing src/tests template files). Run this command inside an initialized project"
        );
    }
    Ok(())
}

fn ensure_c_project_root() -> Result<()> {
    ensure_project_root()?;
    let required = ["src/main.c", "tests/test_main.c"];
    for file in required {
        if !Path::new(file).exists() {
            bail!("mux add currently supports only C template projects (missing {file})");
        }
    }
    Ok(())
}

pub fn add_dependency(spec: &str, target_override: Option<String>) -> Result<()> {
    ensure_c_project_root()?;
    fs::create_dir_all("lib").context("failed to ensure lib directory exists")?;

    let source = resolve_dependency_source(spec)?;
    let dep_name = sanitize_name(&source.name);
    let dep_dir = PathBuf::from("lib").join(&dep_name);
    if dep_dir.exists() {
        bail!(
            "dependency directory '{}' already exists",
            dep_dir.display()
        );
    }

    match &source.kind {
        DependencyKind::LocalPath(path) => {
            copy_dir_recursive(&path, &dep_dir).with_context(|| {
                format!(
                    "failed to copy local dependency from '{}' to '{}'",
                    path.display(),
                    dep_dir.display()
                )
            })?;
        }
        DependencyKind::GitUrl(url) => {
            let dep_dir_string = dep_dir.to_string_lossy().to_string();
            run_command("git", &["clone", "--depth", "1", url, &dep_dir_string])?;
            let git_dir = dep_dir.join(".git");
            if git_dir.exists() {
                fs::remove_dir_all(&git_dir)
                    .with_context(|| format!("failed to remove {}", git_dir.display()))?;
            }
        }
    }

    let include_path = format!("lib/{}", dep_name);
    let has_cmake = dep_dir.join("CMakeLists.txt").exists();
    let has_sources = has_c_sources(&dep_dir)?;
    let target_name = sanitize_name(target_override.as_deref().unwrap_or(&dep_name));
    let cmake_block = render_dependency_cmake_block(
        &dep_name,
        &include_path,
        &target_name,
        has_cmake,
        has_sources,
    );

    append_to_file("CMakeLists.txt", &cmake_block)?;
    let source_ref = match &source.kind {
        DependencyKind::LocalPath(path) => format!("path:{}", path.display()),
        DependencyKind::GitUrl(url) => format!("git:{url}"),
    };
    update_lock_add(LockEntry {
        name: dep_name.clone(),
        source: source_ref,
        target: target_name.clone(),
        include: include_path.clone(),
    })?;

    println!("Added dependency '{}' into {}", dep_name, dep_dir.display());
    println!("Updated CMakeLists.txt");
    println!("Updated mux.lock");
    println!("You can now use includes from '{}'", include_path);
    println!("Run `mux build` to verify linkage.");

    Ok(())
}

pub fn remove_dependency(lib: &str) -> Result<()> {
    ensure_c_project_root()?;

    let dep_name = sanitize_name(lib);
    let dep_dir = PathBuf::from("lib").join(&dep_name);
    let mut removed_any = false;

    if dep_dir.exists() {
        fs::remove_dir_all(&dep_dir)
            .with_context(|| format!("failed to remove {}", dep_dir.display()))?;
        println!("Removed {}", dep_dir.display());
        removed_any = true;
    }

    let cmake_path = Path::new("CMakeLists.txt");
    let cmake = fs::read_to_string(cmake_path)
        .with_context(|| format!("failed to read {}", cmake_path.display()))?;
    let (updated, removed_block) = strip_dependency_block(&cmake, &dep_name);
    if removed_block {
        fs::write(cmake_path, updated)
            .with_context(|| format!("failed to write {}", cmake_path.display()))?;
        println!("Removed dependency block from CMakeLists.txt");
        removed_any = true;
    }

    let removed_lock = update_lock_remove(&dep_name)?;
    if removed_lock {
        println!("Updated mux.lock");
        removed_any = true;
    }

    if !removed_any {
        bail!("dependency '{}' was not found", dep_name);
    }

    Ok(())
}

fn append_to_file(path: impl AsRef<Path>, text: &str) -> Result<()> {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(path.as_ref())
        .with_context(|| format!("failed to open {} for append", path.as_ref().display()))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("failed to append to {}", path.as_ref().display()))
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).with_context(|| format!("failed to create {}", to.display()))?;
    for entry in fs::read_dir(from).with_context(|| format!("failed to read {}", from.display()))? {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = to.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&entry_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&entry_path, &target_path).with_context(|| {
                format!(
                    "failed to copy '{}' to '{}'",
                    entry_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn has_c_sources(root: &Path) -> Result<bool> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in
            fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if file_type.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("c"))
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn render_dependency_cmake_block(
    dep_name: &str,
    include_path: &str,
    target_name: &str,
    has_cmake: bool,
    has_sources: bool,
) -> String {
    let upper = dep_name.to_ascii_uppercase();
    let link_block = format!(
        "target_link_libraries(main PRIVATE {target})\n\
target_link_libraries(test_main PRIVATE {target})\n",
        target = target_name
    );

    let body = if has_cmake {
        format!(
            "set({upper}_BUILD_TESTS OFF CACHE BOOL \"\" FORCE)\n\
set({upper}_BUILD_TESTING OFF CACHE BOOL \"\" FORCE)\n\
set(ENABLE_{upper}_TEST OFF CACHE BOOL \"\" FORCE)\n\
set(ENABLE_{upper}_TESTS OFF CACHE BOOL \"\" FORCE)\n\
set(BUILD_TESTS OFF CACHE BOOL \"\" FORCE)\n\
set(BUILD_TESTING OFF CACHE BOOL \"\" FORCE)\n\
add_subdirectory({include_dir})\n\
target_include_directories(main PRIVATE {include_dir})\n\
target_include_directories(test_main PRIVATE {include_dir})\n\
{link}",
            upper = upper,
            include_dir = include_path,
            link = link_block
        )
    } else if has_sources {
        format!(
            "file(GLOB_RECURSE MUX_{upper}_SOURCES CONFIGURE_DEPENDS {include_dir}/*.c)\n\
add_library({target} STATIC ${{MUX_{upper}_SOURCES}})\n\
target_include_directories({target} PUBLIC {include_dir})\n\
{link}",
            upper = upper,
            include_dir = include_path,
            target = target_name,
            link = link_block
        )
    } else {
        format!(
            "target_include_directories(main PRIVATE {include_dir})\n\
target_include_directories(test_main PRIVATE {include_dir})\n",
            include_dir = include_path
        )
    };

    format!(
        "\n# mux:dep begin {dep}\n\
if(NOT MUX_DEP_{upper}_ADDED)\n\
  set(MUX_DEP_{upper}_ADDED ON)\n\
{body}\
endif()\n\
# mux:dep end {dep}\n",
        dep = dep_name,
        upper = upper,
        body = indent_cmake(&body, 2)
    )
}

fn indent_cmake(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{prefix}{line}\n"))
        .collect::<String>()
}

fn sanitize_name(input: &str) -> String {
    let mapped = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let trimmed = mapped.trim_matches('_').trim_matches('-');
    if trimmed.is_empty() {
        "dep".to_string()
    } else {
        trimmed.to_string()
    }
}

struct DependencySource {
    name: String,
    kind: DependencyKind,
}

enum DependencyKind {
    LocalPath(PathBuf),
    GitUrl(String),
}

fn resolve_dependency_source(spec: &str) -> Result<DependencySource> {
    let local_path = Path::new(spec);
    if local_path.exists() {
        let name = local_path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "dep".to_string());
        return Ok(DependencySource {
            name,
            kind: DependencyKind::LocalPath(local_path.to_path_buf()),
        });
    }

    if spec.contains("://") || spec.starts_with("git@") {
        let name = infer_repo_name(spec);
        return Ok(DependencySource {
            name,
            kind: DependencyKind::GitUrl(spec.to_string()),
        });
    }

    if spec.contains('/') {
        let name = infer_repo_name(spec);
        let url = format!("https://github.com/{spec}.git");
        return Ok(DependencySource {
            name,
            kind: DependencyKind::GitUrl(url),
        });
    }

    let url = format!("https://github.com/{name}/{name}.git", name = spec);
    Ok(DependencySource {
        name: spec.to_string(),
        kind: DependencyKind::GitUrl(url),
    })
}

fn infer_repo_name(spec: &str) -> String {
    let trimmed = spec.trim_end_matches(".git").trim_end_matches('/');
    trimmed
        .rsplit(['/', ':'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("dep")
        .to_string()
}

fn strip_dependency_block(cmake: &str, dep_name: &str) -> (String, bool) {
    let begin = format!("# mux:dep begin {dep_name}");
    let end = format!("# mux:dep end {dep_name}");
    let mut output = Vec::new();
    let mut skipping = false;
    let mut removed = false;

    for line in cmake.lines() {
        if line.trim() == begin {
            skipping = true;
            removed = true;
            continue;
        }
        if skipping && line.trim() == end {
            skipping = false;
            continue;
        }
        if !skipping {
            output.push(line);
        }
    }

    let mut rebuilt = output.join("\n");
    if cmake.ends_with('\n') {
        rebuilt.push('\n');
    }
    (rebuilt, removed)
}

#[derive(Clone)]
struct LockEntry {
    name: String,
    source: String,
    target: String,
    include: String,
}

fn lock_path() -> &'static Path {
    Path::new("mux.lock")
}

fn read_lock_entries() -> Result<Vec<LockEntry>> {
    let path = lock_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut entries = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.len() != 4 {
            bail!(
                "invalid mux.lock format at line {} (expected 4 tab-separated columns)",
                idx + 1
            );
        }
        entries.push(LockEntry {
            name: parts[0].to_string(),
            source: parts[1].to_string(),
            target: parts[2].to_string(),
            include: parts[3].to_string(),
        });
    }
    Ok(entries)
}

fn write_lock_entries(entries: &[LockEntry]) -> Result<()> {
    let mut text = String::from("# mux lock v1\n# name\\tsource\\ttarget\\tinclude\n");
    for entry in entries {
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            entry.name, entry.source, entry.target, entry.include
        ));
    }
    fs::write(lock_path(), text).context("failed to write mux.lock")
}

fn update_lock_add(entry: LockEntry) -> Result<()> {
    let mut entries = read_lock_entries()?;
    if let Some(existing) = entries.iter_mut().find(|e| e.name == entry.name) {
        *existing = entry;
    } else {
        entries.push(entry);
    }
    write_lock_entries(&entries)
}

fn update_lock_remove(name: &str) -> Result<bool> {
    let mut entries = read_lock_entries()?;
    let initial = entries.len();
    entries.retain(|e| e.name != name);
    if entries.len() != initial {
        write_lock_entries(&entries)?;
        return Ok(true);
    }
    Ok(false)
}

pub fn init_project(name: &str, kind: ProjectKind) -> Result<()> {
    let root = PathBuf::from(name);
    if root.exists() {
        bail!("target directory '{}' already exists", root.display());
    }

    fs::create_dir_all(root.join("src"))?;
    fs::create_dir_all(root.join("tests"))?;

    write_file(root.join(".gitignore"), GITIGNORE)?;
    write_file(root.join("CMakePresets.json"), CMAKE_PRESETS)?;

    match kind {
        ProjectKind::C => {
            fs::create_dir_all(root.join("lib/unity"))?;
            write_file(root.join("CMakeLists.txt"), CMAKE_LISTS_C)?;
            write_file(root.join("src/main.c"), MAIN_C)?;
            write_file(root.join("tests/test_main.c"), TEST_MAIN_C)?;
            write_file(root.join("lib/unity/unity.h"), UNITY_H)?;
            write_file(root.join("lib/unity/unity.c"), UNITY_C)?;
        }
        ProjectKind::Cpp => {
            fs::create_dir_all(root.join("lib"))?;
            write_file(root.join("CMakeLists.txt"), CMAKE_LISTS_CPP)?;
            write_file(root.join("src/main.cpp"), MAIN_CPP)?;
            write_file(root.join("tests/test_main.cpp"), TEST_MAIN_CPP)?;
            write_file(root.join("lib/README.md"), LIB_CPP_README)?;
        }
    }

    println!(
        "Initialized {} project template in '{}'",
        project_kind_name(kind),
        root.display()
    );
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  mux build");
    println!("  mux run");

    Ok(())
}

fn write_file(path: impl AsRef<Path>, contents: &str) -> Result<()> {
    fs::write(path.as_ref(), contents)
        .with_context(|| format!("failed to write {}", path.as_ref().display()))
}

pub fn configure_and_build(profile: Profile) -> Result<()> {
    let preset = preset_name(profile);
    println!("==> Configuring preset: {preset}");
    run_command("cmake", &["--preset", &preset])?;

    println!("==> Building preset: {preset}");
    run_command("cmake", &["--build", "--preset", &preset])
}

pub fn run_ctest(profile: Profile) -> Result<()> {
    let preset = preset_name(profile);
    println!("==> Running tests preset: {preset}");
    run_command("ctest", &["--preset", &preset])
}

pub fn run_executable(profile: Profile) -> Result<()> {
    let preset = preset_name(profile);
    let mut exe = PathBuf::from("out").join(&preset).join("main");
    if cfg!(windows) {
        exe.set_extension("exe");
    }

    if !exe.exists() {
        bail!(
            "expected executable '{}' was not found after build",
            exe.display()
        );
    }

    println!("==> Running {}", exe.display());
    run_command(exe.to_string_lossy().as_ref(), &[])
}

pub fn clean_outputs(all: bool, release: bool) -> Result<()> {
    let out = Path::new("out");

    if all {
        if out.exists() {
            fs::remove_dir_all(out).context("failed to remove out directory")?;
            println!("Cleaned all builds");
        } else {
            println!("Nothing to clean: Project is clean");
        }
        return Ok(());
    }

    let profile = if release {
        Profile::Release
    } else {
        Profile::Debug
    };

    let target = out.join(preset_name(profile));
    if target.exists() {
        fs::remove_dir_all(&target)
            .with_context(|| format!("failed to remove {}", target.display()))?;
        println!("Removed {}", target.display());
    } else {
        println!("Nothing to clean: {} does not exist", target.display());
    }

    Ok(())
}

fn run_command(program: &str, args: &[&str]) -> Result<()> {
    let rendered_args = args.join(" ");
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run command: {} {}", program, rendered_args))?;

    if !status.success() {
        bail!("command failed: {} {}", program, rendered_args);
    }
    Ok(())
}

fn preset_name(profile: Profile) -> String {
    format!("{}-{}", platform_prefix(), profile_name(profile))
}

fn platform_prefix() -> &'static str {
    if cfg!(windows) { "windows" } else { "unix" }
}

fn profile_name(profile: Profile) -> &'static str {
    match profile {
        Profile::Debug => "debug",
        Profile::Release => "release",
    }
}

fn project_kind_name(kind: ProjectKind) -> &'static str {
    match kind {
        ProjectKind::C => "C",
        ProjectKind::Cpp => "C++",
    }
}

const GITIGNORE: &str = "out/\n";

const CMAKE_LISTS_C: &str = r#"cmake_minimum_required(VERSION 3.20)
project(mux_project C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

add_library(unity STATIC lib/unity/unity.c)
target_include_directories(unity PUBLIC lib/unity)

add_executable(main src/main.c)
target_link_libraries(main PRIVATE unity)

add_executable(test_main tests/test_main.c)
target_link_libraries(test_main PRIVATE unity)

enable_testing()
add_test(NAME unit_tests COMMAND test_main)
"#;

const CMAKE_LISTS_CPP: &str = r#"cmake_minimum_required(VERSION 3.20)
project(mux_project CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

include(FetchContent)
FetchContent_Declare(
  googletest
  URL https://github.com/google/googletest/archive/refs/tags/v1.14.0.zip
  DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)

add_executable(main src/main.cpp)

enable_testing()
add_executable(test_main tests/test_main.cpp)
target_link_libraries(test_main PRIVATE GTest::gtest_main)
include(GoogleTest)
gtest_discover_tests(test_main)
"#;

const CMAKE_PRESETS: &str = r#"{
  "version": 6,
  "cmakeMinimumRequired": {
    "major": 3,
    "minor": 20,
    "patch": 0
  },
  "configurePresets": [
    {
      "name": "unix-debug",
      "displayName": "Unix Debug",
      "generator": "Ninja",
      "description": "Debug build preset for Linux/macOS",
      "binaryDir": "${sourceDir}/out/unix-debug",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Debug"
      },
      "condition": {
        "type": "anyOf",
        "conditions": [
          {
            "type": "equals",
            "lhs": "${hostSystemName}",
            "rhs": "Linux"
          },
          {
            "type": "equals",
            "lhs": "${hostSystemName}",
            "rhs": "Darwin"
          }
        ]
      }
    },
    {
      "name": "unix-release",
      "displayName": "Unix Release",
      "generator": "Ninja",
      "description": "Release build preset for Linux/macOS",
      "binaryDir": "${sourceDir}/out/unix-release",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Release"
      },
      "condition": {
        "type": "anyOf",
        "conditions": [
          {
            "type": "equals",
            "lhs": "${hostSystemName}",
            "rhs": "Linux"
          },
          {
            "type": "equals",
            "lhs": "${hostSystemName}",
            "rhs": "Darwin"
          }
        ]
      }
    },
    {
      "name": "windows-debug",
      "displayName": "Windows Debug",
      "generator": "Ninja",
      "description": "Debug build preset for Windows",
      "binaryDir": "${sourceDir}/out/windows-debug",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Debug"
      },
      "condition": {
        "type": "equals",
        "lhs": "${hostSystemName}",
        "rhs": "Windows"
      }
    },
    {
      "name": "windows-release",
      "displayName": "Windows Release",
      "generator": "Ninja",
      "description": "Release build preset for Windows",
      "binaryDir": "${sourceDir}/out/windows-release",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Release"
      },
      "condition": {
        "type": "equals",
        "lhs": "${hostSystemName}",
        "rhs": "Windows"
      }
    }
  ],
  "buildPresets": [
    {
      "name": "unix-debug",
      "configurePreset": "unix-debug"
    },
    {
      "name": "unix-release",
      "configurePreset": "unix-release"
    },
    {
      "name": "windows-debug",
      "configurePreset": "windows-debug"
    },
    {
      "name": "windows-release",
      "configurePreset": "windows-release"
    }
  ],
  "testPresets": [
    {
      "name": "unix-debug",
      "configurePreset": "unix-debug",
      "output": {
        "outputOnFailure": true
      }
    },
    {
      "name": "unix-release",
      "configurePreset": "unix-release",
      "output": {
        "outputOnFailure": true
      }
    },
    {
      "name": "windows-debug",
      "configurePreset": "windows-debug",
      "output": {
        "outputOnFailure": true
      }
    },
    {
      "name": "windows-release",
      "configurePreset": "windows-release",
      "output": {
        "outputOnFailure": true
      }
    }
  ]
}
"#;

const MAIN_C: &str = r#"#include <stdio.h>
#include "unity.h"

int main(void) {
    printf("Hello from your mux C starter project!\n");

    // Optional: Unity assertion available in main target too.
    TEST_ASSERT_EQUAL_INT(2, 1 + 1);

    return 0;
}
"#;

const MAIN_CPP: &str = r#"#include <iostream>

int main() {
    std::cout << "Hello from your mux C++ starter project!" << std::endl;
    return 0;
}
"#;

const TEST_MAIN_C: &str = r#"#include "unity.h"

void setUp(void) {}
void tearDown(void) {}

static void test_sample_addition(void) {
    TEST_ASSERT_EQUAL_INT(4, 2 + 2);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_sample_addition);
    return UNITY_END();
}
"#;

const TEST_MAIN_CPP: &str = r#"#include <gtest/gtest.h>

TEST(SampleTest, BasicMath) {
    EXPECT_EQ(4, 2 + 2);
}
"#;

const UNITY_H: &str = r#"#ifndef UNITY_H
#define UNITY_H

#ifdef __cplusplus
extern "C" {
#endif

int UnityBegin(const char *filename);
int UnityEnd(void);
void UnityDefaultTestRun(void (*test_func)(void), const char *name, int line_num);
void UnityAssertEqualInt(int expected, int actual, int line_num, const char *file);

#define UNITY_BEGIN() UnityBegin(__FILE__)
#define UNITY_END() UnityEnd()
#define RUN_TEST(func) UnityDefaultTestRun(func, #func, __LINE__)
#define TEST_ASSERT_EQUAL_INT(expected, actual) \
    UnityAssertEqualInt((expected), (actual), __LINE__, __FILE__)

#ifdef __cplusplus
}
#endif

#endif
"#;

const UNITY_C: &str = r#"#include "unity.h"

#include <stdio.h>

static int g_failures = 0;
static int g_tests_run = 0;

int UnityBegin(const char *filename) {
    g_failures = 0;
    g_tests_run = 0;
    printf("==== Unity test run: %s ====\n", filename);
    return 0;
}

int UnityEnd(void) {
    printf("==== Tests: %d, Failures: %d ====\n", g_tests_run, g_failures);
    return g_failures;
}

void UnityDefaultTestRun(void (*test_func)(void), const char *name, int line_num) {
    ++g_tests_run;
    printf("[RUN] %s (line %d)\n", name, line_num);
    test_func();
}

void UnityAssertEqualInt(int expected, int actual, int line_num, const char *file) {
    if (expected != actual) {
        ++g_failures;
        printf("[FAIL] %s:%d expected %d but was %d\n", file, line_num, expected, actual);
    }
}
"#;

const LIB_CPP_README: &str = r#"# lib/

This project uses GoogleTest for unit tests via CMake `FetchContent`.
No local library sources are required in `lib/` for the default C++ template.
"#;
