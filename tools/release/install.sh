set -e

if ! command -v unzip >/dev/null; then
  echo "Error: unzip is required to install 3em" 1>&2
  exit 1
fi

if [ "$OS" = "Windows_NT" ]; then
  target="x86_64-pc-windows-msvc"
else
  case $(uname -sm) in
  "Darwin x86_64") target="x86_64-apple-darwin" ;;
  *) target="x86_64-unknown-linux-gnu" ;;
  esac
fi

if [ $# -eq 0 ]; then
  zip_url="https://github.com/three-em/3em/releases/latest/download/three_em-${target}.zip"
else
  zip_url="https://github.com/three-em/3em/releases/download/${1}/three_em-${target}.zip"
fi

install_dir="${THREE_EM_INSTALL:-$HOME/.3em}"
bin_dir="$install_dir/bin"
exe="$bin_dir/three_em"

if [ ! -d "$bin_dir" ]; then
  mkdir -p "$bin_dir"
fi

curl --fail --location --progress-bar --output "$exe.zip" "$zip_url"
unzip -d "$bin_dir" -o "$exe.zip"
chmod +x "$exe"
rm "$exe.zip"

echo "3em was installed successfully to $exe"
if command -v three_em >/dev/null; then
  echo "Run 'three_em --help' to get started"
else
  case $SHELL in
  /bin/zsh) shell_profile=".zshrc" ;;
  *) shell_profile=".bash_profile" ;;
  esac
  echo "Manually add the directory to your \$HOME/$shell_profile (or similar)"
  echo "  export 3EM_INSTALL=\"$install_dir\""
  echo "  export PATH=\"\$3EM_INSTALL/bin:\$PATH\""
  echo "Run '$exe --help' to get started"
fi
