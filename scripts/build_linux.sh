set -e

if [ -z "$1" ]; then
    echo "Missing version argument"
    exit 1
fi

version=${1#v}

sudo apt-get update --yes && sudo apt-get install --yes libssl-dev libdbus-1-dev libx11-xcb1 libxkbcommon-x11-dev pkg-config inkscape
cargo build --release --target x86_64-unknown-linux-gnu
strip target/x86_64-unknown-linux-gnu/release/pandora_launcher
mkdir -p dist
mv target/x86_64-unknown-linux-gnu/release/pandora_launcher dist/PandoraLauncher-Linux

inkscape --export-filename="package/icon_512x512.png" --export-width=512 "package/windows.svg"

cargo install cargo-packager
cargo packager --config '{'\
'  "name": "pandora-launcher",'\
'  "outDir": "./dist",'\
'  "formats": ["deb", "appimage"],'\
'  "productName": "Pandora Launcher",'\
'  "version": "'"$version"'",'\
'  "identifier": "com.moulberry.pandoralauncher",'\
'  "resources": [],'\
'  "binaries": [{ "path": "PandoraLauncher-Linux", "main": true }],'\
'  "icons": ["package/icon_512x512.png"]'\
'}'

mv dist/PandoraLauncher-Linux dist/PandoraLauncher-Linux-$version-x86_64
mv dist/pandora-launcher_*_amd64.deb dist/PandoraLauncher-Linux-${version}_amd64.deb
mv dist/pandora-launcher_*_x86_64.AppImage dist/PandoraLauncher-Linux-${version}_x86_64.AppImage
