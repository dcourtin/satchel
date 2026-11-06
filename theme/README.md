# Thème de bureau Satchel

C'est le prolongement du design de Satchel (`docs/spec/05-design.md`) à tout Plasma : Catppuccin Mocha avec un accent mauve, des boutons de fenêtre façon feux tricolores et des widgets épurés.

| Pièce | Fichier | Destination |
|---|---|---|
| Jeu de couleurs | `color-schemes/Satchel.colors` | `~/.local/share/color-schemes/` |
| Décoration de fenêtre (Klassy) | `klassy/klassyrc` | `~/.config/klassy/` |
| Style des widgets (Kvantum) | `kvantum/Satchel/` | `~/.config/Kvantum/` |
| Icônes de fenêtre (le reste vient de Breeze) | `icons/Satchel/` | `~/.local/share/icons/` |
| Thème global | `look-and-feel/org.satchel.desktop/` | `~/.local/share/plasma/look-and-feel/` |

Installation : `./install.sh`, ou `./install.sh --apply` pour l'appliquer aussitôt. Il faut avoir installé `klassy` (AUR) et `kvantum`.

## Notes et pièges
- **Pourquoi Klassy et pas Aurorae :** dans Aurorae, la largeur du coin du cadre SVG est égale à celle de la bordure. Arrondir le haut de la fenêtre de 10 px demanderait donc des bordures de 10 px.
- **Couleurs des boutons Klassy :** elles sont imposées par `ButtonOverrideColors{Active,Inactive}{Close,Minimize,Maximize}`. Le format est un objet JSON état → `[R,G,B]` ou `[opacité %,R,G,B]` (voir `decorationbuttoncolors.cpp`, Klassy 6.7.3). **Klassy ne recalcule ces couleurs qu'au changement de jeu de couleurs** : il faut basculer sur un autre jeu de couleurs puis revenir.
- **Taille des boutons :** `ButtonIconStyle=StyleSystemIconTheme` avec `SystemIconSize=SystemIcon8` et `ScaleBackgroundPercent=150` donnent des pastilles de 12 px, comme dans Satchel. L'écart est réglé par `ButtonSpacingLeft=5`.
- **Thème d'icônes :** `index.theme` doit contenir `FollowsColorScheme=true`. Sans cette ligne, les icônes héritées de Breeze ne sont pas recolorées et deviennent illisibles sur fond sombre.
- **Kvantum** dérive du thème officiel Catppuccin Mocha Mauve (MIT, voir `kvantum/Satchel/LICENSE-catppuccin`). Les changements : sélection bleu `#89b4fa` à 22 %, fenêtres en mantle et listes en base, pas de rectangle de focus ni de traits d'arborescence.
