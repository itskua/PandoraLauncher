set -e

if [ -z "$1" ]; then
  echo "Missing version arg"
  exit 1
fi

version=${1#v}

echo "building release"
cargo build --release --target x86_64-pc-windows-msvc

EXE=target/x86_64-pc-windows-msvc/release/pandora_launcher.exe

echo "stripping exe (llvm)"
if command -v llvm-strip >/dev/null 2>&1; then
  llvm-strip "$EXE"
else
  echo "llvm-strip not found, skiping strip"
fi

mkdir -p dist

# copy exe (safer than mv on windows)
cp "$EXE" dist/PandoraLauncher-Windows.exe

cargo install cargo-packager || true

cargo packager --config '{'\
'  "name": "pandora-launcher",'\
'  "outDir": "./dist",'\
'  "productName": "Pandora Launcher",'\
'  "version": "'"$version"'",'\
'  "identifier": "com.moulberry.pandoralauncher",'\
'  "resources": [],'\
'  "formats": ["nsis"],'\
'  "binaries": [{ "path": "PandoraLauncher-Windows.exe", "main": true }],'\
'  "icons": ["package/windows.ico"]'\
'}'

mv dist/PandoraLauncher-Windows.exe \
   dist/PandoraLauncher-Windows-$version-x86_64.exe

SETUP_EXE=$(ls dist/*_x64-setup.exe | head -n 1)

if [ -z "$SETUP_EXE" ]; then
  echo "setup exe not found"
  exit 1
fi

mv "$SETUP_EXE" \
   dist/PandoraLauncher-Windows-${version}_x64-setup.exe
