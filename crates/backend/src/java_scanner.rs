use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct JavaInstallation {
    pub path: PathBuf,
    pub version: String,
}

pub async fn scan_java_installations() -> Vec<PathBuf> {
    let mut installations = Vec::new();
    
    // Check JAVA_HOME
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let path = PathBuf::from(java_home);
        check_and_add_java(&path, &mut installations);
    }

    // Check PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for path_str in std::env::split_paths(&path_var) {
            check_and_add_java_binary(&path_str, &mut installations);
        }
    }

    // Standard directories
    #[cfg(target_os = "windows")]
    {
        let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
        let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
        
        let paths = vec![
            PathBuf::from(program_files).join("Java"),
            PathBuf::from(program_files_x86).join("Java"),
            PathBuf::from("C:\\Program Files\\Eclipse Adoptium"), // Common for Temurin
            PathBuf::from("C:\\Program Files\\Microsoft\\jdk"),
        ];

        for path in paths {
            if path.exists() {
                scan_directory(&path, &mut installations);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let paths = vec![
            PathBuf::from("/Library/Java/JavaVirtualMachines"),
            PathBuf::from("/System/Library/Java/JavaVirtualMachines"),
        ];
        
        for path in paths {
            if path.exists() {
                scan_directory(&path, &mut installations);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let paths = vec![
            PathBuf::from("/usr/lib/jvm"),
            PathBuf::from("/usr/java"),
        ];

        for path in paths {
            if path.exists() {
                scan_directory(&path, &mut installations);
            }
        }
    }

    // Dedup
    installations.sort();
    installations.dedup();

    installations
}

fn scan_directory(path: &Path, installations: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                check_and_add_java(&path, installations);
            }
        }
    }
}

fn check_and_add_java(root: &Path, installations: &mut Vec<PathBuf>) {
    let bin = root.join("bin");
    if bin.exists() {
        check_and_add_java_binary(&bin, installations);
    } else {
         // Maybe the root itself is the bin dir? Rare but possible if user pointed there.
         // Actually usually root has bin/java
         // Let's also check if root joined with javaw.exe or java exists directly (e.g. if user pointed deep)
         check_and_add_java_binary(root, installations);
    }
}

fn check_and_add_java_binary(path: &Path, installations: &mut Vec<PathBuf>) {
    let java_names = if cfg!(windows) {
        vec!["javaw.exe", "java.exe"]
    } else {
        vec!["java"]
    };

    for name in java_names {
        let java_path = path.join(name);
        if java_path.exists() {
             if !installations.contains(&java_path) {
                 installations.push(java_path.clone());
             }
             // If we found one, we generally don't need both java and javaw from the same dir in the list, 
             // but for completeness/preference we might include both or prefer javaw on windows.
             // Let's just add all valid ones and let UI handle or user pick.
        }
    }
}
