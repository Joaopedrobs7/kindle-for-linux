#!/bin/sh
set -eu

REPO="Joaopedrobs7/kindle-for-linux"
LATEST_API="https://api.github.com/repos/${REPO}/releases/latest"
FALLBACK_DEB_URL="https://github.com/${REPO}/releases/download/v2.0.2/kindle-para-linux_2.0.2_amd64.deb"
APT_REPO="https://joaopedrobs7.github.io/kindle-for-linux"
LIST_PATH="/etc/apt/sources.list.d/kindle-linux.list"
LIST_LINE="deb [trusted=yes] ${APT_REPO} ./"
RELEASE_JSON=""

if [ "$(id -u)" -eq 0 ]; then
  SUDO=""
else
  if ! command -v sudo >/dev/null 2>&1; then
    echo "erro: sudo nao encontrado" >&2
    exit 1
  fi
  SUDO="sudo"
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "erro: curl nao encontrado" >&2
  exit 1
fi

download() {
  url=$1
  dest=$2
  echo "==> Baixando kindle-para-linux..."
  if [ -e "$dest" ] && [ ! -w "$dest" ]; then
    $SUDO rm -f "$dest"
  fi
  curl -fsSL --retry 3 --retry-delay 2 --max-time 300 -o "$dest" "$url"
}

load_release() {
  RELEASE_JSON=$(curl -fsSL --retry 2 --retry-delay 1 --max-time 30 -H "Accept: application/vnd.github+json" -H "User-Agent: kindle-for-linux-install" "$LATEST_API" 2>/dev/null || true)
}

pick_url() {
  ext=$1
  prefer=$2
  chosen=""
  urls=$(printf '%s\n' "$RELEASE_JSON" | sed -n 's/.*"browser_download_url": *"\([^"]*\)".*/\1/p')
  for url in $urls; do
    case $url in
      *"$ext")
        case $url in
          *"$prefer"*)
            chosen=$url
            break
            ;;
        esac
        if [ -z "$chosen" ]; then
          chosen=$url
        fi
        ;;
    esac
  done
  printf '%s\n' "$chosen"
}

apt_repo_reachable() {
  body=$(curl -fsSL --retry 2 --retry-delay 1 --max-time 25 "${APT_REPO}/Release" 2>/dev/null || true)
  case "$body" in
    *SHA256:*|*MD5Sum:*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

configure_apt_repo() {
  if apt_repo_reachable; then
    printf '%s\n' "$LIST_LINE" | $SUDO tee "$LIST_PATH" >/dev/null
    $SUDO env DEBIAN_FRONTEND=noninteractive apt-get update < /dev/null || true
  fi
}

install_linux() {
  if ! command -v apt-get >/dev/null 2>&1; then
    echo "erro: apt-get nao encontrado" >&2
    exit 1
  fi

  arch=$(dpkg --print-architecture 2>/dev/null || true)
  if [ -z "$arch" ]; then
    case "$(uname -m)" in
      x86_64) arch=amd64 ;;
      *) arch=$(uname -m) ;;
    esac
  fi
  if [ "$arch" != "amd64" ]; then
    echo "erro: pacote amd64, arquitetura ${arch}" >&2
    exit 1
  fi

  load_release
  DEB_URL=$(pick_url ".deb" "amd64")
  if [ -z "$DEB_URL" ]; then
    DEB_URL=$FALLBACK_DEB_URL
  fi
  case $DEB_URL in
    */kindle-para-linux_2.0.2_amd64.deb)
      DEB_PATH=/tmp/kindle-para-linux_2.0.2_amd64.deb
      ;;
    *)
      DEB_PATH=/tmp/kindle-para-linux.deb
      ;;
  esac

  download "$DEB_URL" "$DEB_PATH"

  $SUDO DEBIAN_FRONTEND=noninteractive apt-get update < /dev/null \
    && $SUDO DEBIAN_FRONTEND=noninteractive apt-get install -y --reinstall "$DEB_PATH" < /dev/null

  configure_apt_repo
  echo "==> Instalado! Rode 'kindle-linux' ou abra pelo menu de apps."
}

copy_app_bundle() {
  app=$1
  dest="/Applications/$(basename "$app")"
  $SUDO rm -rf "$dest"
  $SUDO cp -R "$app" "/Applications/"
  if command -v xattr >/dev/null 2>&1; then
    $SUDO xattr -dr com.apple.quarantine "$dest" 2>/dev/null || true
  fi
  printf '%s\n' "$dest"
}

install_dmg() {
  dmg=$1
  mnt=$(mktemp -d)
  hdiutil attach -nobrowse -readonly -mountpoint "$mnt" "$dmg"
  app=$(find "$mnt" -name '*.app' -print | head -n 1)
  if [ -z "$app" ]; then
    hdiutil detach "$mnt" >/dev/null 2>&1 || true
    echo "erro: dmg sem .app" >&2
    exit 1
  fi
  copy_app_bundle "$app" >/dev/null
  hdiutil detach "$mnt" >/dev/null 2>&1 || true
}

install_app_archive() {
  archive=$1
  work=$(mktemp -d)
  tar -xzf "$archive" -C "$work"
  app=$(find "$work" -name '*.app' -print | head -n 1)
  if [ -z "$app" ]; then
    echo "erro: arquivo sem .app" >&2
    exit 1
  fi
  copy_app_bundle "$app" >/dev/null
}

build_macos_app() {
  echo "==> Compilando..."
  missing=""
  for cmd in git npm cargo rustc; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
      missing="${missing} ${cmd}"
    fi
  done
  if [ -n "$missing" ]; then
    echo "erro: faltam${missing}" >&2
    exit 1
  fi
  work=$(mktemp -d)
  git clone --depth 1 "https://github.com/${REPO}.git" "$work/src"
  (cd "$work/src" && npm install && npx tauri build --bundles app)
  app=$(find "$work/src/src-tauri/target/release/bundle/macos" -name '*.app' -print | head -n 1)
  if [ -z "$app" ]; then
    echo "erro: build sem .app" >&2
    exit 1
  fi
  copy_app_bundle "$app" >/dev/null
}

macos_done() {
  echo "==> Instalado! Abra pelo Launchpad."
}

install_macos() {
  case "$(uname -m)" in
    arm64|aarch64) prefer=aarch64 ;;
    x86_64) prefer=x64 ;;
    *) prefer=dmg ;;
  esac

  load_release
  dmg_url=$(pick_url ".dmg" "$prefer")
  archive_url=$(pick_url ".app.tar.gz" "$prefer")
  if [ -z "$archive_url" ]; then
    archive_url=$(pick_url ".tar.gz" "app")
  fi

  if [ -n "$dmg_url" ]; then
    download "$dmg_url" /tmp/kindle-para-linux.dmg
    install_dmg /tmp/kindle-para-linux.dmg
    macos_done
    return 0
  fi

  if [ -n "$archive_url" ]; then
    download "$archive_url" /tmp/kindle-para-linux.app.tar.gz
    install_app_archive /tmp/kindle-para-linux.app.tar.gz
    macos_done
    return 0
  fi

  build_macos_app
  macos_done
}

os=$(uname -s)
case "$os" in
  Linux)
    install_linux
    ;;
  Darwin)
    install_macos
    ;;
  *)
    echo "erro: use install.ps1 no Windows" >&2
    exit 1
    ;;
esac
