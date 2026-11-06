#!/bin/bash
# Installe le thème de bureau « Satchel » pour l'utilisateur courant (sans sudo).
# Prérequis : paquets klassy (AUR) et kvantum (dépôts officiels).
#   ./install.sh          copie les fichiers
#   ./install.sh --apply  copie puis applique le thème global
set -euo pipefail
cd "$(dirname "$0")"

DATA=${XDG_DATA_HOME:-$HOME/.local/share}
CONF=${XDG_CONFIG_HOME:-$HOME/.config}

backup() { [ -e "$1" ] && cp -a "$1" "$1.avant-satchel.$(date +%Y%m%d%H%M%S)" || true; }

install -Dm644 color-schemes/Satchel.colors "$DATA/color-schemes/Satchel.colors"
mkdir -p "$DATA/icons" "$CONF/Kvantum" "$CONF/klassy" "$DATA/plasma/look-and-feel"
rm -rf "$DATA/icons/Satchel" && cp -r icons/Satchel "$DATA/icons/"
rm -rf "$CONF/Kvantum/Satchel" && cp -r kvantum/Satchel "$CONF/Kvantum/"
backup "$CONF/klassy/klassyrc" && install -m644 klassy/klassyrc "$CONF/klassy/klassyrc"
rm -rf "$DATA/plasma/look-and-feel/org.satchel.desktop" && cp -r look-and-feel/org.satchel.desktop "$DATA/plasma/look-and-feel/"

# Kvantum choisit son thème dans son propre fichier.
kwriteconfig6 --file "$CONF/Kvantum/kvantum.kvconfig" --group General --key theme Satchel
# La barre d'état pleine largeur de Dolphin remplace la sous-ligne de Satchel.
kwriteconfig6 --file dolphinrc --group General --key ShowStatusBar FullWidth
# Barre latérale de Dolphin façon Satchel : icônes 16 px (les sections Récent/Distant se masquent
# par clic droit, réglage stocké dans user-places.xbel partagé par KDE : non modifié ici).
kwriteconfig6 --file dolphinrc --group PlacesPanel --key IconSize 16

echo "Thème Satchel installé."
if [ "${1:-}" = "--apply" ]; then
    plasma-apply-lookandfeel -a org.satchel.desktop
    # Klassy ne recalcule les couleurs des boutons qu'au changement de jeu de couleurs.
    plasma-apply-colorscheme BreezeDark >/dev/null && plasma-apply-colorscheme Satchel >/dev/null
    qdbus6 org.kde.KWin /KWin reconfigure
    echo "Thème Satchel appliqué. Relance les applis ouvertes pour le style Kvantum."
fi
