# Thème de bureau Satchel

## Principe : une seule source de vérité
**Tout dérive du jeu de couleurs KDE (« Satchel », basé sur Catppuccin Mocha) et de la couleur d'accent**, réglés dans *Configuration du système → Couleurs et thèmes*. Aucune pièce ne contient de couleur en dur : si tu changes le jeu de couleurs ou l'accent, tout suit, **y compris l'appli Satchel**, qui relit `kdeglobals` en direct.

| Pièce | Fichier | Comment elle suit KDE |
|---|---|---|
| Jeu de couleurs | `color-schemes/Satchel.colors` | C'est la source |
| Décoration de fenêtre (Klassy) | `klassy/klassyrc` | Les pastilles référencent le jeu de couleurs : `["NegativeText"]`, `["NeutralText"]`, `["PositiveText"]`. Fenêtre inactive : `["TitleBarTextInactive",30]` |
| Style des widgets (Klassy) | `klassy/klassyrc` `[Style]` | Style dérivé de Breeze : il utilise la palette KDE (arrondis de 8 px, barres de défilement fines) |
| Icônes | `icons/Satchel/` (hérite de Breeze) | Les emplacements et les disques utilisent `ColorScheme-Highlight`, recolorée par Plasma. Les pictos de fenêtre sont recolorés par Klassy |
| Applis GTK et Firefox | généré par `kde-gtk-config` | Le thème Breeze GTK lit `~/.config/gtk-3.0/colors.css`, que KDE génère depuis le jeu de couleurs. Firefox (Flatpak) y a accès en lecture et utilise la barre de titre de KWin (`firefox/user.js`) |
| Satchel | `crates/satchel-core/src/kde.rs` | Palette lue dans `kdeglobals` ; icônes Breeze recolorées comme le fait KIconLoader (`ColorScheme-Text`, `-Accent`, `-Highlight`…) |
| Thème global | `look-and-feel/org.satchel.desktop/` | Regroupe tout le reste et s'applique d'un clic |

Installation : `./install.sh`, ou `./install.sh --apply` pour l'appliquer aussitôt. Il faut avoir installé `klassy` (AUR). Pour Firefox, copier `firefox/user.js` dans le profil, puis choisir le thème « Système – automatique ».

## Ce qui a été essayé puis abandonné (et pourquoi)
- **Kvantum** : il impose sa propre palette à chaque appli et ignore le jeu de couleurs KDE. Il a été retiré parce qu'il cassait la cohérence (par exemple, des icônes recolorées dans le bleu de sa sélection au lieu de l'accent).
- **`userChrome.css` pour Firefox** : les couleurs y étaient en dur et sans lien avec KDE. Il est remplacé par la barre de titre de KWin.
- **Aurorae** : dans un cadre SVG, la largeur du coin est égale à celle de la bordure. Arrondir de 10 px demanderait donc des bordures de 10 px.
- **Codes couleur en dur dans Klassy et dans les icônes** : remplacés par des références au jeu de couleurs.

## Pièges
- **Klassy ne recalcule les couleurs de ses boutons qu'au changement de jeu de couleurs** : il faut basculer sur un autre jeu de couleurs puis revenir. `install.sh --apply` le fait.
- **Format des couleurs imposées dans Klassy** (`decorationbuttoncolors.cpp`, 6.7.3) : un objet JSON état → `["NomDeCouleur"]`, `["NomDeCouleur", opacité %]`, `[R,G,B]` ou `[opacité %,R,G,B]`.
- **Taille des boutons :** `ButtonIconStyle=StyleSystemIconTheme`, `SystemIconSize=SystemIcon8` et `ScaleBackgroundPercent=150` donnent des pastilles de 12 px. L'écart est réglé par `ButtonSpacingLeft=5`.
- **`index.theme` doit contenir `FollowsColorScheme=true`.** Sans cette ligne, les icônes héritées ne sont pas recolorées.
- **Dolphin :** la barre latérale garde ses réglages par défaut. Les sections masquées se règlent par clic droit et sont stockées dans `user-places.xbel`, partagé par KDE.
