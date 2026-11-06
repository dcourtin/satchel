# 01 — Vues d'éléments : livre de recettes comportemental

Source analysée : Dolphin 26.08.3, `src/kitemviews/` (et `src/views/`, `src/settings/` pour les réglages par défaut).
Les références `fichier.cpp:ligne` sont relatives à `dolphin-26.08.3/src/`. Ce document décrit le **comportement observable** ; il ne décrit pas l'architecture C++.

Priorités : **[MVP]** = gestionnaire local à deux panneaux utilisable au quotidien ; **[v1]** = première version complète ; **[Plus tard]** = confort ou cas rares.

Vocabulaire :
- **élément courant** : l'élément qui a le focus clavier (cadre de focus). Il peut ne pas être sélectionné.
- **ancre** : le point de départ d'une sélection par plage (Maj).
- **sélection ancrée** : plage dynamique `[ancre..courant]` ajoutée à l'ensemble des éléments sélectionnés « figés ». La sélection visible est l'union des deux (`kitemviews/kitemlistselectionmanager.cpp:52-101`).
- **zone cœur** : le rectangle icône + texte d'un élément. **Zone pleine** : la ligne entière en vue détails si « surligner toute la ligne » est actif.

---

## 1. Modèle de sélection

### 1.1 Structure de la sélection [MVP]
- La sélection est un ensemble d'index, plus éventuellement une plage ancrée active. Quand une plage ancrée se termine (Ctrl, clic simple, bascule…), elle est « figée » dans l'ensemble (`kitemlistselectionmanager.cpp:178-200`).
- Toute opération `setSelected(index, n, Select|Deselect|Toggle)` termine d'abord la sélection ancrée en cours (`kitemlistselectionmanager.cpp:104-148`).
- Quand des éléments sont insérés ou supprimés, la sélection, l'élément courant et l'ancre sont **décalés** pour suivre les mêmes fichiers. Si l'ancre est supprimée, elle est recalée (`kitemlistselectionmanager.cpp:212-320`). Après un tri, les index sélectionnés suivent les fichiers déplacés (`:323-350`).
- Le signal « sélection changée » envoyé vers l'extérieur (panneau d'infos, barre d'état) est **anti-rebond de 300 ms**. Il passe à 0 ms quand on passe de « rien » à « quelque chose », ou l'inverse (`views/dolphinview.cpp:119-122`, `:1671`).

> Notes pour Rust/Slint : représenter la sélection par `BTreeSet<usize>` (ou une liste de plages) + `Option<(anchor, current)>`. Pour 100k éléments, Tout sélectionner doit produire une plage `[0..n)` et non 100k insertions : Dolphin s'en sort avec `KItemSet`, un ensemble de plages (`kitemviews/kitemset.cpp`). Copier cette idée.

### 1.2 Clic gauche simple [MVP]
Référence : `kitemlistcontroller.cpp:1621-1803` (appui), `:1805-1896` (relâchement).
- **Appui sur un élément non sélectionné, sans modificateur** : la sélection est vidée, l'élément devient courant, sélectionné et ancre.
- **Appui sur un élément déjà sélectionné, sans modificateur** : on ne vide **pas** la sélection à l'appui, car l'utilisateur veut peut-être glisser toute la sélection. Au **relâchement**, si aucun glisser n'a démarré, la sélection est réduite à cet élément (`:1708-1715`, `:1857-1863`).
- **Appui sur la zone vide d'une ligne** (hors icône+texte, vue détails) :
  - s'il y a une multi-sélection, elle est vidée et rien n'est sélectionné. Un lasso démarre si on glisse (`:1675-1704`) ;
  - s'il n'y a rien de sélectionné, un **lasso démarre** depuis la ligne (`:1746-1749`) ;
  - au relâchement sans déplacement (moins que la distance de glisser), le clic sélectionne l'élément de la ligne (`:1835-1849`).
- **Appui sur le fond de la vue** : la sélection est vidée (sans modificateur) et un lasso démarre (`:609-612`).
- **Activation** (ouvrir) : au double-clic par défaut (`:707-747`). En mode clic simple (réglage système KDE `SH_ItemView_ActivateItemOnSingleClick`), l'activation se fait au relâchement, sauf si le relâchement termine un lasso (`:1876-1883`). Avec Maj ou Ctrl, le clic ne fait que modifier la sélection, il n'active jamais (`:1871-1874`).
- **Double-clic sur le fond** : déclenche une action configurable, « Tout sélectionner » par défaut (`settings/dolphin_generalsettings.kcfg:18-20`). [v1]
- **Boutons Précédent/Suivant de la souris** : ils ne sélectionnent jamais rien et naviguent dans l'historique. Un double-clic sur ces boutons compte comme deux appuis (`:1625-1629`, `:712-719`).
- **Clic du milieu** sur un élément : signal « ouvrir dans un nouvel onglet » (`:1885-1887`). [v1]

### 1.3 Ctrl+clic [MVP]
- Ctrl+appui sur la zone cœur d'un élément : **bascule** sa sélection à l'appui, l'élément devient courant et nouvelle ancre (`:1773-1785`). Si on glisse ensuite, on démarre un glisser en mode copie.
- Ctrl+appui sur la zone vide d'une ligne : démarre un lasso au lieu de basculer (`:1778-1779`).
- Ctrl+clic n'active jamais l'élément.

### 1.4 Maj+clic [MVP]
- Maj+appui : l'ancre **ne bouge pas**, l'élément cliqué devient courant. La sélection = éléments déjà figés ∪ `[ancre..cliqué]` (`:1720-1724`, `:1788-1791`).
- **Particularité** : les éléments sélectionnés avant (par Ctrl+clic, par exemple) **restent sélectionnés**, parce que Maj empêche le vidage (`:1671`). Une série de Maj+clics successifs redimensionne la même plage dynamique.
- S'il n'y a pas d'ancre active, Maj+clic sélectionne l'élément et en fait l'ancre.
- Ctrl+Maj+clic : même comportement, la plage s'ajoute à l'existant.

### 1.5 Clic droit [MVP]
- Clic droit sur un élément non sélectionné : la sélection est remplacée par cet élément, puis le menu de l'élément s'ouvre. Sur un élément déjà sélectionné : la multi-sélection est conservée (`:1671`, `:749-800`).
- Clic droit dans la zone vide d'une ligne (sans « toute la ligne ») : rien n'est sélectionné, on obtient le menu de la vue (`:1752-1756`).
- Clic droit sur le fond : pas de lasso. Un lasso en cours est annulé (`:1725-1739`).
- Avant d'ouvrir le menu de la vue, tous les survols sont retirés, pour qu'aucune ligne ne semble visée (`:787-798`).
- **Touche Menu** : si au moins un élément est sélectionné, le menu s'ouvre au coin bas-droit de l'élément courant (s'il est sélectionné) ou du premier sélectionné. Sinon, on obtient le menu de la vue (`:751-770`).
- Clic droit sur l'en-tête de colonnes : menu des colonnes (`:775-778`).

### 1.6 Sélection au lasso [MVP]
Référence : `kitemlistcontroller.cpp:1344-1421`, `:1898-1916`, `kitemlistview.cpp:1643-1668`, `:1717-1773`, `:2853-2877`.
- Le lasso démarre sur un appui qui ne vise pas un élément (fond, ou zone vide de ligne quand c'est permis). Il n'existe qu'en mode multi-sélection.
- À chaque mouvement, **l'élément sous le pointeur devient courant et ancre** (bug KDE 163451, `:658-666`).
- Les éléments sélectionnés sont ceux dont la **zone pleine** coupe le rectangle du lasso.
- Modificateurs, lus en continu pendant le geste :
  - aucun : le lasso remplace la sélection précédente ;
  - **Maj** : union avec la sélection d'avant le lasso ;
  - **Ctrl** : **XOR** avec la sélection d'avant (les éléments du lasso basculent) (`:1412-1420`).
- Le lasso suit le défilement : son point d'arrivée est corrigé du décalage de défilement (`:1320-1342`).
- Le calcul ne parcourt pas tout le modèle. Il teste les éléments visibles, puis avance index par index hors de l'écran dans la direction du lasso, jusqu'à dépasser le rectangle (`:1393-1410`). Le coût reste donc borné même avec 100k éléments.
- Si le lasso fait moins que la distance de glisser au relâchement, c'est un clic (`:1835`).
- **Défilement automatique** pendant le lasso ou un glisser [MVP] :
  - marge active de **64 px** sur chaque bord ;
  - premier déclenchement après **700 ms** dans la marge, puis répétition à **60 Hz** (16 ms) ;
  - vitesse : `±(4 + d²/96)` px par tick, où `d` est la profondeur dans la marge, plafonnée à **128 px**. L'accélération est limitée à **+1 px par tick**, pour une montée progressive ;
  - pour un lasso, le défilement ne démarre que si le lasso fait déjà au moins **4 px** et va dans le même sens que le défilement. Cela évite de défiler quand on commence un lasso près d'un bord.
- Au relâchement, le rectangle du lasso **s'efface en fondu de 150 ms** (courbe de Bézier (0.4,0)→(1,1)) (`kitemlistview.cpp:46`, `:1650-1661`). [v1]

### 1.7 Tout sélectionner, inverser, désélectionner [MVP]
- Tout sélectionner (Ctrl+A) : sélectionne la plage `[0..n)` (`views/dolphinview.cpp:835-839`).
- Inverser la sélection (Ctrl+Maj+A) : bascule `[0..n)` (`dolphinview.cpp:841-845`).
- **Échap** : vide la sélection (hors mode sélection unique), annule la recherche en tapant et quitte le mode sélection s'il est actif (`kitemlistcontroller.cpp:446-454`).
- La recherche en tapant est annulée quand la sélection devient vide ou que l'élément courant disparaît (`private/kitemlistkeyboardsearchmanager.cpp:94-111`).

### 1.8 Bascule de sélection au survol [v1]
- Avec le réglage « case de sélection » actif (par défaut `true`, `dolphin_generalsettings.kcfg:110-112`), un petit bouton +/✓ apparaît sur l'icône survolée. Il n'apparaît pas si le bouton gauche est déjà enfoncé (`kitemlistwidget.cpp:273-277`).
- Taille : **16 px**, **22 px** si l'icône fait 48 px ou plus, **32 px** si elle fait 128 px ou plus (`kstandarditemlistwidget.cpp:563-575`).
- Un clic dessus bascule l'élément sans toucher au reste, et le rend courant et ancre (`kitemlistcontroller.cpp:1640-1653`). Un glisser depuis un élément non sélectionné le sélectionne d'abord (`:638-642`).

---

## 2. Clavier

Référence générale : `kitemlistcontroller.cpp:238-516`.

### 2.1 Règles communes [MVP]
- **Sans modificateur** : la touche déplace l'élément courant, **remplace** la sélection par lui et en fait l'ancre (`:497-509`).
- **Maj** : l'élément courant bouge et l'ancre reste. La plage `[ancre..courant]` est sélectionnée. Avec Haut/Bas, si aucune plage n'est active mais que l'élément courant est sélectionné, une plage démarre depuis lui (`:363-374`).
- **Ctrl** : seul l'élément courant (cadre de focus) bouge, la sélection ne change pas (`:493-496`). Ensuite, **Ctrl+Espace** bascule l'élément courant.
- **Espace sans modificateur** : sélectionne l'élément courant s'il ne l'est pas. Sinon, l'espace est envoyé à la recherche en tapant, mais seulement si une recherche est déjà en cours (`:456-475`, `kitemlistkeyboardsearchmanager.cpp:39-44`).
- **Un seul élément dans la vue** : n'importe quelle touche de navigation le sélectionne (`:316-322`).
- Après une touche de navigation, la vue défile au plus court pour montrer l'élément courant, marges incluses (`:512-514`, `kitemlistview.cpp:565-605`).
- **Entrée** : s'il y a 2 éléments sélectionnés ou plus, ils sont tous activés. S'il y en a un, il est activé. S'il n'y en a aucun, c'est l'élément courant (`:433-444`).
- Les touches non consommées remontent à la fenêtre pour les raccourcis (`:476-479`).
- **Droite-à-gauche** : Gauche et Droite sont inversées (`:246-260`). [Plus tard]

### 2.2 Navigation dans une grille (icônes) [MVP]
- **Gauche/Droite** : élément précédent ou suivant dans l'ordre linéaire. On passe d'une ligne à l'autre.
- **Haut/Bas** : on va dans la ligne précédente ou suivante, sur l'élément dont la **position x est la plus proche** de la position x mémorisée (« ancre clavier », `:1509-1607`).
  - La position x mémorisée n'est mise à jour que par Gauche, Droite, Début, Fin, un clic ou la recherche. En descendant à travers une ligne courte (fin de groupe), on retrouve donc la colonne d'origine à la ligne suivante.
- **Début/Fin** : premier ou dernier élément.
- **PgPréc/PgSuiv** : on monte ou descend ligne par ligne, en gardant la colonne, jusqu'au dernier élément dont le haut (ou le bas) reste dans une hauteur de vue à partir de l'élément actuel (`:377-431`).

### 2.3 Navigation en vue compacte (défilement horizontal) [v1]
- La vue compacte remplit des **colonnes de haut en bas** et défile horizontalement. Les flèches sont tournées en interne : Haut/Bas = précédent/suivant dans la colonne, Gauche/Droite = colonne voisine (`:296-314`).
- PgPréc/PgSuiv : premier ou dernier élément de la colonne courante (`:378-386`, `:406-414`).

### 2.4 Navigation en vue détails avec dossiers dépliables [v1]
- **Droite** sur un dossier dépliable : le **déplie**, et déplie aussi **tous les dossiers sélectionnés**. **Gauche** : les replie (`:262-284`). Sans Maj, les dossiers traités sont aussi sélectionnés.
- **Gauche** sur un élément enfant (niveau > 0) qui n'est pas un dossier déplié : on remonte au **dossier parent** (`:337-351`).
- Si le dossier ne peut pas changer d'état (déjà déplié, ou vide), la touche retombe sur la navigation normale.

### 2.5 Recherche en tapant (type-ahead) [MVP]
Référence : `private/kitemlistkeyboardsearchmanager.cpp:12-90`, `kitemlistcontroller.cpp:518-551`, `kfileitemmodel.cpp:579-594`.
- **Délai : 1000 ms**. Si plus de 1000 ms séparent deux frappes, la chaîne repart de zéro.
- Correspondance : **préfixe**, **insensible à la casse**, **insensible aux diacritiques** (« é » trouve « e »), sur le nom affiché.
- La recherche part de l'élément courant, ou du suivant pour la première lettre d'une nouvelle recherche. Elle boucle jusqu'à la fin, puis reprend au début. S'il n'y a ni sélection ni élément courant pertinent, elle part de l'index 0.
- **Répétition d'une même lettre** (« aaa ») : si la chaîne complète ne trouve rien, on passe au prochain élément qui commence par cette lettre. Une chaîne déjà trouvée sert de base : « 444 » trouvé, puis « 4444 » → suivant de « 444 ».
- Effet : l'élément trouvé devient courant, **seul sélectionné** (sauf en mode sélection) et ancre. La vue le fait défiler **en haut** de l'écran (position « Beginning », `:548`).
- Si l'espace est la première frappe, elle n'ouvre pas de recherche.

> Notes pour Rust/Slint : pré-calculer un nom « plié » (minuscules + suppression des diacritiques via `unicode-normalization` NFD + filtrage des marques combinantes) au chargement. La recherche devient alors un simple `starts_with`.

### 2.6 Zoom [MVP]
- **Ctrl+molette** : un cran de zoom tous les 120 unités d'angle accumulées, pour les molettes haute résolution (`views/dolphinview.cpp:1174-1190`). Ctrl+molette n'est jamais transmis au défilement (`kitemlistcontainer.cpp:179-182`).
- Pincement (pavé tactile) : un cran par variation d'échelle cumulée de ±0,2 (`kitemlistcontroller.cpp:1193-1219`). [Plus tard]

---

## 3. Dispositions, tailles, zoom

### 3.1 Niveaux de zoom [MVP]
Référence : `views/zoomlevelinfo.cpp:13-72`.

| Niveau | 0 | 1 | 2 | 3 | 4 | 5 | 6 | … | 16 |
|---|---|---|---|---|---|---|---|---|---|
| Icône (px) | 16 | 22 | 32 | 48 | 64 | 80 | 96 | +16/niveau | 256 |

- Au-delà du niveau 4 : `taille = 64 + (niveau − 4) × 16`.
- Chaque mode mémorise **deux tailles** : une pour les icônes, une pour les aperçus (vignettes affichées). Le zoom modifie celle qui est active (`views/dolphinitemlistview.cpp:34-75`).

| Mode | Icône par défaut | Aperçu par défaut | Source |
|---|---|---|---|
| Icônes | 32 (niv. 2) | **64 (niv. 4)** | `settings/dolphin_iconsmodesettings.kcfg:18-24` |
| Compact | 16 (niv. 0) | 48 (niv. 3) | `dolphin_compactmodesettings.kcfg:18-24` |
| Détails | 16 (niv. 0) | 48 (niv. 3) | `dolphin_detailsmodesettings.kcfg:18-24` |

- Les aperçus sont **activés par défaut** (`dolphin_directoryviewpropertysettings.kcfg:36-39`). Le mode par défaut est **Icônes** (`:24-27`).
- Le zoom est appliqué **immédiatement** au dessin, mais la régénération des vignettes attend **300 ms** sans nouveau changement de zoom (`kfileitemlistview.cpp:34-37`, `:390-405`).
- Un changement de taille qui modifie le nombre de colonnes **n'est pas animé** (`kitemlistview.cpp:880-883`).

### 3.2 Vue Icônes [MVP]
Référence : `views/dolphinitemlistview.cpp:209-230`, `kstandarditemlistwidget.cpp:136-195`, `:1322-1400`.
- Marge intérieure (padding) = **2 px**.
- **Largeur de cellule** : `48 + indexLargeurTexte × 64 × facteurPolice × facteurZoom`, avec :
  - `facteurZoom = exp(niveau / 13)` (1,0 au niveau 0 ; 1,36 au niveau 4 ; 3,4 au niveau 16) ;
  - `facteurPolice = largeurMoyenneCaractère / 9` (9 px correspond à Noto Sans 10 pt) ;
  - `indexLargeurTexte` vaut **1** par défaut (`dolphin_iconsmodesettings.kcfg:26-28`). Les valeurs correspondent à Petite, Moyenne, Grande… dans les réglages ;
  - plancher : `icône + 4 × facteurZoom`.
  - Exemple par défaut (aperçus 64 px, police standard) : environ **135 px** de large.
- **Hauteur** : `3 × padding + icône + hauteurLigne × lignes`. Le nom est **coupé sur au plus 3 lignes** par défaut (`MaximumTextLines=3`, `:30-32`), à la limite des mots ou n'importe où si un mot est trop long. Chaque ligne de rôle supplémentaire (taille, date sous le nom) ajoute une ligne.
- La hauteur d'une **rangée** est celle de son élément le plus haut (`private/kitemlistviewlayouter.cpp:478-531`).
- La dernière ligne visible est **élidée** quand le nom dépasse le nombre maximal de lignes (`kstandarditemlistwidget.cpp:1360-1373`).
- Avant découpe, le nom passe par un pré-traitement qui insère des points de coupure possibles (après `_`, `.`, `-`…, via `KStringHandler::preProcessWrap`). Les retours à la ligne dans un nom sont affichés `↵` (`:1310-1318`).
- Marges entre cellules = `PM_SizeGripSize` du style, identiques en horizontal et en vertical (`dolphinitemlistview.cpp:225-227`).
- L'espace horizontal inutilisé est **réparti également** entre les colonnes : la grille est justifiée (`kitemlistviewlayouter.cpp:405-415`).
- Défilement **vertical**.

### 3.3 Vue Compacte [v1]
Référence : `dolphinitemlistview.cpp:231-244`, `kstandarditemlistwidget.cpp:197-245`.
- Icône à gauche et texte à droite, sur une seule ligne. Les rôles supplémentaires s'empilent en lignes sous le nom.
- Hauteur : `2×padding + max(icône, nbRôles × hauteurLigne)`.
- Largeur de base : `4×padding + icône + 5 × hauteurPolice`. Une **colonne** prend la largeur du plus long texte qu'elle contient.
- Largeur maximale du texte : sans limite par défaut (`MaximumTextWidthIndex=0`). Sinon `index × 10 × hauteurPolice`, avec élision au-delà.
- Marge horizontale de 8 px. Défilement **horizontal**, avec remplissage colonne par colonne.

### 3.4 Vue Détails [MVP]
Référence : `dolphinitemlistview.cpp:245-249`, `kitemlistview.cpp:2385-2560`, `private/kitemlistheaderwidget.cpp`.
- Hauteur de ligne : `2×padding + max(icône, hauteurLigne)`, soit environ 20-22 px avec une icône de 16.
- **Colonnes par défaut** : Nom, Taille, Modifié (`views/viewproperties.cpp:535-542`). L'en-tête n'est visible qu'en vue détails.
- **Marges latérales** de **20 px** à gauche et à droite par défaut (`dolphin_detailsmodesettings.kcfg:30-36`). On peut les redimensionner en tirant le bord gauche de la première colonne ou le bord droit de la dernière. Elles servent de zone de désélection et de dépôt.
- **Surligner toute la ligne** : activé par défaut (`:38-40`). La sélection et le survol couvrent alors toute la largeur des colonnes. Mais **un appui ne vise un élément non sélectionné que sur sa zone cœur** (icône + nom), pour qu'on puisse démarrer un lasso depuis la zone vide d'une ligne (`kstandarditemlistwidget.cpp:525-555`).
- **Lignes en couleurs alternées** quand il y a plus d'une colonne (`kstandarditemlistview.cpp:22`). Quand la vue n'a pas le focus, la couleur alternée est légèrement atténuée (±1 %) (`kitemlistwidget.cpp:106-121`).
- **Largeur des colonnes, mode automatique** (par défaut) :
  - largeur préférée = max(largeur du titre + marges de l'en-tête, contenu le plus large) + `6 × padding` (12 px) (`kstandarditemlistwidget.cpp:64-96`, `:1783-1786`) ;
  - pour la colonne Nom, on ajoute icône, `2×padding` et, si les dossiers sont dépliables, `(niveau + 1) × hauteurLigne` ;
  - **budget de temps** : si le calcul dépasse **200 ms** après 100 éléments, il s'arrête et accepte une colonne un peu trop étroite (`kitemlistview.cpp:2420-2426`) ;
  - la **colonne Nom s'étire** pour remplir la largeur restante. Si la place manque, elle se réduit jusqu'à un minimum de `min(sa largeur, 2×icône + 200)` px, et une barre de défilement horizontale apparaît (`kitemlistview.cpp:2537-2560`).
- **Redimensionnement manuel** : on tire la poignée (le curseur devient ↔). La largeur minimale est de `4 × hauteurPolice` (`kitemlistheaderwidget.cpp:191-195`, `:282-323`). Le premier redimensionnement manuel **désactive le mode automatique** pour cette vue. Tirer la marge droite à zéro puis au-delà de 10 px pousse la dernière colonne au-delà de l'écran (`:297-312`).
- **Double-clic sur une poignée** : la colonne revient à sa largeur préférée, ajustée au contenu (`:382-397`).
- **Réordonner** : on glisse un titre de colonne au-delà de la distance de glisser, et la colonne suit le pointeur. **La colonne Nom n'est pas déplaçable** : le curseur devient « interdit » (`:325-379`).
- **Clic sur un titre** : si c'est la colonne de tri, l'ordre s'inverse. Sinon, le tri passe sur cette colonne avec **l'ordre par défaut du rôle** (voir §4.4) (`:250-268`).
- Info-bulle sur certains titres (dates, permissions) indiquant que le format est configurable (`views/dolphinview.cpp:1153-1163`). [Plus tard]
- Défilement à la molette en vue détails : arrondi à un **nombre entier de lignes** par cran (`kstandarditemlistview.cpp:113-122`). Pas de base dans les autres vues : `2 × hauteurPolice` par ligne de molette (`kitemlistview.cpp:402-413`).

### 3.5 Dossiers dépliables (arborescence en vue détails) [v1]
Référence : `kfileitemmodel.cpp:845-913`, `kstandarditemlistwidget.cpp:1055-1080`.
- Activé par défaut (`ExpandableFolders=true`). Uniquement en vue détails.
- Un dossier est dépliable s'il contient **au moins une entrée** (rôle calculé en tâche de fond).
- Un triangle (expander) précède l'icône. Le décalage vaut `niveau × hauteurLigne` : chaque niveau est indenté d'une hauteur de ligne.
- Un clic sur le triangle bascule l'état **sans changer la sélection** et sans activer. L'élément devient courant (`kitemlistcontroller.cpp:1633-1639`, `:1866-1871`).
- Un double-clic sur un dossier dépliable le déplie **et** l'ouvre (`:723-729`).
- Déplier lance une écoute du sous-dossier. Les enfants sont insérés **juste après le parent, triés** selon les mêmes règles que le niveau 0 (le parent précède toujours ses descendants, `kfileitemmodel.cpp:2362-2391`).
- Replier retire les enfants et **mémorise les sous-dossiers qui étaient dépliés** : les redéplier restaure l'arbre (`:880-909`).
- Le regroupement par catégories ignore les enfants : seuls les éléments de niveau 0 créent des groupes.
- Pendant un glisser, le survol prolongé d'un dossier dépliable le **déplie ou replie** au lieu de l'ouvrir (§7.3).

### 3.6 Élision des noms [MVP]
Référence : `kstandarditemlistwidget.cpp:1285-1308`, `dolphin_contentdisplaysettings.kcfg:41-47`.
- Mode par défaut : **milieu** (« Rapport_fin…2024.pdf »).
- Mode alternatif, « droite » : `début…` + **extension conservée** (« Rapport_fin….pdf »). Si même « ….ext » ne tient pas, l'extension est sacrifiée.
- Colonnes autres que Nom : élision à droite (`:1418`).
- En vue icônes, si les info-bulles riches sont désactivées, le survol d'un nom élidé affiche le **nom complet en info-bulle simple** (`views/dolphinview.cpp:2872-2893`). [v1]

---

## 4. Tri

### 4.1 Ordre des critères [MVP]
Référence : `kfileitemmodel.cpp:2358-2429`, `:2451-2618`.
1. **Arborescence** (vue détails dépliée) : un parent précède ses descendants. Deux éléments de branches différentes sont comparés par leurs ancêtres de même niveau.
2. **Cachés en dernier** (si activé, **désactivé par défaut**, `dolphin_directoryviewpropertysettings.kcfg:72-74`) : ce critère s'applique **avant** « dossiers d'abord ». On obtient donc dossiers visibles, fichiers visibles, puis dossiers cachés, fichiers cachés.
3. **Dossiers d'abord** (activé par défaut, `:67-69`). Il est aussi forcé quand on trie par taille et que la taille d'un dossier s'affiche en nombre d'éléments, car les unités ne sont pas comparables.
4. **Groupe** (si le regroupement est actif et que le rôle de groupe diffère du rôle de tri) : les groupes sont **toujours croissants**, quel que soit l'ordre de tri.
5. **Rôle de tri** dans l'ordre demandé.
6. Départage : nom affiché, puis nom réel, puis URL complète. Le tri doit **ne jamais retourner « égal »** pour rester stable (bug KDE 433247, `:2453-2456`, `:2600-2617`).
- Note : « dossiers d'abord » reste vrai en ordre **décroissant**, car l'inversion ne porte que sur le rôle (étape 5).

### 4.2 Tri naturel des noms [MVP]
Référence : `kfileitemmodel.cpp:43-262`, `:2620-2648`, `:1276-1298`.
- Réglage global à trois valeurs : **Naturel** (défaut), insensible à la casse, sensible à la casse (`dolphin_generalsettings.kcfg:164-172`).
- Le mode naturel repose sur une collation locale ICU, insensible à la casse, avec mode numérique. Dolphin y ajoute un comparateur maison **sensible aux décimales et aux versions** :
  - les suites de chiffres ASCII sont comparées **numériquement**, zéros de tête ignorés (`007` = `7` au premier passage) ;
  - une chaîne de **deux segments** `a.b` est lue comme un **décimal** : `1.5 > 1.10`, car 0,5 > 0,10 ;
  - une chaîne de **trois segments ou plus** est lue comme une **version**, segment par segment : `1.2.10 > 1.2.9` ; à égalité de préfixe, la chaîne la plus longue est plus grande (`:142-182`) ;
  - les segments de texte sont comparés par la collation. Dès qu'ils diffèrent, c'est **le reste entier des deux chaînes** qui est comparé par la collation (`:236-241`) ;
  - en cas d'égalité, départage par comparaison brute insensible puis sensible à la casse, pour un ordre déterministe (`:250-261`).
- **Extension traitée à part** : on compare d'abord le nom de base, puis l'extension. Le séparateur est le dernier `.`, **sauf s'il est entouré de chiffres**. Ainsi `v1.5` n'a pas d'extension et `photo.2.jpg` a pour base `photo.2` (`:183-200`, `:2624-2637`). Résultat : `a.txt` < `a-b.txt`.
- Modes non naturels : comparaison Unicode brute, insensible ou sensible à la casse. En mode insensible, départage par la casse.

> Notes pour Rust/Slint : utiliser `icu_collator` avec `numeric: On` et `strength: Secondary` (insensible à la casse), puis reproduire en surcouche le traitement décimal/version et la séparation de l'extension. Pré-calculer une **clé de tri** par fichier au chargement (`icu_collator` ne donne pas de clé binaire ; sinon stocker des segments pré-découpés). La comparaison reste de loin le point chaud du tri de 100k noms.

### 4.3 Rôles de tri et de colonnes disponibles
Référence : `kfileitemmodel.cpp:3124-3171`.
- **[MVP]** Nom (`text`), Taille, Modifié, Type.
- **[v1]** Créé, Accédé, Extension, Chemin, Nom du dossier, Permissions, Propriétaire, Groupe, Cible du lien, Date de suppression (corbeille).
- **[Plus tard]** Rôles de métadonnées (Baloo) : Note, Étiquettes, Commentaire, Provenance du téléchargement ; Document (Titre, Auteur, Éditeur, Pages, Mots, Lignes) ; Image (Date de prise de vue, Dimensions, Largeur, Hauteur, Orientation) ; Audio (Album, Artiste, Codec, Débit, Durée, Genre, Année, Piste) ; Vidéo (Format d'image, Codec, Débit, Espace colorimétrique, Dimensions, Durée, Images/s, Format de pixel).
- Comparaison par rôle : la taille d'un fichier est comparée en octets. Pour un dossier, c'est sa taille récursive, ou son **nombre d'éléments** (mode par défaut `ContentCount`, `dolphin_contentdisplaysettings.kcfg:9-22`) ; une valeur inconnue passe en premier. Les dates sont comparées par horodatage. Pour les rôles texte, **les valeurs vides vont en fin** de liste, en comparaison naturelle si le rôle est textuel (`kfileitemmodel.cpp:2563-2593`).

### 4.4 Ordre par défaut d'un rôle [MVP]
- Lors d'un changement de rôle par clic d'en-tête : ordre **décroissant** pour les dates, la taille, les dimensions, les pages, mots et lignes, la note, la durée, le débit et les images/s. **Croissant** pour les autres (`views/dolphinview.cpp:1877-1900`).
- L'ordre choisi est mémorisé **par rôle** pendant la session (`dolphinview.cpp:2895-2902`). [v1]

### 4.5 Re-tri en direct [MVP]
- Quand une valeur triable change (fichier modifié, taille de dossier calculée), le modèle vérifie d'abord si l'élément est encore à sa place par rapport à ses voisins. S'il ne l'est plus, un re-tri complet est programmé avec un **anti-rebond de 100 ms** (`kfileitemmodel.cpp:328-335`, `:2075-2125`).
- Le re-tri produit un signal « éléments déplacés » sur la plage minimale touchée, pas une réinitialisation : la sélection et le défilement survivent (`:1300-1375`).

---

## 5. Regroupement par catégories [v1]

Référence : `kfileitemmodel.cpp:635-686`, `:2650-2990` ; disposition `private/kitemlistviewlayouter.cpp:455-531`.
- Désactivé par défaut (`GroupedSorting=false`). Le rôle de groupe est séparé du rôle de tri (`GroupRole`, vide = identique au rôle de tri).
- Chaque groupe commence sur une **nouvelle ligne**, précédée d'un en-tête :
  - hauteur d'en-tête = `hauteurPolice + 2×padding + margeVerticale` en icônes, `hauteurPolice + 4×padding` en détails ;
  - espace avant un groupe (sauf le premier) = `icône / 4` en icônes, `icône / 2` en détails (`kitemlistview.cpp:2691-2713`) ;
  - en vue compacte, les en-têtes sont en haut de chaque groupe de colonnes, avec une largeur minimale d'environ 15 caractères (`kitemlistviewlayouter.cpp:496-512`).
- Groupes **par nom** : la première lettre en majuscule, rangée dans A–Z selon la collation locale (« É » va dans E). Un `~` initial est ignoré. Les chiffres vont dans « 0 - 9 », les symboles dans « Autres ». Une lettre non latine forme son propre groupe (`:2650-2720`).
- Groupes **par taille** : Dossiers, Petit (< 5 Mo), Moyen (< 10 Mo), Gros (`:2723-2765`).
- Groupes **par date** (modifiée, créée, accédée, supprimée) (`:2767-2925`) :
  - dans le mois courant : Aujourd'hui, Hier, nom du jour (2-6 jours), Il y a une semaine, Il y a deux semaines, Il y a trois semaines, Plus tôt ce mois-ci ;
  - le mois précédent : variantes « Hier (Mois aaaa) », « Il y a une semaine (Mois aaaa) »…, puis « Plus tôt en Mois aaaa » ;
  - au-delà : « Mois aaaa ».
- Groupes par note et par permissions : groupes dédiés. Les autres rôles groupent par **valeur texte exacte**.
- Les groupes sont recalculés paresseusement : après une insertion ou un tri, la liste est vidée et recalculée au premier accès (`:635-640`, `:1813`).

---

## 6. Survol, mise en évidence, info-bulles

### 6.1 Rendu des états [MVP]
Référence : `kitemlistwidget.cpp:103-149`, `:630-690`.
- Avec le style Breeze, Dolphin dessine lui-même les états : rectangle arrondi de **rayon 5 px** sur la zone pleine, trait de 1,25 px.
  - **survol seul** : couleur du texte à **6 %** d'opacité ;
  - **sélectionné** : couleur de surbrillance à **32 %** ;
  - **sélectionné + survolé** : surbrillance à **40 %** ;
  - **appui** (clic en cours) : surbrillance à **100 %** ;
  - **élément courant** : contour de surbrillance (assombri de 10 % sur thème clair, éclairci de 10 % sur thème sombre), opaque si sélectionné ou survolé, à 80 % sinon. Le contour n'apparaît que si la vue a le focus et la fenêtre est active.
- Un élément courant non sélectionné n'a **pas** de fond de sélection, seulement le contour (`:123-131`).
- Pendant le renommage en ligne, la mise en évidence de l'élément est masquée.
- **Fichiers coupés** (presse-papiers « couper ») : icône rendue **désactivée** (grisée). **Fichiers cachés** : icône **semi-transparente** (`kstandarditemlistwidget.cpp:1158-1164`, `:1003-1012`).
- **Lien symbolique** : nom en *italique*. C'est la police « lien » de la vue fichier, hors périmètre exact ici.

### 6.2 Règles de survol [MVP]
Référence : `kitemlistcontroller.cpp:948-1051`.
- Le survol est **instantané**, sans fondu, dans cette version (`kitemlistwidget.cpp:263-290`).
- Le survol cible la zone cœur. Le triangle d'arborescence a son propre état de survol, exclusif de celui de l'élément.
- **Avec une multi-sélection**, survoler la zone vide d'une ligne ne met **pas** la ligne en évidence. C'est un indice visuel qu'un clic à cet endroit va désélectionner (`:1006-1012`).
- Quitter la vue retire tout survol et tout état « appuyé ».

### 6.3 Info-bulles [v1]
Référence : `views/tooltips/tooltipmanager.cpp:40-95`, `views/dolphinview.cpp:1482-1500`.
- Info-bulles riches **désactivées par défaut** (`ShowToolTips=false`).
- Quand elles sont actives : la récupération du contenu (aperçu **512×512**, métadonnées) démarre après **200 ms** de survol, et l'affichage a lieu à **500 ms**. Un passage rapide sur des éléments ne lance donc aucune tâche coûteuse.
- Aucune info-bulle pendant un glisser. Toute info-bulle se ferme au défilement, au renommage, au changement de dossier ou à la perte de visibilité de la fenêtre.
- Chaque survol d'élément émet en plus « demander les infos de l'élément » : c'est ce qui alimente le **panneau d'infos**.

> Notes pour Rust/Slint : le panneau d'infos de Satchel doit suivre le survol **et** la sélection, avec l'anti-rebond de 300 ms de §1.1 sur la sélection. Prévoir le même délai à deux étages (200 ms pour charger, 500 ms pour afficher) si on ajoute des info-bulles.

---

## 7. Glisser-déposer

### 7.1 Démarrage d'un glisser depuis la vue [MVP]
Référence : `kitemlistcontroller.cpp:633-650`, `:1423-1453`, `kfileitemlistview.cpp:109-188`.
- Le glisser démarre quand le bouton gauche est enfoncé sur un élément et que le pointeur s'est déplacé d'au moins **`startDragDistance`** (distance de Manhattan, 10 px par défaut dans Qt/KDE).
- Si l'élément pressé n'était pas sélectionné (cas de la case de sélection), il est d'abord ajouté. Si on glisse un élément déjà sélectionné, **toute la sélection** part et elle n'est pas réduite au relâchement.
- Actions proposées : Déplacer | Copier | Lien, avec **Copier** par défaut. Dolphin affiche ensuite un menu au dépôt si aucun modificateur n'est enfoncé.
- **Image du glisser** :
  - 1 élément : rendu de l'icône ou de la vignette de l'élément ;
  - 2 à 9 éléments : grille de 3 colonnes au plus, icônes de **32 px** ;
  - 10 à 16 : 4 colonnes au plus, **22 px** ;
  - plus de 16 : 5 colonnes au plus, **16 px**, la grille étant tronquée à 5×5 ;
  - 1 px d'espace entre les icônes, point d'accroche **en haut au centre**.
- Les URLs sont aussi exportées pour le portail XDG (glisser vers des applications Flatpak). [v1]

### 7.2 Retour visuel au survol d'une cible [MVP]
Référence : `kitemlistcontroller.cpp:812-939`, `views/draganddrophelper.cpp:20-67`.
- La cible de dépôt est l'élément dont la **zone cœur** est sous le pointeur. Sur une zone vide (marges, fond, partie vide d'une ligne en détails), la cible est **le dossier affiché**.
- L'élément cible passe à l'état **survolé** (même rendu que le survol souris).
- **Cible acceptée** : dossier accessible en écriture, fichier `.desktop`, ou fichier local **exécutable** (dépôt = lancer avec les fichiers en argument).
- **Refus** (curseur « interdit ») : si l'une des URLs glissées **est la cible elle-même**, comme déposer un dossier sur lui-même ou déposer dans le dossier d'origine sur le fond. Le résultat est mis en cache par URL cible pendant tout le glisser.
- L'auto-défilement près des bords s'applique aussi pendant un glisser (§1.6). Quitter la vue arrête l'auto-défilement et le minuteur d'ouverture.

### 7.3 Ouverture automatique au survol (« ressort ») [v1]
- Option **désactivée par défaut** (« Ouvrir les dossiers pendant un glisser », `AutoExpandFolders=false`).
- Quand elle est active : après **750 ms** sur un dossier (ou un élément `.desktop` ou exécutable), le dossier s'ouvre. En vue détails avec un dossier dépliable, il est **déplié ou replié** à la place (`kitemlistcontroller.cpp:65-68`, `:553-581`).
- Pendant l'attente, l'icône **clignote** : `folder-open` pendant le premier tiers, l'icône d'origine pendant le deuxième, `folder-open` pendant le dernier (`kstandarditemlistwidget.cpp:608-640`).
- L'ouverture n'a lieu que si le pointeur est encore au-dessus de la vue (bugs KDE 293200, 305783).
- Les onglets s'activent au survol après **800 ms** (`dolphintabbar.cpp:51-54`).

---

## 8. Renommage en ligne

### 8.1 Déclenchement [MVP]
Référence : `views/dolphinview.cpp:854-900`, `kitemlistview.cpp:758-777`.
- F2 ou action « Renommer » : avec **un seul** élément sélectionné et le renommage en ligne activé (défaut `RenameInline=true`), la vue fait d'abord **défiler jusqu'à l'élément**. **Une fois le défilement terminé**, l'éditeur s'ouvre et les info-bulles se ferment.
- Avec plusieurs éléments : boîte de dialogue de renommage par lot [v1]. Après le renommage, les nouveaux noms sont resélectionnés.
- **Clic lent** sur le nom [v1] : un clic sur le **texte** du **seul** élément sélectionné, en mode double-clic, lance un minuteur égal à l'intervalle de double-clic du système. S'il n'y a pas de second clic, et que le même élément est toujours seul sélectionné, l'éditeur s'ouvre. Le clic lent est annulé par un double-clic, un glisser, un changement de dossier ou un défilement. Il est réservé aux éléments déplaçables (`dolphinview.cpp:1594-1604`, `:2115-2129`, `:1090-1094`).

### 8.2 Présélection [MVP]
Référence : `kfileitemlistwidget.cpp:161-188`.
- **Dossier** : nom entier sélectionné.
- **Fichier** : nom **sans l'extension**. L'extension est déterminée par la base MIME, ce qui gère `.tar.gz`. Pour une extension inconnue, on exclut ce qui suit le dernier `.`, sauf si ce point est le premier ou le dernier caractère (`.bashrc` et `fichier.` sont sélectionnés entièrement).

### 8.3 Éditeur et touches [MVP]
Référence : `private/kitemlistroleeditor.cpp:15-177`, `kstandarditemlistwidget.cpp:861-1000`.
- Zone de texte multiligne sans barre de défilement. Elle reprend la police et l'alignement du nom (centré en icônes). Elle **grandit en hauteur** avec le contenu, dans la limite de la vue.
- **Entrée** : valide. **Échap** : annule, sans valider par la perte de focus qui suit.
- **Perte de focus** : **valide**, sauf si le focus part vers un menu ou une popup (par exemple la correction orthographique).
- **Redimensionnement de la vue** : valide et ferme l'éditeur.
- **Tab** / **Maj+Tab** : valide et ouvre l'éditeur sur l'élément **suivant / précédent**, ce qui permet de renommer en chaîne. En vues **compacte et détails**, **Bas / Haut** font de même. En vue icônes, Haut/Bas déplacent le curseur dans le texte (`kitemlistroleeditor.cpp:89-102`, `views/dolphinview.cpp:2392-2397`).
- **Gauche / Droite** avec une sélection active : le curseur va au début ou à la fin de la sélection, qui est effacée. **Début / Fin** (avec Maj éventuel) : début ou fin du texte entier (`:103-128`).
- Le texte saisi est encodé comme nom de fichier : `/` devient un caractère de remplacement (`KIO::encodeFileName`).
- L'éditeur suit le défilement de la vue (`kitemlistview.cpp:776`).

### 8.4 Validation et erreurs [MVP]
Référence : `views/dolphinview.cpp:2291-2398`, `:2173-2190`.
- On **ignore** les noms vides, inchangés, `.` ou `..`. Aucun message n'est affiché.
- **Nom commençant par `.`** alors que les fichiers cachés sont masqués : confirmation « Renommer et masquer » / Annuler. [v1]
- **Changement d'extension** qui change le type MIME (fichier local, type connu) : confirmation avec l'icône du nouveau type et l'option « Ne plus demander » (`ConfirmRenameFileType=true` par défaut). Le nouveau type est deviné avec le nom **et le contenu**. [v1]
- Le renommage est un déplacement asynchrone, **enregistré pour annuler** (Ctrl+Z). Les erreurs, comme les permissions ou un nom existant, passent par la boîte d'erreur ou de conflit standard de KIO.
- L'élément renommé reste **sélectionné et courant** sous son nouveau nom. Le modèle n'est mis à jour **qu'en cas de succès**. Si un fichier porte déjà le nouveau nom, on laisse le dialogue de conflit agir sans toucher au modèle (bug KDE 328262).

> Notes pour Rust/Slint : Slint n'a pas d'éditeur de texte flottant au-dessus d'une liste virtualisée ; il faut un `TextInput` positionné en absolu au-dessus de la cellule, avec des coordonnées recalculées à chaque défilement. Prévoir un état explicite `Editing { index, original, direction }`, et gérer la perte de focus (= valider) avec soin dans la boucle d'événements.

---

## 9. Vignettes et icônes

### 9.1 Quand les rôles et les vignettes sont calculés [MVP icônes, v1 vignettes]
Référence : `kfileitemmodelrolesupdater.cpp:46-58`, `:880-963`, `:1426-1490`.
- Le listage n'attend pas le type MIME : il est déterminé **paresseusement** (`kfileitemmodel.cpp:290`). Les éléments s'affichent d'abord avec une **icône provisoire**, déduite de l'extension ou de « dossier ».
- Dès que la plage visible est connue : **icônes définitives** des éléments visibles calculées **de façon synchrone, dans un budget de 200 ms**. Le reste est traité en tâche de fond.
- **Ordre de priorité** des vignettes et rôles coûteux :
  1. **fichiers visibles** d'abord, puis **dossiers visibles**, plus lents (aperçu composite du contenu) ;
  2. jusqu'à **5 pages** après la zone visible, plafonné à 250 éléments ;
  3. jusqu'à 5 pages avant, en ordre inverse ;
  4. la **dernière page** puis la **première page**, pour que Début et Fin soient prêts ;
  5. le reste, jusqu'à **500 éléments au total**. Au-delà, rien n'est calculé hors écran.
- Pendant le défilement, la mise à jour de la plage visible est différée de **50 ms**, pour que le défilement ne soit jamais interrompu par du chargement d'icônes (`kfileitemlistview.cpp:28-32`, `:366-378`). Après un zoom, on attend **300 ms**.
- Les modifications de fichiers qui arrivent en rafale sont regroupées : **100 ms** sans nouvel événement avant de recalculer (`kfileitemmodelrolesupdater.cpp:107-112`).
- Changer le rôle de tri vers un rôle calculé (taille de dossier…) déclenche un calcul synchrone de ce rôle pour tous les éléments **dans un budget de 200 ms**, puis la suite en asynchrone avec re-tri progressif (`:527-560`).

### 9.2 Tailles et cache [v1]
Référence : `kfileitemmodelrolesupdater.cpp:986-1040`.
- On ne demande au générateur que **deux tailles** : **128×128** si l'icône fait 128 px ou moins, **256×256** au-delà. Elles correspondent au cache freedesktop `~/.cache/thumbnails/normal` et `large`. La réduction à la taille d'affichage se fait côté vue. Le rendu tient compte du facteur d'échelle de l'écran (DPR).
- En vue icônes, la vignette peut occuper toute la **largeur** de la cellule (moins 2×padding) pour une hauteur égale à la taille d'icône. Les vignettes paysage sont donc plus larges que l'icône (`kfileitemlistview.cpp:461-471`).
- **Cadre** : les vignettes **opaques** (photos, sans canal alpha) de plus de 22 px reçoivent un cadre avec ombre. Les vignettes avec transparence sont seulement mises à l'échelle.
- **Agrandir les petites vignettes** (défaut `true`) : une image plus petite que la cellule est agrandie. Sinon, elle est centrée dans un cadre à la taille de la cellule.
- Plugins de vignettes configurables : liste globale `PreviewSettings/Plugins`.
- **Séquence au survol** [Plus tard] : pour les vidéos et dossiers, le survol fait défiler des images successives toutes les **700 ms**, avec un pré-chargement de 5 images d'avance (`kitemlistwidget.cpp:280-283`, `kfileitemmodelrolesupdater.cpp:1040-1080`).

> Notes pour Rust/Slint : réutiliser le cache freedesktop (MD5 de l'URI, PNG avec `Thumb::URI`/`Thumb::MTime`), commun avec Dolphin, ce qui donne des vignettes déjà présentes. Générer les vignettes dans un pool de threads, avec une **file prioritaire reconstruite à chaque changement de plage visible** (ordre §9.1), et annuler les tâches sorties de la fenêtre.

---

## 10. Animations [v1]

Référence : `private/kitemlistviewanimation.cpp:98-170`, `kitemlistview.cpp:1873-2000`, `:2615-2631`.
- **Durée unique : 200 ms**, ou 1 ms (= aucune animation) si les animations sont désactivées dans le style système.
- **Apparition** (éléments insérés) : fondu d'opacité 0→1, courbe *InQuart*.
- **Disparition** (éléments supprimés) : fondu 1→0, courbe *OutQuart*.
- **Déplacement** (les éléments suivants se décalent) : animation de position, courbe linéaire.
  - Dans une grille, un élément n'est animé en glissement que s'il **reste sur la même ligne**. S'il change de ligne, il réapparaît à sa nouvelle place avec un fondu, pour éviter les croisements de trajectoires.
  - Une animation interrompue reprend vers la nouvelle cible **pour le temps restant**.
- **Quand on n'anime pas** :
  - lors du chargement initial (tous les éléments insérés d'un coup) ;
  - dans une grille, si plus des **2/3 d'une ligne** sont insérés ou supprimés ;
  - en vue détails avec dossiers dépliables (déplier ou replier ne s'anime pas) ;
  - si plusieurs plages sont supprimées à la fois ;
  - si le changement de taille de la vue, des marges ou du zoom change le **nombre de colonnes** ;
  - si le nombre maximal de lignes ou la largeur de texte change.
- **Défilement fluide** à la molette et au clavier : durée = durée d'animation du style (paramètre de vitesse global KDE), courbe *InOutQuad*. Un nouveau cran pendant l'animation la prolonge sans saut (*OutQuad*, avec correction d'une image). Le défilement fluide se désactive avec `SmoothScroll=false` (`private/kitemlistsmoothscroller.cpp:30-37`, `:80-125`, `:202-212`). Les pavés tactiles (pixelDelta) défilent sans animation.
- Fondu de fin du lasso : 150 ms (§1.6). Clignotement d'ouverture automatique : 750 ms (§7.3).

> Notes pour Rust/Slint : les `animate` de Slint sur `x/y/opacity` couvrent ces cas, mais ils s'appliquent à **toutes** les modifications. Il faut donc un drapeau « animer ce changement » côté modèle Rust (durée 0 sinon), qui reproduise les règles « quand on n'anime pas » ci-dessus. Sans cela, le chargement d'un dossier ou le redimensionnement de fenêtre deviennent lents et visuellement brouillons.

---

## 11. Performances : dossiers de 100k fichiers

### 11.1 Chargement incrémental [MVP]
Référence : `kfileitemmodel.cpp:316-325`, `:1417-1486`, `:1799-1898`.
- Les entrées reçues du listage sont **mises en attente** et pas insérées une par une. Elles sont envoyées au modèle :
  - à la **fin du listage**, ce qui donne une seule insertion pour un dossier local ;
  - ou au plus tard **toutes les 2000 ms** pour les sources lentes (réseau, recherche), afin d'afficher des résultats partiels.
- **Insertion par lots triés** : le lot est trié, puis **fusionné** avec la liste existante en O(n), en partant de la fin pour minimiser les déplacements. Le résultat est une liste de **plages insérées**, pas un « reset ». Dans le cas courant d'une vue vide, le lot devient directement la liste.
- **Pré-tri rapide** : avant le tri naturel, coûteux, le lot est pré-trié par comparaison brute de chaînes, très rapide. Le tri naturel, appliqué à une séquence presque triée, va alors beaucoup plus vite.
- **Tri parallèle** (*merge sort* multi-thread, autant de threads que de cœurs) pour les rôles textuels. Les rôles numériques sont triés sur un seul thread, car le comparateur n'est pas réentrant (bug KDE 312679) (`:2431-2449`).
- **Filtre** (barre de filtre) : les éléments filtrés sont gardés à part et ne sont pas détruits. Pour les arbres, la chaîne des parents d'un élément visible est conservée (`:1453-1474`).
- Un message « Chargement… » ne s'affiche qu'après **500 ms** de chargement (`views/dolphinview.cpp:171-174`).
- L'index URL → position est **reconstruit paresseusement par blocs de 1000** à la demande, au lieu d'être maintenu à chaque insertion (`kfileitemmodel.cpp:711-780`, `:1893`).

### 11.2 Mise en page paresseuse et virtualisation [MVP]
Référence : `private/kitemlistviewlayouter.cpp:357-612`, `private/kitemlistsizehintresolver.cpp`, `kitemlistview.cpp:1873-2000`.
- **Seuls les éléments visibles ont un widget**. Les widgets qui sortent de l'écran sont **recyclés** pour les éléments qui entrent : on change leur index et leurs données, sans créer de nouveau widget.
- La mise en page calcule pour **tous** les éléments une ligne et une colonne, plus un tableau de **décalages verticaux par ligne**. C'est O(n), mais avec des opérations très légères. Elle n'est refaite que si elle est marquée « sale ».
- La **plage visible** est trouvée par **recherche dichotomique** sur les décalages de ligne : O(log n) par défilement. On inclut la ligne partiellement visible au-dessus.
- **Hauteurs d'éléments en cache** : en vue icônes, la hauteur (nombre de lignes du nom) et l'état « élidé » sont mesurés une seule fois par élément et gardés dans un tableau parallèle au modèle. Le tableau est décalé lors des insertions et suppressions, et seules les entrées des éléments modifiés sont invalidées. On ne recalcule tout qu'en cas de changement de police, de largeur ou de zoom.
- **Regroupement des modifications** (« transactions ») : plusieurs changements de propriétés (zoom + taille + marges) ne déclenchent qu'**une** mise en page. Si l'un d'eux demande « sans animation », toute la transaction est non animée (`kitemlistview.cpp:1875-1882`).
- Largeur automatique des colonnes de détails avec un budget de 200 ms (§3.4). Calcul synchrone des icônes visibles avec un budget de 200 ms (§9.1).

### 11.3 Techniques à reproduire dans Satchel
1. **[MVP]** Listage en thread séparé (`std::fs::read_dir` ou `getdents64`), envoi par **lots** au thread UI : fin de listage, ou toutes les ~200 ms pour garder l'interface vivante. Dolphin attend 2 s pour les sources lentes, ce qui est trop long pour du local.
2. **[MVP]** Tri parallèle (`rayon::par_sort_by`) avec **clé de tri pré-calculée**, puis fusion O(n) dans la liste existante. Émettre des plages insérées, pas un reset.
3. **[MVP]** Liste virtualisée Slint (`ListView`, ou un `for` sur un sous-modèle de fenêtre visible), avec **recyclage**. Ne jamais instancier 100k éléments.
4. **[MVP]** Mise en page en tableau plat (`Vec<RowOffset>`) + dichotomie. Pour la grille, une hauteur de rangée = max des hauteurs mises en cache.
5. **[MVP]** Type MIME et icône définitive en tâche de fond. Afficher une icône par extension tout de suite.
6. **[v1]** File de vignettes priorisée (§9.1), plafond de 500 éléments hors écran, anti-rebond de 50 ms au défilement et de 300 ms au zoom.
7. **[v1]** Anti-rebond de 100 ms sur les re-tris déclenchés par des changements de fichiers. Anti-rebond de 300 ms sur la sélection envoyée au panneau d'infos.
8. **[MVP]** Sélection stockée en **plages** (Tout sélectionner sur 100k en O(1)). Le lasso ne teste que la fenêtre utile.
9. **À éviter** : le comparateur naturel de Dolphin découpe des sous-chaînes (`mid()`) à chaque comparaison, sous un mutex global de collation (`kfileitemmodel.cpp:2622`). En Rust, comparer des segments pré-découpés, sans allocation ni verrou.

---

## 12. Divers

- **Placeholder** de dossier vide ou en chargement : texte centré, police ×1,3, opacité 50 % (`views/dolphinview.cpp:178-198`). [MVP]
- **Fichiers cachés** masqués par défaut (`HiddenFilesShown=false`). Option pour masquer aussi les fichiers de type `application/x-trash` (`*~`, `.bak`), désactivée par défaut. [MVP]
- **Taille des dossiers** en vue détails : nombre d'éléments par défaut. Mode récursif possible, avec une profondeur limitée à 10 (`dolphin_contentdisplaysettings.kcfg:9-26`). Dates relatives courtes par défaut (« Hier, 14:03 »). [v1]
- **Réglages de vue par dossier ou globaux** : globaux par défaut (`GlobalViewProps=true`). Le zoom, le mode, le tri, les colonnes visibles et leurs largeurs sont mémorisés par dossier si on désactive cette option. [v1]
- **Mode sélection** (tactile, avec une barre d'outils) : clic = bascule, Entrée = bascule, flèches = déplacent le focus sans sélectionner. [Plus tard]
- **Gestes tactiles** : tap, tap long (menu), balayage à deux doigts (précédent/suivant), pincement (zoom) (`kitemlistcontroller.cpp:1067-1268`). [Plus tard]
- **Accessibilité** : la vue expose ses éléments aux lecteurs d'écran (`kitemviews/accessibility/`). [Plus tard, à prévoir dans l'architecture Slint]
