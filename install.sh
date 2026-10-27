#!/usr/bin/env bash
# Instala o Kindle para Linux e, se o repositório APT no GitHub Pages
# responder, deixa o apt pronto para as próximas atualizações.
set -euo pipefail

DEB_URL="https://github.com/Joaopedrobs7/kindle-for-linux/releases/download/v2.0.0/kindle-para-linux_2.0.0_amd64.deb"
DEB_PATH="/tmp/kindle-para-linux_2.0.0_amd64.deb"
APT_REPO="https://joaopedrobs7.github.io/kindle-for-linux"
LIST_PATH="/etc/apt/sources.list.d/kindle-linux.list"
LIST_LINE="deb [trusted=yes] ${APT_REPO} ./"

if [[ "$(id -u)" -ne 0 ]] && ! command -v sudo >/dev/null 2>&1; then
  echo "Execute este instalador como root, ou instale sudo." >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl é necessário para baixar o pacote." >&2
  exit 1
fi

if ! command -v apt-get >/dev/null 2>&1; then
  echo "Este instalador precisa do apt-get (Ubuntu ou Debian)." >&2
  exit 1
fi

arch="$(dpkg --print-architecture 2>/dev/null || true)"
if [[ -z "${arch}" ]]; then
  case "$(uname -m)" in
    x86_64) arch="amd64" ;;
    *) arch="$(uname -m)" ;;
  esac
fi
if [[ "${arch}" != "amd64" ]]; then
  echo "O pacote publicado é amd64. Arquitetura atual: ${arch}." >&2
  exit 1
fi

run_root() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

apt_repo_reachable() {
  local body
  body="$(curl -fsSL --retry 2 --retry-delay 1 --max-time 25 "${APT_REPO}/Release" 2>/dev/null || true)"
  [[ "${body}" == *"SHA256:"* || "${body}" == *"MD5Sum:"* ]]
}

echo "Baixando ${DEB_URL}"
curl -fsSL --retry 3 --retry-delay 2 --max-time 180 -o "${DEB_PATH}" "${DEB_URL}"

# O stdin fica em /dev/null porque o instalador pode ser recebido por um pipe.
if [[ "$(id -u)" -eq 0 ]]; then
  DEBIAN_FRONTEND=noninteractive apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y /tmp/kindle-para-linux_2.0.0_amd64.deb
else
  sudo DEBIAN_FRONTEND=noninteractive apt-get update && sudo DEBIAN_FRONTEND=noninteractive apt-get install -y /tmp/kindle-para-linux_2.0.0_amd64.deb
fi < /dev/null

if apt_repo_reachable; then
  printf '%s\n' "${LIST_LINE}" | run_root tee "${LIST_PATH}" >/dev/null
  if ! run_root env DEBIAN_FRONTEND=noninteractive apt-get update < /dev/null; then
    echo "O aplicativo foi instalado, mas a atualização da lista APT falhou." >&2
  fi
else
  echo "Repositório APT ainda indisponível; o pacote instalado continua válido." >&2
fi

cat <<'EOF'

Kindle para Linux está instalado.
Ele aparece no menu de aplicações e também pode ser aberto com o comando kindle-linux.
EOF
