# 05 — Spec du design

Référence visuelle : `docs/design/reference-forklift.png`. C'est une capture de ForkLift 4, gardée **uniquement comme inspiration locale**. Ne pas la redistribuer et ne pas la commiter dans un dépôt public. Satchel reprend la **structure** et la **qualité de finition**, pas l'identité visuelle : pas de nom, d'icône ni d'éléments de marque ForkLift. Les couleurs sont celles de **Catppuccin Mocha**, cohérentes avec le bureau de l'utilisateur (Plasma et kitty en Catppuccin Mocha, accent mauve).

Les tailles sont en **px logiques**, puis multipliées par le facteur d'échelle de l'écran. L'utilisateur a un écran à 1.0 et un écran 4K à 1.7.

---

## 1. Anatomie de la fenêtre

```
┌──────────────┬───────────────────────────────────────────────────────────────┬──────────────┐
│ ● ● ●        │ (‹ ›)  Titre      [▦ ☰ ▥] [☆ ⚯] [⟳ ⧉ ＋ 🗑]        [🔍 Search     ] │              │ ← barre unifiée (52)
│              ├───────────────────────────────┬───────────────────────────────┤  ⓘ  ⇅  ⚠    │
│ Devices      │ ⌂ david › Documents › Site    │▔▔▔▔▔▔▔▔ (liseré actif) ▔▔▔▔▔▔▔▔│              │
│  Disque  42G │   19 éléments, 42 Go libres   │ ⌂ david › Images              │ [ aperçu  ]  │ ← en-têtes de panneau (56)
│ Shares       ├───────────────────────────────┤   1 sur 9 sélectionné (1,5 Mo)│              │
│ Favorites    │ Nom          ˄   Taille  Date │                               │ Nom.jpg      │
│  ⌂ david     │ › ▸ admin   ●●●  9 Mo   …     │   [vignette] [vignette]       │ Image JPEG   │
│  …           │   ▫ index.php      405 o …    │                               │ General   ⌄  │
│ Tags         │                               │                               │ Permissions ⌄│
│  ● Orange    │                               │                               │              │
└──────────────┴───────────────────────────────┴───────────────────────────────┴──────────────┘
  barre latérale      panneau gauche (liste)        panneau droit (icônes)      panneau d'infos
  210 (180–320)        flex 1                         flex 1                     300 (260–420)
```

- **La barre latérale occupe toute la hauteur**, y compris la zone de titre. Les boutons de fenêtre sont dans son coin haut-gauche, comme sur macOS.
- **La barre d'outils est unifiée avec la barre de titre** : on n'utilise pas la décoration du serveur (SSD), on dessine nous-mêmes. On la saisit pour déplacer la fenêtre (`winit::Window::drag_window`), et un double-clic maximise.
- Le **panneau d'infos** occupe toute la hauteur à droite, sous la barre unifiée qui s'arrête à sa gauche (comme sur la référence). On l'ouvre et le ferme avec `F11` et un bouton.
- On peut **redimensionner** la barre latérale, le séparateur entre les panneaux et le panneau d'infos. Les largeurs sont persistées.
- **Coins de fenêtre** : 12 px de rayon quand la fenêtre n'est pas maximisée. Au moment de l'implémentation, vérifier que KWin accepte une fenêtre sans décoration avec transparence.

### Boutons de fenêtre (CSD)
- Trois pastilles de 12 px espacées de 8 px, à 18 px du bord gauche, centrées verticalement dans les 52 px de la barre.
- Couleurs : fermer `#f38ba8` (red), réduire `#f9e2af` (yellow), agrandir `#a6e3a1` (green). Inactives (fenêtre sans focus) : `#45475a` (surface1).
- Au survol du groupe, les glyphes ×, −, + apparaissent en `#11111b` à 60 %.
- **[Plus tard]** : une option pour suivre l'ordre des boutons configuré dans KDE (`kwinrc [org.kde.kdecoration2] ButtonsOnLeft/Right`).

---

## 2. Jetons de design

### Couleurs (Catppuccin Mocha)
| Jeton | Valeur | Usage |
|---|---|---|
| `bg.sidebar` | `#181825` (mantle), 92 % d'opacité | Barre latérale. On ajoute le flou KWin si on arrive à l'obtenir (voir §8). |
| `bg.window` | `#1e1e2e` (base) | Panneaux de fichiers. |
| `bg.toolbar` | `#1e1e2e` | Barre unifiée. |
| `bg.inspector` | `#181825` (mantle) | Panneau d'infos. |
| `bg.capsule` | `#313244` (surface0) | Groupes de boutons de la barre d'outils, champ de recherche. |
| `bg.capsule.hover` | `#45475a` (surface1) | |
| `bg.capsule.pressed` | `#585b70` (surface2) | |
| `bg.row.hover` | `#cdd6f4` à 4 % | Survol d'une ligne ou d'une tuile. |
| `bg.row.selected` | `#89b4fa` à 22 % | Sélection dans un panneau **actif**. |
| `bg.row.selected.inactive` | `#cdd6f4` à 8 % | Sélection dans le panneau inactif. |
| `bg.sidebar.selected` | `#cdd6f4` à 10 % | Élément courant de la barre latérale. |
| `label.selected.bg` | `#89b4fa` (blue) | « Pilule » autour du nom de la tuile sélectionnée (vue icônes). |
| `label.selected.fg` | `#11111b` (crust) | |
| `fg.primary` | `#cdd6f4` (text) | |
| `fg.secondary` | `#a6adc8` (subtext0) | Tailles, dates, sous-lignes d'en-tête, valeurs secondaires. |
| `fg.tertiary` | `#7f849c` (overlay1) | Titres de section de la barre latérale, en-têtes de colonnes. |
| `fg.disabled` | `#6c7086` (overlay0) | |
| `accent` | `#cba6f7` (mauve) | Liseré du panneau actif, icônes de la barre latérale, focus clavier, curseur d'édition. |
| `separator` | `#313244` (surface0) | Traits de 1 px. |
| `danger` | `#f38ba8` | Supprimer, erreurs. |
| `warning` | `#f9e2af` | |
| `success` | `#a6e3a1` | |

Les couleurs des tags suivent celles de Finder et ForkLift, transposées en Catppuccin :
Rouge `#f38ba8`, Orange `#fab387`, Jaune `#f9e2af`, Vert `#a6e3a1`, Bleu `#89b4fa`, Violet `#cba6f7`, Gris `#9399b2`.

**Règle :** toutes les couleurs passent par un `global Theme` en Slint. Prévoir dès le départ une **variante Latte** (claire), pour pouvoir proposer un thème clair et suivre le jeu de couleurs KDE plus tard.

### Typographie
- Famille : **Inter** si elle est installée (paquet `inter-font`), sinon **Noto Sans**. Pour les tailles et les dates, utiliser les chiffres tabulaires (`font-feature tnum`) si Slint le permet, sinon une police à chiffres alignés.

| Rôle | Taille | Graisse | Couleur |
|---|---|---|---|
| Titre de la barre d'outils (dossier actif) | 15 | 600 | `fg.primary` |
| Fil d'Ariane de l'en-tête | 13 | 500 | `fg.primary` (le dernier segment est aussi en 500) |
| Sous-ligne de l'en-tête | 11.5 | 400 | `fg.secondary` |
| Titre de section de la barre latérale | 11 | 600 | `fg.tertiary` |
| Élément de la barre latérale | 13 | 400 | `fg.primary` |
| En-tête de colonne | 11.5 | 600 | `fg.tertiary` (la colonne triée est en `fg.primary`) |
| Ligne, colonne nom | 13 | 400 | `fg.primary` |
| Ligne, colonnes taille et date | 12.5 | 400 | `fg.secondary` |
| Nom sous une tuile | 12.5 | 400 | `fg.primary` |
| Nom de fichier du panneau d'infos | 15 | 600 | `fg.primary` |
| Type et taille du panneau d'infos | 12 | 400 | `fg.secondary` |
| Titre de section du panneau d'infos | 13 | 600 | `fg.primary` |
| Clé / valeur du panneau d'infos | 12 / 12 | 400 / 400 | `fg.secondary` / `fg.primary` |

### Espacements et rayons
- Grille de 4 px. Marges internes des panneaux : 12 px. Barre latérale : 10 px horizontalement.
- Rayons : capsules 8, élément de la barre latérale 6, tuile 8, vignette 6, aperçu du panneau d'infos 10, pilule du nom 4, menus 10, fenêtre 12.
- Hauteurs : barre unifiée 52, en-tête de panneau 56, en-tête de colonnes 28, ligne 28, élément de la barre latérale 28, titre de section de la barre latérale 30 (avec 14 au-dessus), ligne clé/valeur du panneau d'infos 26.

### Icônes
- Barre d'outils et barre latérale : **icônes symboliques** au trait de 1,5 px, en 16 px pour la barre latérale et 18 px pour la barre d'outils. Source : le thème d'icônes freedesktop (Breeze `*-symbolic`), recolorées en `fg.primary`, ou `accent` pour la barre latérale. Si le rendu ne convient pas, embarquer un jeu SVG libre (Lucide, sous licence ISC).
- Fichiers : icônes MIME du thème de l'utilisateur (spec freedesktop Icon Theme), 16 px dans la vue liste et 64 à 128 px dans la vue icônes. Les dossiers sont en `accent` (bleu) si le thème n'impose rien.

---

## 3. Barre d'outils unifiée (52 px)
De gauche à droite, à partir de la fin de la barre latérale :
1. **Capsule navigation** : ‹ précédent, › suivant (32×32 chacun, dans une seule capsule). L'état désactivé est en `fg.disabled`.
2. **Titre** : nom du dossier du panneau actif, à 12 px de la capsule.
3. *(espace flexible)*
4. **Capsule vue** : icônes, liste, colonnes **[Plus tard]**. Le bouton du mode actif a un fond `bg.capsule.pressed`.
5. **Capsule favoris** : ☆ ajouter le dossier aux favoris, ⚯ se connecter à un serveur **[Plus tard]**.
6. **Capsule actions** : ⟳ synchroniser **[Plus tard]**, ⧉ copier vers l'autre panneau, ＋ nouveau dossier, 🗑 corbeille.
7. **Recherche** : champ capsule de 220 px (min. 160), icône loupe, texte indicatif « Rechercher ». Il filtre le panneau actif.
- Les capsules sont espacées de 10 px. Les boutons font 34×30 et les icônes 18 px.
- Info-bulle sur chaque bouton, avec le raccourci clavier.

## 4. Barre latérale
- Sections, dans l'ordre : **Appareils** (disques, avec l'espace libre aligné à droite en `fg.secondary`), **Réseau** (montages distants) **[v1]**, **Favoris** (compatible avec `user-places.xbel`, voir spec 04), **Tags** (pastille de 10 px et nom) **[v1]**.
- Titres de section en `fg.tertiary`, en casse normale (pas en majuscules). On peut replier une section en cliquant sur son titre, et le chevron n'apparaît qu'au survol.
- Élément : icône `accent` de 16 px, 8 px d'écart, libellé élidé à la fin. L'élément sélectionné (le dossier du panneau actif) a un fond `bg.sidebar.selected` arrondi à 6 px, avec 6 px de marge latérale.
- Cible de dépôt : au survol pendant un glisser-déposer, contour de 1,5 px en `accent`. Entre deux éléments, une ligne d'insertion signale un réordonnancement.

## 5. Panneaux de fichiers

### En-tête de panneau (56 px)
- Ligne 1 : icône du lieu racine (maison pour `~`, disque, serveur) en 22 px, puis le **fil d'Ariane**. Les segments sont séparés par un chevron `›` en `fg.tertiary`. Chaque segment est cliquable, il a un fond capsule au survol et accepte les dépôts.
- Ligne 2 : la **sous-ligne d'état**, qui remplace la barre d'état de Dolphin :
  - sans sélection : `19 éléments, 42,4 Go disponibles` ;
  - avec sélection : `1 sur 9 sélectionné (1,5 Mo), 42,4 Go disponibles` ;
  - avec un filtre : `7 sur 19 éléments correspondent à « php »`.
- Un clic sur le fil d'Ariane en dehors des segments, ou `Ctrl+L`, passe en **mode édition** : champ texte avec complétion. `Échap` annule.
- **Liseré du panneau actif** : barre de 3 px en `accent` en haut du panneau, arrondie à 1,5 px, avec 12 px de marge latérale. Le panneau inactif n'a pas de liseré, et sa sélection passe en `bg.row.selected.inactive`. Le passage d'un état à l'autre est un fondu de 120 ms.

### Vue liste (détails)
- En-tête de colonnes : Nom (flexible), Tags (pastilles, 44 px), Taille (alignée à droite, 80 px), Date de modification (130 px). Un clic trie et un deuxième clic inverse le tri. La flèche ˄˅ s'affiche à droite du titre de la colonne triée. Un clic droit ouvre le menu de choix des colonnes **[v1]**.
- Ligne de 28 px. Un triangle de dépliage `›` de 10 px, en `fg.tertiary`, s'affiche pour les dossiers, et tourne de 90° en 120 ms à l'ouverture. Indentation de 16 px par niveau. Icône de 16 px, puis le nom.
- **Pastilles de tags** : cercles de 10 px **qui se chevauchent** (décalage de 6 px, contour de 1,5 px couleur `bg.window` pour les séparer), 3 au maximum puis « +n ».
- Pas de lignes alternées. Ni grille ni séparateur entre les lignes.
- Taille : « 9 Mo », « 351 octets », unités SI par défaut (1 Mo = 1 000 000 octets, comme Dolphin et KDE). Les dossiers affichent le nombre d'éléments **[MVP]**, et leur taille récursive est une option **[Plus tard]**.
- Date : selon la locale. Format court `2023-08-29` ou relatif (« Aujourd'hui 15:06 ») **[v1]**.

### Vue icônes
- Tuile de 150×150 : vignette ou icône de 96 px centrée dans un carré de 112 px, nom sur 1 ligne élidée au milieu, sur 2 lignes au survol ou à la sélection **[v1]**.
- Les vignettes d'images ont un rayon de 6 et une ombre légère (0 1 3, noir à 35 %), en conservant leur proportion.
- **Sélection** : fond de tuile `bg.row.selected` arrondi à 8 autour de la vignette, et le nom passe dans une **pilule** `label.selected.bg`/`fg`, comme sur la référence.
- La grille se répartit sur la largeur, avec un écart minimal de 16 px.

### Vide, chargement, erreur
- Dossier vide : texte centré « Dossier vide » en `fg.tertiary`.
- Chargement de plus de 150 ms : petit indicateur dans la sous-ligne, sans écran bloquant.
- Erreur (permission refusée, dossier supprimé) : bande de message en haut du panneau, avec une action (« Remonter au parent »).

## 6. Panneau d'infos (inspecteur)
- **Onglets** en haut (icônes de 18 px centrées) : ⓘ Infos **[MVP]**, ⇅ Comparer ou synchroniser **[Plus tard]**, ⚠ Problèmes **[Plus tard]**. L'onglet actif est en `accent`.
- **Aperçu** : largeur du panneau moins 24, rapport 4:3 au maximum, rayon 10, `object-fit: contain` sur fond `bg.window`. Pour un dossier ou un fichier sans aperçu, grande icône MIME de 128 px.
- **Nom** (15/600, sélectionnable) et **type et taille** (« Image JPEG — 1,5 Mo »).
- **Section General** (repliable, chevron à droite) : Chemin, Taille (« 1,5 Mo (1 469 302 octets) »), Modifié, Créé (si le système de fichiers le fournit via statx), Accédé **[v1]**, Extension, Fichier caché (case). Les clés sont à gauche en `fg.secondary` et les valeurs à droite en `fg.primary`, élidées au milieu, avec la valeur complète en info-bulle. Séparateur de 1 px entre les lignes.
- **Section Permissions** (repliable) **[v1]** : grille de cases R, W, X × Propriétaire, Groupe, Autres. Champ **Octal** modifiable (`644`) à côté de la notation `rw-r--r--`. Listes déroulantes Propriétaire et Groupe : on ne peut les modifier que si on a le droit de le faire. Une modification s'applique immédiatement, avec une annulation possible. Avec plusieurs fichiers sélectionnés, une case est à l'état mixte si les fichiers diffèrent.
- **Sélection multiple** : « 5 éléments, 12,3 Mo » avec une mosaïque de 4 vignettes au maximum.

## 7. Mouvement
- Durées : 120 ms pour les micro-transitions (survol, liseré actif, chevrons), 180 ms pour l'ouverture et la fermeture des panneaux et le dépliage de sections. Courbe `ease-out`.
- Pas d'animation pendant le défilement ni lors d'un changement de dossier : l'affichage doit être immédiat, la réactivité passe avant tout.

## 8. Risques techniques à valider pendant le prototype
1. **CSD sous KWin avec Slint et winit** : fenêtre sans décoration, coins arrondis transparents, ombre, glisser et redimensionner par les bords. S'il y a un problème, se rabattre sur la décoration du serveur (SSD) avec une barre d'outils normale sous la barre de titre KDE.
2. **Flou derrière la barre latérale** : il faut le protocole Wayland `org_kde_kwin_blur`, que Slint et winit n'exposent pas. Pour le MVP, un fond opaque ou presque. Piste ensuite : un petit client Wayland qui applique le flou à la surface (accès à `wl_surface` via raw-window-handle).
3. **Rendu de 10k lignes et plus** : utiliser `ListView` de Slint, qui est virtualisée, et vérifier les FPS au défilement sur l'écran 4K à l'échelle 1.7.
4. **Icônes du thème système et vignettes** : résolution des icônes freedesktop en Rust (crate `freedesktop-icons`), et vignettes du cache partagé `~/.cache/thumbnails` (spec freedesktop Thumbnail), pour réutiliser celles que KDE a déjà générées.
