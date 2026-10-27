#!/bin/sh
# Instala o Kindle para Linux. Rode com sh, sem sudo na frente:
#   curl -fsSL https://raw.githubusercontent.com/Joaopedrobs7/kindle-for-linux/main/install.sh | sh
#   curl -fsSL https://joaopedrobs7.github.io/kindle-for-linux/install.sh | sh
# O script chama sudo só quando não está rodando como root.
set -eu

DEB_URL="https://github.com/Joaopedrobs7/kindle-for-linux/releases/download/v2.0.0/kindle-para-linux_2.0.0_amd64.deb"
DEB_PATH="/tmp/kindle-para-linux_2.0.0_amd64.deb"
APT_REPO="https://joaopedrobs7.github.io/kindle-for-linux"
LIST_PATH="/etc/apt/sources.list.d/kindle-linux.list"
LIST_LINE="deb [trusted=yes] ${APT_REPO} ./"

if [ "$(id -u)" -eq 0 ]; then
  SUDO=""
else
  if ! command -v sudo >/dev/null 2>&1; then
    echo "Execute este instalador como root, ou instale sudo." >&2
    exit 1
  fi
  SUDO="sudo"
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl é necessário para baixar o pacote." >&2
  exit 1
fi

if ! command -v apt-get >/dev/null 2>&1; then
  echo "Este instalador precisa do apt-get (Ubuntu ou Debian)." >&2
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
  echo "O pacote publicado é amd64. Arquitetura atual: ${arch}." >&2
  exit 1
fi

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

echo "Baixando ${DEB_URL}"
if [ -e "${DEB_PATH}" ] && [ ! -w "${DEB_PATH}" ]; then
  $SUDO rm -f "${DEB_PATH}"
fi
curl -fsSL --retry 3 --retry-delay 2 --max-time 180 -o "${DEB_PATH}" "${DEB_URL}"

# stdin vem de /dev/null porque o instalador pode ser recebido por curl | sh.
$SUDO DEBIAN_FRONTEND=noninteractive apt-get update < /dev/null \
  && $SUDO DEBIAN_FRONTEND=noninteractive apt-get install -y /tmp/kindle-para-linux_2.0.0_amd64.deb < /dev/null

if apt_repo_reachable; then
  printf '%s\n' "$LIST_LINE" | $SUDO tee "$LIST_PATH" >/dev/null
  if ! $SUDO env DEBIAN_FRONTEND=noninteractive apt-get update < /dev/null; then
    echo "O aplicativo foi instalado, mas a atualização da lista APT falhou." >&2
  fi
else
  echo "Repositório APT ainda indisponível; o pacote instalado continua válido." >&2
fi

cat <<'EOF'

Kindle para Linux está instalado.
Ele aparece no menu de aplicações e também pode ser aberto com o comando kindle-linux.
EOF
