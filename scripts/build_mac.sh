set -e

if [ -z "$1" ]; then
    echo "Missing version argument"
    exit 1
fi

version=${1#v}

cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

strip target/aarch64-apple-darwin/release/pandora_launcher
strip target/x86_64-apple-darwin/release/pandora_launcher

mkdir -p dist

lipo -create -output dist/LuminaForgeLauncher-macOS target/x86_64-apple-darwin/release/pandora_launcher target/aarch64-apple-darwin/release/pandora_launcher

cargo install cargo-packager
cargo packager --config '{'\
'  "name": "lumina-forge-launcher",'\
'  "outDir": "./dist",'\
'  "formats": ["dmg", "app"],'\
'  "productName": "LuminaForge Launcher",'\
'  "version": "'"$version"'",'\
'  "identifier": "com.moulberry.luminaforgelauncher",'\
'  "resources": [],'\
'  "binaries": [{ "path": "LuminaForgeLauncher-macOS", "main": true }],'\
'  "icons": ["package/mac.icns"]'\
'}'

mv dist/LuminaForgeLauncher-macOS dist/LuminaForgeLauncher-macOS-$version-Universal
mv dist/lumina-forge-launcher_*_universal.dmg dist/LuminaForgeLauncher-macOS-${version}-Universal.dmg
tar -czf dist/LuminaForge.Launcher.app.tar.gz -C dist/ "LuminaForge Launcher.app"


