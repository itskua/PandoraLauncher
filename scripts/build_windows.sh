set -e

cargo build --release --target x86_64-pc-windows-msvc
strip target/x86_64-pc-windows-msvc/release/pandora_launcher.exe
mv target/x86_64-pc-windows-msvc/release/pandora_launcher dist/PandoraLauncher-Windows-x86_64.exe
