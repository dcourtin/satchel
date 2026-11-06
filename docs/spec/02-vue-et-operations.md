# 02 — Vue d'un dossier et opérations sur les fichiers

Livre de recettes comportemental extrait de **Dolphin 26.08.3**. Il ne s'agit pas d'une traduction du code de Dolphin. Les références `fichier.cpp:ligne` sont relatives à `dolphin-26.08.3/src/`.

Une bonne part du comportement réel vient de **KIO** (KF6), dont nous n'avons pas la source. Ces points sont marqués **(KIO, inféré)**. Ils décrivent le comportement connu de KIO. Les points marqués « à vérifier » doivent être confirmés dans `kio/src/` ou dans Dolphin avant d'être implémentés.

Priorités :
- **[MVP]** : gestionnaire local à deux panneaux, utilisable au quotidien.
- **[v1]** : finition attendue d'un gestionnaire complet.
- **[Plus tard]** : confort, réseau, cas rares.

---

## 1. Modes de vue

- **[MVP]** Trois modes : **Icônes** (0), **Détails** (1) et **Compact** (2). Ces valeurs sont stockées dans `ViewMode`, bornées par `qBound(0, mode, 2)`. Raccourcis : `Ctrl+1` Icônes, `Ctrl+2` Compact, `Ctrl+3` Détails. Le kcfg mentionne encore un mode « colonnes (2) » obsolète, à ignorer. Réf. `views/dolphinview.h:66`, `views/viewproperties.cpp:340-344`, `views/dolphinviewactionhandler.cpp:778-809`.
- **[MVP]** Un changement de mode remet les deux barres de défilement à 0, enregistre le mode dans les propriétés du dossier puis réapplique **toutes** les propriétés. Réf. `views/dolphinview.cpp:313-329`.
- **[MVP]** Défaut d'un dossier jamais vu : Icônes, aperçus activés, tri par nom croissant, dossiers en premier, sans regroupement, fichiers cachés masqués. Réf. `settings/dolphin_directoryviewpropertysettings.kcfg`.
- **[v1]** Colonnes du mode Détails : par défaut, `size` et `modificationtime` en plus du nom, tant que l'utilisateur n'a pas personnalisé les colonnes. Un marqueur `CustomizedDetails` distingue « aucune colonne choisie » de « jamais personnalisé ». Réf. `views/viewproperties.cpp:502-545`, `:488-495`.
- **[v1]** Les colonnes visibles sont mémorisées **par mode** dans une seule liste préfixée : `VisibleRoles=Details_size,Details_modificationtime,Icons_size,Compact_…`. Le rôle `text` (le nom) est toujours en tête. Réf. `views/viewproperties.cpp:463-500`, `:734-753`.
- **[v1]** Largeurs de colonnes (`HeaderColumnWidths`) : elles ne sont appliquées que si leur nombre est égal au nombre de colonnes visibles. Sinon, le redimensionnement automatique est utilisé. Marges latérales du mode Détails : 20 px à gauche et 20 px à droite. Réf. `views/dolphinview.cpp:2535-2552`, `settings/dolphin_detailsmodesettings.kcfg`.
- **[Plus tard]** Le mode Détails permet de **déplier les dossiers en arbre** (`ExpandableFolders`, vrai par défaut). Réf. `settings/dolphin_detailsmodesettings.kcfg`, `views/dolphinview.cpp:2909-2914`.
- **[Plus tard]** **Vue dynamique** (`DynamicView`, désactivée par défaut). Après le chargement, Dolphin passe le dossier en Icônes si au moins 2/3 des fichiers sont des images ou des vidéos. Dans ce calcul, un dossier compte pour 1/3 d'élément. Le basculement n'a lieu qu'une fois par dossier (drapeau `DynamicViewPassed`), jamais si un sous-dossier est déplié, et il est abandonné dès que plus d'un tiers des fichiers ne sont pas des médias. Réf. `views/dolphinview.cpp:2585-2650`.

### Tri et regroupement
- **[MVP]** Le tri est **naturel** par défaut (`SortingChoice=NaturalSorting`, collator numérique : `fichier2` avant `fichier10`). Les autres choix sont « insensible à la casse » et « sensible à la casse ». Réf. `kitemviews/kfileitemmodel.cpp:285`, `settings/dolphin_generalsettings.kcfg:164`.
- **[MVP]** Quand on change de critère de tri, l'ordre devient l'ordre **préféré** pour ce critère. Les dates (`modificationtime`, `creationtime`, `accesstime`, `deletiontime`, `imageDateTime`, `releaseYear`), les tailles et dimensions (`size`, `width`, `height`, `pageCount`, `wordCount`, `lineCount`) et `rating`, `duration`, `bitrate`, `frameRate` sont **décroissants** par défaut. Tout le reste est croissant. L'ordre choisi pour un critère est mémorisé pendant la session. Réf. `views/dolphinview.cpp:533-552`, `:1877-1900`, `:2895-2907`.
- **[MVP]** « Dossiers en premier » est vrai par défaut. **[v1]** « Fichiers cachés en dernier » est faux par défaut. Réf. kcfg.
- **[v1]** Le regroupement (`GroupedSorting`) est désactivé par défaut. `GroupRole` vide signifie « grouper selon le critère de tri ». Le menu « Grouper par » propose « Aucun » et « Comme le tri ». Réf. `views/dolphinviewactionhandler.cpp:343-374`.
- **[v1]** Un tri modifié pendant le chargement est appliqué de façon groupée, au plus toutes les 100 ms. Réf. `kitemviews/kfileitemmodel.cpp:331-335`.

## 2. Propriétés de vue par dossier

### Ce qui est mémorisé
**[MVP]** Le schéma comporte deux groupes INI.
- `[Settings]` : `HiddenFilesShown`.
- `[Dolphin]` : `Version` (actuelle : 4), `ViewMode`, `ZoomLevel` (-1 = défaut du mode), `PreviewsShown`, `GroupedSorting`, `SortRole` (`text` par défaut), `GroupRole`, `SortOrder`, `SortFoldersFirst`, `SortHiddenLast`, `VisibleRoles`, `HeaderColumnWidths`, `Timestamp`, `DynamicViewPassed`.

Réf. `settings/dolphin_directoryviewpropertysettings.kcfg`, `views/viewproperties.cpp:26-39`.

Le filtre de nom, la sélection et la position de défilement **ne font pas** partie de ces propriétés. Les deux derniers sont gardés dans l'historique de navigation (`saveState`/`restoreState`). Réf. `views/dolphinview.cpp:1767-1817`.

### Où c'est stocké (ordre de priorité)
1. **[v1]** D'abord dans l'**attribut étendu** `user.kde.fm.viewproperties#1` du dossier (via `KFileMetaData::UserMetaData`), qui contient le texte INI complet. C'est l'emplacement principal quand le système de fichiers le permet. Réf. `views/viewproperties.cpp:31`, `:54-99`, `:622-725`.
2. **[v1]** Sinon, dans le fichier **`.directory`** du dossier. Si ce fichier contient déjà `[Dolphin]` ou `[Settings]`, il est lu en priorité sur l'attribut étendu, ce qui permet de migrer les anciens fichiers. Dolphin ne copie que ses deux groupes. **Les autres groupes, comme `[Desktop Entry] Icon=` (icône personnalisée), sont toujours préservés.** Après une écriture réussie dans l'attribut étendu, Dolphin retire ses groupes du `.directory` et supprime le fichier s'il devient vide. Réf. `views/viewproperties.cpp:62-83`, `:597-614`.
3. **[MVP]** Sinon, ou si le dossier est **hors du `$HOME`**, **lent** (réseau), **non inscriptible**, ou si son `.directory` existe mais n'est pas lisible et inscriptible : stockage dans `~/.local/share/dolphin/view_properties/local/<chemin absolu>/`. Réf. `views/viewproperties.cpp:182-210`, `:727-732`.
4. URL distantes : `…/view_properties/remote/`. Recherches : `…/search/<SHA1 base64 de l'URL, '/'→'-'>`. Corbeille : `…/trash`. Récents et chronologie : `…/recentlyused` et `…/timeline`. Mode global : `…/global`. Réf. `views/viewproperties.cpp:168-217`, `:844-850`.

- **[v1]** Si l'attribut étendu manque de place (`NoSpace`), Dolphin le vide et se rabat sur `.directory`, en ne copiant que ses groupes. Réf. `views/viewproperties.cpp:696-716`.
- **[v1]** **Pas d'écriture inutile.** Si toutes les valeurs sont égales aux défauts globaux, Dolphin **efface** l'attribut étendu et nettoie `.directory` au lieu d'écrire. Exceptions : le fichier global lui-même et les dossiers spéciaux, qui ont leur propre style par défaut. Réf. `views/viewproperties.cpp:633-669`.
- **[MVP]** L'écriture se fait **à chaque modification** : chaque setter de `DolphinView` crée un `ViewProperties`, le modifie, et le destructeur sauvegarde si quelque chose a changé. Réf. `views/viewproperties.cpp:303-317`, `views/dolphinview.cpp:370`, `:400`, `:419`, etc.

### Vue globale et vue par dossier
- **[MVP]** `GlobalViewProps` vaut **vrai par défaut** : un seul jeu de propriétés pour tous les dossiers, stocké dans `…/view_properties/global`. Réf. `settings/dolphin_generalsettings.kcfg:90-92`, `views/viewproperties.cpp:159`, `:180-181`.
- **[MVP]** En mode global, les **dossiers spéciaux** (recherche, corbeille, récents, chronologie, **Téléchargements**) gardent leur propre jeu. Leur style par défaut n'est appliqué qu'à la première visite, ou quand leur timestamp est périmé. Réf. `views/viewproperties.cpp:219-230`.
- **[v1]** En mode global, le **zoom** n'est pas relu par dossier : il reste celui de la vue. Réf. `views/dolphinview.cpp:2517-2529`.
- **[v1]** Quand on passe de « par dossier » à « global », les propriétés du dossier courant sont copiées dans le jeu global. Réf. `settings/viewmodes/generalviewsettingspage.cpp:230-236`.
- Un dossier sans propriétés hérite du **jeu global** (`defaultProperties()`). En mode par dossier, ce jeu sert aussi de modèle pour les dossiers neufs. Réf. `views/viewproperties.cpp:102-121`, `:232-238`.

Styles par défaut des dossiers spéciaux. Réf. `views/viewproperties.cpp:240-279`.

| Dossier | Mode | Colonnes / tri |
|---|---|---|
| Corbeille **[MVP]** | Détails | nom, `path` (emplacement d'origine), `deletiontime` |
| Téléchargements **[v1]** | inchangé | tri par date de modification décroissante, regroupé, sans « dossiers en premier » |
| Récents **[Plus tard]** | Détails | nom, chemin, `accesstime`, tri décroissant, regroupé |
| Recherche **[v1]** | Détails | nom, `path`, `modificationtime` (images et vidéos : Icônes avec aperçus) |

### Horodatage (timestamp)
- **[v1]** Chaque modification met à jour `Timestamp` à l'heure courante. Réf. `views/viewproperties.cpp:587-591`.
- **[v1]** La boîte « Ajuster le style d'affichage » propose trois portées : **ce dossier**, **ce dossier et ses sous-dossiers**, ou **tous les dossiers**. « Tous les dossiers » écrit le jeu global et pose `GeneralSettings::ViewPropsTimestamp = maintenant`. Ensuite, **toutes les propriétés par dossier plus anciennes que ce timestamp sont ignorées** à la lecture, sans être supprimées. C'est une invalidation en O(1). Les deux portées étendues demandent confirmation : « The view properties of all sub-folders/folders will be changed. Do you want to continue? ». La portée sous-dossiers parcourt récursivement l'arborescence (`ApplyViewPropsJob`) avec une fenêtre de progression. La case « Utiliser comme réglages par défaut » écrit seulement le jeu global. Réf. `settings/viewpropertiesdialog.cpp:329-371`, `views/viewproperties.cpp:223-226`.
- **[v1]** « Restaurer les valeurs par défaut » recharge le jeu global dans le dossier courant. L'action n'est active que si la vue diffère des défauts (signal `viewSettingsChanged(isDefaults)`). Réf. `views/viewproperties.cpp:123-150`, `views/dolphinview.cpp:2089-2095`.
- Migration des versions : de 1 à 4, `AdditionalInfo` devient `VisibleRoles`, `name` devient `text` et `date` devient `modificationtime`. Réf. `views/viewproperties.cpp:281-300`.

> **Notes pour Rust/Slint.** Utiliser la crate `xattr` (`user.*`). N'écrire **jamais** dans le dossier observé sans nécessité : par défaut, stocker dans `$XDG_DATA_HOME/satchel/view_properties/` avec une clé = chemin canonique. Optionnellement, lire `user.kde.fm.viewproperties#1` et `.directory [Dolphin]` pour reprendre les réglages Dolphin existants. Si Satchel écrit un jour dans `.directory`, utiliser un parseur INI qui **préserve les groupes inconnus**. Garder le mécanisme du timestamp global pour « appliquer à tous ». Mode global par défaut, comme Dolphin, et un jeu propre à chaque panneau ? À décider dans la spec des panneaux.

## 3. Zoom

- **[MVP]** 17 niveaux, de 0 à 16. Taille d'icône : 0 → 16 px, 1 → 22, 2 → 32, 3 → 48, 4 → 64, puis `64 + 16 × (niveau − 4)`, soit 256 px au niveau 16. Réf. `views/zoomlevelinfo.cpp:13-71`.
- **[MVP]** Le niveau par défaut dépend du mode et des aperçus :

| Mode | Sans aperçu | Avec aperçus |
|---|---|---|
| Icônes | 32 px | 64 px |
| Compact | 16 px | 48 px |
| Détails | 16 px | 48 px |

  Activer ou désactiver les aperçus change donc le zoom. Réf. `settings/dolphin_*modesettings.kcfg`, `views/dolphinview.cpp:2559-2565`, `:364-383`.
- **[MVP]** `Ctrl+molette` : les deltas s'accumulent et un cran de 120 vaut un niveau. L'accumulateur est remis à zéro au relâchement de Ctrl. `Ctrl++` / `Ctrl+-` (actions standard KDE), `Ctrl+0` pour revenir au défaut. Le geste de pincement émet aussi `increaseZoom`/`decreaseZoom`. Réf. `views/dolphinview.cpp:1173-1190`, `:1113-1115`, `views/dolphinviewactionhandler.cpp:255-270`.
- **[v1]** Le zoom stocké vaut **-1 quand il est égal au défaut**, pour que le dossier suive un changement ultérieur du défaut. Réf. `views/dolphinview.cpp:518-519`.
- **[Plus tard]** Curseur de zoom dans la barre d'état (`ShowZoomSlider`, faux par défaut).

## 4. Fichiers cachés et filtrage

- **[MVP]** Sont cachés : les noms commençant par `.`, ainsi que les noms listés dans un fichier **`.hidden`** du dossier (convention freedesktop, gérée par KDirLister). Les éléments cachés affichés sont **semi-transparents**. Réf. `views/dolphinviewactionhandler.cpp:376-391`.
- **[v1]** Option `HideXTrashFile` (faux par défaut) : masque aussi les fichiers de type MIME `application/x-trash` (`*~`, `*.bak`, `*.old`…) tant que les cachés ne sont pas affichés. Réf. `kitemviews/kfileitemmodel.cpp:339`, `:480-505`.
- **[MVP]** Bascule : `Ctrl+H` (ainsi que `Alt+.`, raccourcis standard KDE). L'état est **mémorisé par dossier**, ou globalement en mode global. La **sélection est conservée** pendant la bascule. La barre d'adresse suit le même réglage pour ses sous-menus de dossiers. Réf. `views/dolphinview.cpp:390-406`, `dolphinviewcontainer.cpp:1082-1087`.
- **[MVP]** **Barre de filtre** (non persistée par dossier). Le motif porte sur le nom affiché, en trois modes : **texte simple** (sous-chaîne, défaut), **glob** (joker non ancré) et **regex**. Option de sensibilité à la casse. Un motif invalide masque tout. Le filtre MIME, s'il existe, se combine en ET. Si rien ne correspond, le message affiché est « No items matching the filter ». Réf. `kitemviews/private/kfileitemmodelfilter.cpp:60-170`, `views/dolphinview.cpp:2828-2829`.
- **[v1]** La barre de filtre peut être **verrouillée** : elle est vidée au changement de dossier sauf si elle est verrouillée (`clearIfUnlocked`). Réf. `dolphinviewcontainer.cpp:859-862`.
- **[MVP]** Recherche en tapant : taper des lettres saute au premier élément correspondant. Une bulle montre la saisie et la complétion en grisé, ou la saisie en rouge si rien ne correspond. Réf. `views/dolphinview.cpp:134-152`.

## 5. Surveillance du dossier (KDirLister / KDirWatch)

- **[MVP]** Le dossier affiché est surveillé en continu. KDirLister reçoit les ajouts, suppressions et modifications (`itemsAdded`, `itemsDeleted`, `refreshItems`) par inotify en local et par `org.kde.KDirNotify` (D-Bus) pour ce que font KIO et les autres applications KDE. Réf. `kitemviews/kfileitemmodel.cpp:287-303`.
- **[MVP]** Pendant un listage lent, les éléments sont insérés **par lots, au moins toutes les 2 s**, sans attendre la fin. Réf. `kitemviews/kfileitemmodel.cpp:320-325`.
- **[MVP]** **Le dossier courant est supprimé ou renommé** :
  - en local, Dolphin remonte au **premier ancêtre existant**, de façon différée d'un tour de boucle ;
  - il affiche un avertissement : « Current location changed, *chemin* is no longer accessible. » ;
  - en distant, l'avertissement seul s'affiche.
  
  Réf. `kitemviews/kfileitemmodel.cpp:1531-1537`, `dolphinviewcontainer.cpp:1096-1110`.
- **[v1]** Redirections : si l'URL est redirigée (par exemple un lien `.desktop` ou un `smb`), l'URL de la vue est mise à jour avant l'émission du signal. Réf. `views/dolphinview.cpp:1998-2006`.
- **[MVP]** `F5` recharge en **préservant** la sélection, l'élément courant et le défilement. Réf. `views/dolphinview.cpp:648-659`.
- **[MVP]** Placeholders :
  - « Loading… » s'affiche seulement si le chargement dure plus de **500 ms** ;
  - pour un dossier vide : « Folder is empty », ou « Trash is empty » pour `trash:/` ;
  - après une annulation par `Échap` : « Loading canceled ».
  
  Réf. `views/dolphinview.cpp:171-174`, `:2796-2870`, `:211`.
- **[MVP]** L'état « dossier inscriptible » est **désactivé pendant le chargement** puis recalculé à la fin. Il active ou désactive « Créer un dossier », « Créer un fichier » et « Coller ». Réf. `views/dolphinview.cpp:2192-2223`, `:2701-2720`, `views/dolphinviewactionhandler.cpp:770-776`.

> **Notes pour Rust/Slint.** Utiliser `notify` (inotify) avec `notify-debouncer-full` (~100 ms). Après chaque opération faite par Satchel, injecter aussi un événement interne immédiat, sans attendre inotify, pour que les **deux panneaux** se mettent à jour. Les événements inotify `IN_DELETE_SELF` et `IN_MOVE_SELF` sur le dossier observé déclenchent la remontée à l'ancêtre. Attention à la limite `max_user_watches` : surveiller seulement les dossiers affichés, pas récursivement. Les points de montage FUSE et réseau ne remontent pas d'inotify fiable : prévoir une relecture à la reprise du focus.

## 6. Ouverture des fichiers

- **[MVP]** **Simple ou double clic** : Dolphin suit le réglage global de KDE (`SH_ItemView_ActivateItemOnSingleClick`, issu de `kdeglobals [KDE] SingleClick`). Le défaut de Plasma 6 est le double clic. Réf. `kitemviews/kitemlistcontroller.cpp:738-744`, `:1660-1690`.
- **[MVP]** **Entrée** : si plusieurs éléments sont sélectionnés, tous sont activés. Si un seul est sélectionné, il est activé. Sans sélection, l'élément courant est activé. Réf. `kitemviews/kitemlistcontroller.cpp:433-444`.
- **[MVP]** **Alt + activation** ouvre la boîte de propriétés au lieu d'ouvrir l'élément. Réf. `views/dolphinview.cpp:1222-1226`.
- **[MVP]** Un **dossier** s'ouvre dans la vue. Avec Ctrl, il s'ouvre dans un nouvel onglet ; avec Ctrl+Maj, dans un nouvel onglet activé ; avec Maj, dans une nouvelle fenêtre. Réf. `dolphinviewcontainer.cpp:765-779`.
- **[MVP]** Un **fichier** est confié à `KIO::OpenUrlJob(targetUrl, mimetype)`, qui choisit l'application par défaut selon `mimeapps.list`. L'option `setShowOpenOrExecuteDialog(true)` est activée. En cas d'erreur, le message s'affiche dans la bande de la vue. Réf. `dolphinviewcontainer.cpp:781-786`, `:1112-1117`.
- **[MVP]** **Exécutables** (KIO, inféré) :
  - un **script** texte exécutable et local déclenche le dialogue « Ouvrir / Exécuter » selon `kiorc [Executable scripts] behaviourOnLaunch` : `alwaysAsk` (défaut), `open` ou `execute` ;
  - un binaire n'est lancé que si le bit `x` est présent. Sinon il est ouvert comme fichier, ou KIO propose de le rendre exécutable (à vérifier) ;
  - un `.desktop` non fiable (sans bit `x` hors des emplacements système) produit un avertissement.
  
  Réf. `settings/interface/confirmationssettingspage.cpp:29`, `:140-152`, `dolphinviewcontainer.cpp:799-810`.
- **[v1]** Un `.desktop` de `Type=Link` vers une URL non-http **navigue** vers la cible au lieu de l'ouvrir. Réf. `views/dolphinview.cpp:1862-1871`.
- **[v1]** **Archives** : `BrowseThroughArchives` vaut faux par défaut, donc les archives s'ouvrent avec l'application associée. Si l'option est activée, une archive locale s'ouvre comme un dossier (`zip:/`, `tar:/`…). Les documents OpenDocument ne sont pas concernés : pas d'héritage de type MIME. Réf. `views/dolphinview.cpp:1834-1875`.
- **[v1]** Activer **plus de 5 éléments** à la fois demande une confirmation : « Are you sure you want to open %1 items? ». Une case « ne plus demander » est disponible (clé `ConfirmOpenManyFolders`). Dans une activation multiple, les dossiers s'ouvrent en **onglets** (Maj seul : fenêtres) et les fichiers avec leur application préférée (`runPreferredApplications`). Réf. `views/dolphinview.cpp:1234-1282`, `dolphinviewcontainer.cpp:835-841`.
- **[v1]** **Clic du milieu** : un dossier ou une archive s'ouvre dans un nouvel onglet (Maj : onglet activé). Un fichier s'ouvre avec la **2ᵉ** application associée (Maj : la **3ᵉ**). Pour les scripts exécutables, le décalage est réduit d'un cran. Le menu « Ouvrir avec » affiche les indications « Middle Click » et « Shift+Middle Click ». Réf. `views/dolphinview.cpp:1284-1306`, `dolphinviewcontainer.cpp:789-833`, `dolphincontextmenu.cpp:500-528`.
- **[v1]** Boutons Précédent et Suivant de la souris : navigation dans l'historique. Réf. `views/dolphinview.cpp:1582-1593`.
- **[Plus tard]** Double clic sur le fond : action configurable (`DoubleClickViewAction`, défaut `edit_select_all`) ou commande personnalisée avec `{path}`. Réf. `dolphinmainwindow.cpp:3195-3215`.

> **Notes pour Rust/Slint.** N'utilisez pas `open::that` (qui passe par `xdg-open`) pour le MVP : le comportement varie selon le bureau. Le mieux est de résoudre soi-même l'application :
> - type MIME avec `xdg-mime` (globs et magic de shared-mime-info) ;
> - défaut d'après `mimeapps.list`, dans l'ordre de la spec : `$XDG_CONFIG_HOME/kde-mimeapps.list`, puis `mimeapps.list`, puis les dossiers système `[Default Applications]`, et enfin le cache `mimeinfo.cache` ;
> - lancement avec `freedesktop-desktop-entry` (développer `%f %F %u %U`).
>
> Plan B : `gio::AppInfo::default_for_type` via la crate `gio`. Ne jamais exécuter un fichier sans confirmation explicite.

## 7. Presse-papiers : copier, couper, coller

- **[MVP]** Copier (`Ctrl+C`) et Couper (`Ctrl+X`) placent la sélection dans le presse-papiers par `KUrlMimeData::setUrls(urls, mostLocalUrls)`. Les éléments dont un parent est déjà sélectionné (arbre déplié) sont exclus. Réf. `views/dolphinview.cpp:933-946`, `kitemviews/kfileitemmodel.cpp:526-562`.
- **[MVP]** Types MIME (KIO, inféré) :
  - `text/uri-list` avec les URL « les plus locales » (`file://…`) ;
  - `application/x-kde4-urilist` avec les URL KDE d'origine, si elles diffèrent (`trash:/`, `desktop:/`…) ;
  - pour couper, `application/x-kde-cutselection` = `"1"` (posé par `KIO::setClipboardDataCut`) ;
  - en plus, `KUrlMimeData::exportUrlsToPortal` ajoute une entrée pour le portail de transfert de fichiers (applications sandboxées).
  
  Réf. `views/dolphinview.cpp:936-937`.
- **[MVP]** Coller (`Ctrl+V`) appelle `KIO::paste(mimeData, url)`. Le libellé est dynamique (« Coller un fichier », « Coller %1 éléments », « Coller le contenu du presse-papiers… »). L'action est **désactivée** si le presse-papiers est vide ou si le dossier n'est pas inscriptible. Une infobulle donne alors la raison : « Cannot paste: The clipboard is empty. » ou « …You do not have permission to write into this folder. ». L'état est recalculé à chaque changement du presse-papiers. Réf. `views/dolphinview.cpp:1734-1740`, `:2652-2673`, `dolphinmainwindow.cpp:217`, `:941-951`.
- **[MVP]** **Après un collage, les éléments collés sont sélectionnés.** L'ancienne sélection est vidée et le premier élément créé devient l'élément courant, avec défilement jusqu'à lui. Les URL sont collectées depuis `copying`/`moving`/`linking`/`itemCreated`. Celles qui ne sont pas encore dans le modèle sont sélectionnées au prochain `directoryLoadingCompleted`. Le même mécanisme s'applique au dépôt, à la duplication et à « Copier/Déplacer vers l'autre vue ». Réf. `views/dolphinview.cpp:1607-1659`, `:2008-2044`.
- **[MVP]** « **Coller dans le dossier** » : si un seul dossier est sélectionné, le menu contextuel colle **dans** ce dossier. Réf. `views/dolphinview.cpp:997-1003`, `dolphincontextmenu.cpp:445-471`.
- **[MVP]** Coller-couper (KIO, inféré) : un déplacement suivi du **vidage du presse-papiers**, qui empêche un second collage. Si le presse-papiers ne contient **que des données** (texte, image), KIO demande un nom de fichier et un format, puis crée le fichier. Cette création est annulable.
- **[v1]** « Copier l'emplacement » (`Ctrl+Alt+C`) copie le chemin local du **premier** élément sélectionné en texte brut, ou l'URL s'il n'y a pas de chemin local. Réf. `views/dolphinview.cpp:2763-2779`.
- **[MVP]** Panneaux jumeaux : **Copier vers l'autre vue** (`Maj+F5`) et **Déplacer vers l'autre vue** (`Maj+F6`) lancent `KIO::copy` ou `KIO::move` vers l'URL de la vue inactive, **sans confirmation**. Les éléments arrivés sont sélectionnés dans la vue cible. `Ctrl+F3` donne le focus à l'autre vue. Réf. `dolphintabwidget.cpp:354-380`, `views/dolphinview.cpp:948-990`, `dolphinmainwindow.cpp:1882-1910`.
- **[Plus tard]** Sous-menus « Copier vers » et « Déplacer vers » (`ShowCopyMoveMenu`, faux par défaut). Réf. `dolphincontextmenu.cpp:316-322`.

> **Notes pour Rust/Slint.** Le presse-papiers de Slint ne gère que le texte. Sur Plasma Wayland, utiliser `wl-clipboard-rs` (types MIME arbitraires), avec repli sur X11 par `x11-clipboard`. Publier :
> - `text/uri-list` (CRLF, URL percent-encodées) ;
> - `application/x-kde-cutselection` (`1`) quand on coupe ;
> - `x-special/gnome-copied-files` (`cut\n` ou `copy\n` suivi des URL) pour Nautilus et les applications GTK ;
> - `text/plain` (chemins, un par ligne) pour les terminaux et éditeurs.
>
> En lecture, accepter les quatre. Le nom réel du type MIME KDE est `application/x-kde-cutselection`, sans tiret entre *cut* et *selection*.

## 8. Supprimer et mettre à la corbeille

- **[MVP]** `Suppr` met à la corbeille. `Maj+Suppr` supprime définitivement. **La répétition automatique est désactivée** sur ces deux actions : garder la touche enfoncée ne supprime pas une série d'éléments. Réf. `views/dolphinviewactionhandler.cpp:116-139`.
- **[MVP]** Pour un élément **non local**, ou qui ne peut pas aller à la corbeille, l'action « Corbeille » est désactivée. Une action cachée `delete_shortcut` reprend alors `Suppr` pour **supprimer définitivement**, avec confirmation. Réf. `views/dolphinviewactionhandler.cpp:141-150`.
- **[MVP]** Les deux opérations passent par `KIO::DeleteOrTrashJob(urls, Trash|Delete, DefaultConfirmation)`. La liste est simplifiée : un descendant d'un élément déjà sélectionné est retiré. Réf. `views/dolphinview.cpp:907-931`, `:2675-2691`.
- **[MVP]** **Confirmations** (fichier `kiorc [Confirmations]`, partagé avec tout KDE) :

| Clé | Défaut |
|---|---|
| `ConfirmTrash` | **faux** : mettre à la corbeille ne demande rien |
| `ConfirmDelete` | **vrai** |
| `ConfirmEmptyTrash` | **vrai** |

  Réf. `settings/interface/confirmationssettingspage.cpp:26-28`, `:133-139`. Le dialogue de KIO (inféré) liste les éléments, indique « Cette action est irréversible » pour la suppression et propose une case « Ne plus demander ».
- **[MVP]** Après le succès de l'opération :
  - si la vue est active, l'élément **qui suit** le dernier élément sélectionné est sélectionné et devient l'élément courant ;
  - la barre d'état affiche « Trash operation completed. » ou « Delete operation completed. » ;
  - l'annulation par l'utilisateur n'affiche rien ;
  - les autres erreurs vont dans la bande de messages.
  
  Réf. `views/dolphinview.cpp:2131-2171`.
- **[MVP]** **Maj pendant que le menu contextuel est ouvert** bascule en direct l'entrée « Mettre à la corbeille » en « Supprimer », et inversement. Si `kdeglobals [KDE] ShowDeleteCommand=true`, ou si la sélection n'est pas locale, les deux entrées sont affichées séparément. Réf. `dolphinremoveaction.cpp:27-67`, `dolphincontextmenu.cpp:103-120`, `:417-434`.
- **[MVP]** **Autre système de fichiers** (spec Trash freedesktop, KIO inféré) :
  - sur la partition du `$HOME`, la corbeille est `~/.local/share/Trash/{files,info}` ;
  - ailleurs, elle est `$topdir/.Trash/$uid` (si `.Trash` existe, a le bit sticky et n'est pas un lien symbolique), sinon `$topdir/.Trash-$uid` ;
  - **jamais de copie entre périphériques vers la corbeille du home** ;
  - si aucune corbeille n'est possible (système de fichiers en lecture seule, montage réseau, FAT sans droits), KIO **propose de supprimer définitivement** à la place (`AskUserActionInterface::DeleteInsteadOfTrash`).
- **[v1]** **Corbeille pleine** (`kcm_trash`, fichier `ktrashrc`, par partition, inféré) :
  - limite de taille en % de la partition (10 % par défaut) ;
  - limite d'âge optionnelle ;
  - action quand la limite est atteinte : avertir et refuser, supprimer les plus anciens, ou supprimer les plus gros ;
  - un fichier plus gros que la limite déclenche la proposition de suppression définitive.
  
  Dolphin intègre ce module de réglages tel quel. Réf. `settings/trash/trashsettingspage.cpp:15-30`.
- **[MVP]** **Vue de la corbeille** :
  - menu du fond : Trier, Mode, **Vider la corbeille** (désactivé si elle est vide, état suivi en direct) et Configurer la corbeille ;
  - menu d'un élément : **Restaurer à l'emplacement d'origine** (au pluriel selon le nombre), Couper, Copier, **Supprimer** (définitivement) et Propriétés ;
  - **pas** de Renommer ni de Corbeille.
  
  Réf. `dolphincontextmenu.cpp:122-199`.
- **[MVP]** Vider la corbeille demande confirmation (`ConfirmEmptyTrash`), puis envoie une notification « Trash Emptied ». La corbeille vide est lue dans `trashrc [Status] Empty`. Le changement est suivi par un `KDirLister` sur `trash:/`, rafraîchi quand un support amovible est monté ou retiré. Réf. `trash/dolphintrash.cpp:22-104`.
- **[MVP]** Restauration (KIO, inféré) : l'élément est déplacé vers `Path=` de son `.trashinfo`. Il y a erreur si **le dossier d'origine n'existe plus** (« …recréez ce dossier ou faites glisser l'élément ailleurs ») ou si **un élément porte déjà ce nom** à destination. Glisser hors de la corbeille vaut restauration vers un autre endroit.

> **Notes pour Rust/Slint.** La crate `trash` (`delete_all`, `os_limited::{list, restore_all, purge_all}`) implémente la spec freedesktop, y compris les corbeilles `$topdir`. Vérifier son comportement sur les montages et **refuser explicitement** les copies entre périphériques. Les limites de taille de `ktrashrc` ne sont pas gérées par la crate : soit les lire et les appliquer (v1), soit avertir. Écrire `.trashinfo` **avant** de déplacer le fichier, et mettre à jour `directorysizes`. Lire et écrire `kiorc [Confirmations]` permet de partager les préférences avec Dolphin.

## 9. Renommer

- **[MVP]** `F2`. **Un seul élément** avec `RenameInline=true` (défaut) : édition en place, après défilement jusqu'à l'élément. **Plusieurs éléments**, ou mode en place désactivé : dialogue `KIO::RenameFileDialog`. Réf. `views/dolphinview.cpp:854-905`, `settings/dolphin_generalsettings.kcfg:106`.
- **[MVP]** **Sélection initiale** du nom : tout le nom pour un dossier. Pour un fichier, le nom sans l'extension connue du type MIME (`.tar.gz` est reconnu). Si l'extension est inconnue, la sélection s'arrête avant le dernier `.`, sauf si ce point est le premier ou le dernier caractère. Réf. `kitemviews/kfileitemlistwidget.cpp:161-185`.
- **[MVP]** Touches de l'éditeur :
  - `Entrée` valide, `Échap` annule ;
  - **la perte de focus valide**, sauf si elle est due à un menu ;
  - `Tab` / `Maj+Tab` valident puis passent au renommage de l'élément suivant ou précédent ;
  - `Haut` / `Bas` font de même si l'enchaînement est autorisé ;
  - `Gauche` / `Droite` réduisent la sélection à son début ou à sa fin ;
  - `Début` / `Fin` (avec Maj pour étendre la sélection).
  
  Réf. `kitemviews/private/kitemlistroleeditor.cpp:55-120`, `views/dolphinview.cpp:2392-2397`.
- **[MVP]** **Renommage en deux clics** : un clic sur le *texte* d'un élément déjà seul sélectionné ouvre l'édition après `doubleClickInterval`. Uniquement en mode double clic, si l'élément est déplaçable. Le renommage est annulé par un glisser, une perte de focus de la fenêtre ou un menu contextuel. Réf. `views/dolphinview.cpp:1595-1605`, `:2115-2129`, `:1090-1094`, `:1129-1133`.
- **[MVP]** Validation :
  - un nom vide, identique, `.` ou `..` est **ignoré** sans message ;
  - le nom est passé par `KIO::encodeFileName`, qui remplace **`/` par `∕` (U+2215)** au lieu de refuser (KIO, inféré) ;
  - ensuite : `KIO::moveAs(old, new)`, enregistré dans l'annulation sous le type `Rename`, avec gestion d'erreurs automatique.
  
  Réf. `views/dolphinview.cpp:2304-2308`, `:2378-2381`.
- **[MVP]** **Confirmation « masquer »** : si les cachés ne sont pas affichés et que le nouveau nom commence par `.` alors que l'ancien non, Dolphin demande « Adding a dot to the beginning of this file's/folder's name will hide it from view. » avec les boutons « Rename and Hide » et Annuler. Une case « ne plus demander » est disponible (`ConfirmHide`). Réf. `views/dolphinview.cpp:2310-2329`.
- **[v1]** **Confirmation « changement de type »** (`ConfirmRenameFileType`, vrai par défaut). Conditions : fichier local, non lent, de type connu. Le nouveau type est deviné **à partir du nom et du contenu**. S'il diffère du type actuel, Dolphin affiche « This will change the file type from "%1" to "%2"… » ou « This will make the file type unknown… », avec l'icône du nouveau type et une case « Do not ask again ». Réf. `views/dolphinview.cpp:2331-2376`.
- **[MVP]** Si le nom cible existe déjà, `CopyJob` ouvre le dialogue de conflit (§12). Le modèle n'est mis à jour que sur succès, et seulement s'il n'y avait pas déjà d'élément à ce nom. Réf. `views/dolphinview.cpp:2173-2190`, `:2383-2390`.
- **[v1]** **Renommage par lot** (`RenameFileDialog`, KIO, inféré) :
  - champ de modèle avec `#` comme espace réservé : `#` donne 1, 2, 3 ; `###` donne 001, 002 ;
  - compteur de départ à 1 par défaut ;
  - **l'extension de chaque fichier est conservée** ;
  - le bouton OK est désactivé sans `#` ;
  - l'opération est annulable en bloc (`BatchRename`).
  
  À la fin, Dolphin met à jour les noms dans le modèle et **sélectionne les éléments renommés**. Une erreur s'affiche dans une boîte modale. Réf. `views/dolphinview.cpp:879-898`, `views/dolphinviewactionhandler.cpp:110-114` (« Renaming multiple items at once results in their new names differing only in a number »).
- **[Plus tard]** Renommage sensible à la casse uniquement sur un système de fichiers insensible à la casse : KIO passe par un nom temporaire (inféré).

## 10. Créer : dossier, fichier, lien, doublon

- **[MVP]** **Nouveau dossier** : `F10` (`KStandardShortcut::createFolder`). Actif si le dossier est inscriptible et que le protocole sait créer des dossiers. **Nouveau sous-dossier** : crée dans le dossier sélectionné s'il y en a exactement un et qu'il est inscriptible. Un seul dialogue de création à la fois : un nouvel appui est ignoré tant que le précédent est ouvert. Réf. `views/dolphinviewactionhandler.cpp:85-106`, `:978-997`, `dolphinmainwindow.cpp:829-847`.
- **[MVP]** Dialogue de nom (KIO `KNewFileMenu`, inféré) :
  - nom proposé « Nouveau dossier », déjà sélectionné ;
  - un nom déjà pris désactive OK et affiche un message ;
  - un nom commençant par `.` déclenche l'avertissement « sera caché » ;
  - un nom contenant `/` crée la **hiérarchie** (mkpath) ;
  - `~` en tête produit un avertissement.
  
  Dolphin ne fait que relayer les erreurs vers la bande de messages, en ignorant l'annulation. Réf. `dolphinnewfilemenu.cpp:28-35`.
- **[MVP]** **Sélection après création** : l'élément créé est sélectionné et devient courant dès qu'il apparaît dans le modèle. Conditions : la vue est active, l'utilisateur n'a pas changé la sélection entre-temps et la vue est encore dans le dossier parent. Pour une hiérarchie `a/b/c`, c'est `a` qui est sélectionné. Créé via le menu d'un dossier en vue Détails, le dossier parent est **déplié** jusqu'au nouvel élément. Réf. `views/dolphinview.cpp:1910-1996`, `dolphincontextmenu.cpp:219-238`.
- **[v1]** **Nouveau fichier** : sous-menu « Créer nouveau » rempli à partir des modèles. Fichier texte, HTML, vide, etc. viennent des `.desktop` de `~/.local/share/templates` et `/usr/share/templates` (KIO, inféré). Il contient aussi « Lien vers un fichier ou un dossier… », qui crée un **lien symbolique**, et « Lien vers un emplacement (URL)… », qui crée un `.desktop Type=Link`. La création est annulable. Réf. `dolphincontextmenu.cpp:345-349`.
- **[MVP]** **Dupliquer ici** (`Ctrl+D`) :
  - chaque élément est copié en `"<nom> copy.<ext>"`. L'extension est déterminée par le type MIME, le `.tar.gz` est donc conservé. Sans extension connue, on obtient `"<nom> copy"`. La chaîne est traduite ; en français, vérifier le `.po` (« %1 copie » ?) ;
  - `setAutoRename(true)` : si le nom est pris, KIO ajoute un suffixe numéroté, **jamais d'écrasement** ;
  - les doublons sont sélectionnés et l'opération est annulable.
  
  Réf. `views/dolphinview.cpp:1005-1055`, `views/dolphinviewactionhandler.cpp:152-157`.
- **[v1]** **Lien** : on en crée un par « Lier ici » au dépôt (§11) ou par le modèle. Pour un **lien symbolique** sélectionné, l'action « Afficher la cible » (`show_target`) est proposée dans le menu contextuel. Réf. `dolphincontextmenu.cpp:289-292`.

> **Notes pour Rust/Slint.** `std::fs::create_dir` (pas `create_dir_all`, sauf si le nom contient `/` et que l'utilisateur l'a voulu). Modèles : lire `XDG_TEMPLATES_DIR` (`~/Modèles`, fichiers bruts façon GNOME) **et** les `.desktop` KDE. Liens : `std::os::unix::fs::symlink`. Pour dupliquer, construire le nom selon le motif ` copy` puis ` copy (2)`. Le format exact de KIO (`KFileUtils::suggestName`) est à vérifier.

## 11. Glisser-déposer

- **[MVP]** **Cible du dépôt** : un dossier, un `.desktop` ou un **exécutable local** reçoivent le dépôt eux-mêmes. Tout autre élément, y compris **une archive**, renvoie le dépôt **au dossier de la vue**. Déposer sur une archive n'ajoute donc **pas** le fichier à l'archive. Réf. `views/dolphinview.cpp:1509-1527`, `views/draganddrophelper.cpp:64-67`.
- **[MVP]** Le survol est accepté sur un dossier **inscriptible**, un `.desktop` ou un exécutable local. Il est **refusé si la cible fait partie des éléments glissés** : sur soi-même, l'action est forcée à `IgnoreAction` (avec un cache par URL de destination, vidé à chaque nouveau glisser). Réf. `views/draganddrophelper.cpp:20-35`, `:51-53`, `:69-83`.
- **[MVP]** Action proposée par défaut à l'**émission** du glisser : `CopyAction`. Actions permises : Move, Copy, Link. Réf. `kitemviews/kitemlistcontroller.cpp:1451`.
- **[MVP]** **Choix de l'action au dépôt** (`KIO::drop` / `DropJob`, KIO, inféré) :
  - `Maj` déplace, `Ctrl` copie, `Ctrl+Maj` crée un lien ;
  - **sans modificateur**, un menu s'ouvre au point de dépôt : « Déplacer ici (Maj) », « Copier ici (Ctrl) », « Lier ici (Ctrl+Maj) », puis les actions de greffons (par exemple Ark « Extraire ici » et « Compresser »), puis « Annuler (Échap) ». Les entrées impossibles sont désactivées (pas de Déplacer si la source n'est pas supprimable) ;
  - **déposer sur la corbeille** déplace vers la corbeille sans menu ;
  - **Dolphin ne fait aucune distinction « même périphérique / autre périphérique »** pour le choix par défaut. Le périphérique n'influe que sur la réalisation : `rename(2)` atomique sur le même système de fichiers, copie puis suppression sinon.
  
  À vérifier dans `kio/src/widgets/dropjob.cpp` : l'existence d'une option globale de déplacement automatique.
- **[MVP]** Dépôt sur un **`.desktop` d'application** : l'application est lancée avec les URL. Sur un **exécutable** : il est lancé avec les fichiers en arguments. Sur un `.desktop Type=Link` vers un dossier : dépôt dans la cible (KIO, inféré).
- **[v1]** Dépôt de **données sans URL** (texte ou image depuis un navigateur) : KIO demande un nom de fichier, comme pour un collage de données (inféré).
- **[v1]** **Depuis Ark** (types MIME `application/x-kde-ark-dndextract-service` et `…-path`) : Dolphin n'utilise pas KIO. Il appelle Ark par D-Bus (`org.kde.ark.DndExtract.extractSelectedFilesTo(dest)`). Réf. `views/draganddrophelper.cpp:37-50`, `:90-93`.
- **[v1]** Un dépôt venant du **panneau Emplacements** exclut les actions de greffons (`ExcludePluginsActions`), jugées potentiellement destructrices. Réf. `views/dolphinview.cpp:1531-1536`.
- **[MVP]** Un dépôt **dans le dossier de la vue** sélectionne les éléments arrivés, comme un collage. Réf. `views/dolphinview.cpp:1543-1561`.
- **[v1]** Survol prolongé d'un dossier pendant un glisser : il s'ouvre après **750 ms** si `AutoExpandFolders` est activé (faux par défaut). Réf. `kitemviews/kitemlistcontroller.cpp:65-68`, `views/dolphinview.cpp:131`.
- **[MVP]** Le glisser démarre après `QApplication::startDragDistance()`. Réf. `kitemviews/kitemlistcontroller.cpp:637-648`.

> **Notes pour Rust/Slint.** Slint ne gère pas encore le glisser-déposer **avec le système** (à vérifier pour la version utilisée). Il faudra probablement passer par winit (dépôt de fichiers seulement) ou par du code Wayland natif (`wl_data_device`, offre `text/uri-list`). Le glisser-déposer interne entre les deux panneaux peut être fait en Slint pur pour le MVP. Satchel peut proposer un réglage façon ForkLift/Finder : déplacer par défaut sur le même système de fichiers (comparer `st_dev`), copier sinon. Garder le menu sans modificateur comme défaut compatible KDE.

## 12. Conflits de copie et de déplacement (tout vient de KIO)

Dolphin lance toujours `KIO::copy`, `KIO::move` ou `KIO::moveAs` avec `KIO::DefaultFlags` : **jamais `Overwrite`**. Le dialogue de conflit vient donc entièrement de `KIO::RenameDialog` / `SkipDialog` via le délégué d'interface du job. Réf. `views/dolphinview.cpp:958`, `:980`, `:2378`. Comportement à réimplémenter (inféré) :

- **[MVP]** **Fichier contre fichier** : le dialogue compare la source et la destination (aperçu, taille, date de modification) et indique laquelle est la plus récente. Boutons :
  - **Écraser** ;
  - **Écraser si plus ancien** (multi-éléments) ;
  - **Ignorer** ;
  - **Renommer**, avec un champ de nouveau nom et un bouton « Suggérer un nouveau nom » ;
  - **Annuler**.
  
  La case **« Appliquer à tous »** transforme Ignorer en « Ignorer tout », Renommer en « Renommer automatiquement » et Écraser en « Écraser tout ». Ces choix « tout » sont mémorisés pour le reste du job.
- **[MVP]** **Dossier contre dossier** : le bouton principal est « **Fusionner** » (*Write Into*), qui fusionne le contenu. Les conflits de fichiers internes sont posés un par un ou selon « Appliquer à tous ». **Fichier contre dossier**, ou l'inverse : pas d'écrasement possible, seulement Renommer, Ignorer ou Annuler.
- **[MVP]** **Renommage automatique** (`suggestName`, inféré) : `nom (1).ext`, puis `(2)`… Un numéro existant est incrémenté. L'extension et le `.tar.gz` sont préservés.
- **[MVP]** **Source identique à la destination** (copier un fichier sur lui-même dans le même dossier) : erreur, ou proposition de renommer. **Déplacer un dossier dans lui-même ou dans un descendant** : erreur « impossible de déplacer un dossier dans lui-même » (`ERR_CANNOT_MOVE_INTO_ITSELF`, nom à vérifier).
- **[MVP]** **Erreurs par élément** (`SkipDialog`) : permission refusée, fichier illisible, etc. Les choix sont Ignorer, Ignorer tout, Réessayer et Annuler. Les avertissements non bloquants du `CopyJob` arrivent dans la bande de messages de Dolphin. Réf. `views/dolphinview.cpp:964-966`, `:986-988`.
- **[MVP]** **Déplacement entre systèmes de fichiers** : KIO copie l'élément, puis supprime la source **seulement si la copie a réussi**. Une copie interrompue supprime le fichier partiel de destination. Les métadonnées (date de modification, permissions) sont préservées. KIO prévient si le volume de destination n'a pas assez de place.
- **[v1]** Barre de progression et notification globales (`KUiServerV2JobTracker`, centre de notifications Plasma), avec pause et annulation. Dolphin n'affiche pas de progression propre pour les copies.

> **Notes pour Rust/Slint.** Moteur de copie maison : `std::fs::copy` (qui utilise `copy_file_range` sur Linux), découpé pour la progression et l'annulation. Ou bien `fs_extra` (moins de contrôle).
> - Toujours `rename(2)` d'abord. Si l'erreur est `EXDEV`, faire copie puis `fsync` puis suppression.
> - Écrire dans `nom.satchel-part` puis faire `rename` : un échec ne laisse ainsi jamais de fichier tronqué sous le vrai nom.
> - Le dialogue de conflit est une fenêtre Slint qui reçoit une `ConflictQuestion` par canal, et le moteur attend la réponse. La politique « appliquer à tous » est stockée dans le job.
> - Pour la fusion de dossiers, utiliser `renameat2(RENAME_NOREPLACE)` (crate `rustix`) afin d'éviter les situations de concurrence (TOCTOU).

## 13. Annulation (undo)

- **[MVP]** `Ctrl+Z` annule, `Ctrl+Maj+Z` rétablit (redo). Le gestionnaire est **global à l'application** : `KIO::FileUndoManager`, une pile partagée par toutes les vues. Le libellé de l'action est dynamique (« Annuler : Renommer »…). Les actions sont désactivées au départ. Réf. `dolphinmainwindow.cpp:157-163`, `:860-902`, `:1841-1842`, `:2122-2140`.
- **[MVP]** **Opérations annulables** et leur effet à l'annulation :

| Opération | Annulation |
|---|---|
| Copier, coller-copier, Dupliquer, Copier vers l'autre vue | supprime les copies |
| Déplacer, coller-couper, Déplacer vers l'autre vue | redéplace à l'origine |
| Renommer, renommage par lot | renomme à l'inverse |
| Mettre à la corbeille | restaure |
| Créer un dossier (dont « Créer le dossier manquant » via `Mkpath`) | supprime le dossier, s'il est vide (à vérifier) |
| Créer un fichier par modèle ou collage de données (`Put`) | supprime le fichier |
| Lier | supprime le lien |

  Réf. `views/dolphinview.cpp:967`, `:989`, `:1053`, `:2380`, `dolphinviewcontainer.cpp:1045-1046`.
- **[MVP]** **Non annulables** : la **suppression définitive**, le **vidage de la corbeille** et la restauration depuis la corbeille. Les changements de propriétés de vue ne sont pas non plus concernés.
- **[MVP]** **Garde-fous de KIO** (inféré) :
  - annuler une copie **supprime** des fichiers, donc KIO demande confirmation ;
  - si un fichier copié a été **modifié depuis** (date de modification différente), KIO prévient explicitement que les modifications seront perdues ;
  - si l'annulation échoue partiellement (fichier déplacé entre-temps), les erreurs sont montrées dans la bande de messages de la vue active (`UndoUiInterface::jobError` de Dolphin).
  
  Réf. `dolphinmainwindow.cpp:3174-3183`.
- **[Plus tard]** `Ctrl+Maj+T` rouvre le dernier onglet fermé. Cette fonction n'est pas liée à l'annulation des opérations sur fichiers. Réf. `dolphinmainwindow.cpp:2114-2120`.

> **Notes pour Rust/Slint.** Représenter chaque commande par un enregistrement `{ kind, pairs: Vec<(src, dst)>, mtimes_after }`. Pour la corbeille, mémoriser le chemin dans `Trash/files` retourné par l'opération. À l'annulation, revérifier l'existence des fichiers et leur date de modification avant d'agir. Taille de pile : 50 suffit. Pas besoin de persister la pile entre deux sessions.

## 14. Menu contextuel

Le menu commence par les entrées du menu hamburger, si la barre de menus est cachée. Réf. `dolphincontextmenu.cpp:70`. Le contexte est déduit du schéma de l'URL (`trash`, `*search*`, `timeline`, `recentlyused`) et de la présence d'un élément sous le curseur avec une sélection non vide. Réf. `dolphincontextmenu.cpp:68-101`.

### Fond du dossier [MVP]
Réf. `dolphincontextmenu.cpp:340-383`. Dans l'ordre :
1. **Créer nouveau ▸** (dossier, fichier, modèles, liens).
2. **Ouvrir avec** pour le dossier courant (par exemple l'ouvrir dans Kate ou VS Code), même s'il est vide.
3. **Coller** (libellé dynamique), seulement si le collage est possible.
4. **Ajouter aux Emplacements**, si le dossier n'y est pas déjà.
5. ─ **Trier par ▸**, **Mode d'affichage ▸**. ─
6. Actions supplémentaires : **Ouvrir un terminal ici** (si local), menus de service KDE, actions de gestion de versions.
7. ─ **Propriétés** (`Alt+Entrée`).

### Un fichier [MVP]
Réf. `dolphincontextmenu.cpp:266-338`, `:385-435`.
1. **Ouvrir avec** : application par défaut puis sous-menu « Ouvrir avec ▸ » (autres applications, puis « Autre application… »). Dolphin s'exclut lui-même de la liste.
2. Seulement pour un **lien symbolique** : **Afficher la cible** ─.
3. Couper, Copier, **Copier l'emplacement** (actif seulement pour un élément), **Coller** (si possible ; sur un dossier sélectionné, « coller dans »).
4. **Dupliquer ici**, **Renommer**.
5. ─ **Mettre à la corbeille** (bascule en Supprimer avec Maj), ou les deux entrées (cf. §8).
6. ─ Actions supplémentaires : terminal (dossier local), menus de service, gestion de versions.
7. Copier vers ▸ / Déplacer vers ▸ (si l'option est active).
8. En vue double : **Copier vers l'autre vue**, **Déplacer vers l'autre vue**.
9. ─ **Propriétés**.

### Un dossier [MVP]
Même menu que pour un fichier, avec en tête : **Ouvrir dans un nouvel onglet**, **Ouvrir dans une nouvelle fenêtre**, **Ouvrir dans la vue scindée**, puis Ouvrir avec, puis **Créer nouveau ▸** (dans ce dossier ; désactivé s'il n'est pas inscriptible) ─. On y trouve aussi **Ajouter aux Emplacements** s'il n'y est pas déjà. Réf. `dolphincontextmenu.cpp:201-241`, `:410-413`.

### Plusieurs éléments [MVP]
**Ouvrir dans de nouveaux onglets** n'apparaît que si **tous** les éléments s'ouvrent comme des dossiers. Viennent ensuite Ouvrir avec (applications communes à tous les types), puis le même bloc que pour un fichier, sans Copier l'emplacement actif. Réf. `dolphincontextmenu.cpp:293-310`.

### Résultats de recherche, récents, chronologie [v1]
Pour un fichier : Ouvrir avec, puis **Ouvrir le chemin**, **Ouvrir le chemin dans un nouvel onglet** et **…dans une nouvelle fenêtre**. Ces trois actions naviguent vers le parent et sélectionnent l'élément. Réf. `dolphincontextmenu.cpp:243-264`, `:279-284`.

### Corbeille [MVP]
Voir §8. Réf. `dolphincontextmenu.cpp:122-199`.

### Ouvrir avec
**[MVP]** Il est fourni par `KFileItemActions::insertOpenWithActionsTo`. Les applications viennent de `mimeapps.list` et de `mimeinfo.cache`, triées par préférence. « Autre application… » ouvre le sélecteur, qui permet de « se souvenir de l'association ». Réf. `dolphincontextmenu.cpp:495-529`.

### Réglages du menu (`dolphin_contextmenusettings.kcfg`)
Tous **vrais** par défaut, sauf `ShowCopyMoveMenu` : Ajouter aux Emplacements, Trier, Mode, Nouvel onglet, Nouvelle fenêtre, Vue scindée, Copier l'emplacement, Dupliquer, Terminal, Copier/Déplacer vers l'autre vue. **[v1]**

> **Notes pour Rust/Slint.** Le menu de Slint (`ContextMenuArea`/`Menu`) suffit. Recalculer le menu à l'ouverture. Écouter Maj pendant qu'il est ouvert pour basculer Corbeille ↔ Supprimer. Les menus de service KDE (`share/kio/servicemenus/*.desktop`) relèvent de **[Plus tard]**.

## 15. Messages d'erreur et bande de messages

- **[MVP]** **Une bande de messages par vue** (`KMessageWidget`), en haut du panneau :
  - quatre types : Information, Avertissement, Erreur, Positif ;
  - **un seul message à la fois** : le suivant remplace le précédent et ses boutons ;
  - apparition animée, retour à la ligne seulement si le texte ne tient pas ;
  - croix de fermeture ; pas de disparition automatique (à vérifier) ;
  - avec l'accessibilité active, la bande prend le focus.
  
  Réf. `dolphinviewcontainer.cpp:486-523`.
- **[MVP]** **L'annulation par l'utilisateur (`ERR_USER_CANCELED`) n'est jamais affichée.** Réf. `views/dolphinview.cpp:1640`, `:2136`, `dolphinnewfilemenu.cpp:30`.
- **[MVP]** Cas particuliers avec bouton d'action :

| Situation | Message | Bouton |
|---|---|---|
| Dossier local inexistant (`ERR_DOES_NOT_EXIST`) | message de KIO | « Create missing folder » : mkpath annulable, puis recharge ; désactivé avec l'infobulle « You do not have permission… » si l'ancêtre existant le plus proche n'est pas inscriptible |
| Accès refusé (`ERR_CANNOT_ENTER_DIRECTORY`) avec le worker `admin` disponible **[Plus tard]** | « Authorization required to enter this folder. » | « Agir en administrateur » |
| Ouvrir un fichier comme dossier (`ERR_IS_FILE`) | « Trying to open a file as a folder: … », après navigation vers le parent | « Open as File » |
| URL invalide ou vide | « The location '%1' is invalid. » / « The location is empty. » | — |
| Protocole inconnu | « Invalid protocol '%1' » | — (retour arrière) |
| Protocole source non listable (http…) | message d'information | — (le navigateur est lancé) |
| Dossier courant supprimé | voir §5 | — |
| Dolphin lancé en root | « Running Dolphin as root can be dangerous. Please be careful. » (Avertissement) | — |

  Réf. `dolphinviewcontainer.cpp:1021-1068`, `:730-756`, `:918-936`, `:104`, `views/dolphinview.cpp:2401-2411`.
- **[MVP]** **Barre d'état** :
  - confirmations brèves (« Trash operation completed. ») en texte temporaire, avec au moins **1 s** d'affichage et un débit limité à un changement toutes les 50 ms ;
  - progression du listage affichée seulement après **500 ms** ;
  - texte de sélection : « 1 dossier sélectionné, 3 fichiers sélectionnés (1,2 Mio) » ;
  - sans sélection : total du dossier, avec la taille récursive si le worker la fournit ;
  - mise à jour de la barre d'état différée de 300 ms (forcée si l'attente dépasse 2 s).
  
  Réf. `statusbar/dolphinstatusbar.cpp:37-38`, `:106-120`, `views/dolphinview.cpp:725-792`, `dolphinviewcontainer.cpp:171-174`, `:658-667`.
- **[v1]** Les actions désactivées expliquent pourquoi au survol (`DisabledActionNotifier`), par exemple pour Coller. Réf. `dolphinmainwindow.cpp:944-948`.
- **[v1]** Un renommage par lot en erreur affiche une **boîte modale** (`KMessageBox::error`), pas la bande. Réf. `views/dolphinview.cpp:894-896`.

> **Notes pour Rust/Slint.** Un composant `MessageBand { kind, text, actions[] }` par panneau. Côté Rust, une fonction `show_message(panel, kind, text, actions)` qui remplace le message courant. Traduire les `io::ErrorKind` et les `errno` (`EACCES`, `ENOSPC`, `EXDEV`, `EROFS`, `ENOTEMPTY`, `ENAMETOOLONG`) en phrases françaises, sans jamais afficher un `Debug`.

## 16. Sélection (rappels utiles aux opérations)

- **[MVP]** Les changements de sélection sont signalés aux actions **immédiatement** au passage vide ↔ non vide, et sinon regroupés toutes les **300 ms**. Juste avant un menu contextuel, le signal est forcé pour que les actions soient à jour. Réf. `views/dolphinview.cpp:119-122`, `:1661-1679`, `:1308-1321`.
- **[MVP]** Lancer Couper, Copier, Renommer, Corbeille, Supprimer, Dupliquer ou Copier l'emplacement **sans sélection** ouvre le **mode sélection** (barre du bas dédiée) au lieu de ne rien faire. Réf. `views/dolphinviewactionhandler.cpp:526-557`, `:895-931`, `dolphinmainwindow.cpp:904-921`. **[Plus tard]** pour Satchel : désactiver simplement l'action.
- **[MVP]** `Échap` vide la sélection et annule le chargement en cours. `Espace` sélectionne l'élément courant ; `Ctrl+Espace` inverse sa sélection. Réf. `kitemviews/kitemlistcontroller.cpp:446-470`, `views/dolphinview.cpp:211`.

## 17. Sécurité des données

Recensement des situations où une opération peut **perdre des données**, avec la parade de Dolphin ou de KIO. Toutes sont **[MVP]** sauf mention contraire.

| # | Risque | Parade dans Dolphin/KIO | Réf. |
|---|---|---|---|
| 1 | Suppression définitive involontaire | `ConfirmDelete=true` par défaut. Le dialogue liste les éléments et indique que l'action est irréversible. Une entrée Supprimer distincte n'est affichée qu'en option ou pour le non-local | `confirmationssettingspage.cpp:28`, `dolphincontextmenu.cpp:418` |
| 2 | Touche Suppr maintenue enfoncée | `setAutoRepeat(false)` sur Corbeille et Supprimer | `dolphinviewactionhandler.cpp:118`, `:131` |
| 3 | Corbeille impossible (autre système de fichiers, lecture seule, réseau, trop gros) | KIO **demande** s'il faut supprimer définitivement. Jamais de suppression silencieuse ni de copie implicite vers le home | KIO, inféré |
| 4 | Corbeille pleine | Limite par partition dans `ktrashrc`. L'action « supprimer les plus anciens ou les plus gros » est opt-in | `trashsettingspage.cpp`, KIO, inféré |
| 5 | Vidage de la corbeille | `ConfirmEmptyTrash=true` et notification « Trash Emptied » | `dolphintrash.cpp:78-98` |
| 6 | Écrasement lors d'une copie, d'un déplacement, d'un collage ou d'un dépôt | Jamais `KIO::Overwrite`. Dialogue de conflit avec comparaison des tailles et des dates ; « appliquer à tous » explicite | `dolphinview.cpp:958`, `:980` |
| 7 | Écrasement au renommage | `moveAs` sans écrasement, d'où le dialogue de conflit. Le modèle n'est modifié qu'en cas de succès | `dolphinview.cpp:2173-2190`, `:2378-2390` |
| 8 | Doublon écrasant l'original ou une autre copie | `setAutoRename(true)` | `dolphinview.cpp:1044` |
| 9 | Déplacement entre systèmes de fichiers interrompu | Copie complète puis suppression de la source uniquement après succès. Le fichier partiel est supprimé | KIO, inféré |
| 10 | Déplacer ou déposer un dossier dans lui-même | Dépôt sur soi ignoré ; KIO renvoie une erreur pour un dépôt dans un descendant | `draganddrophelper.cpp:51-53`, `:69-74` |
| 11 | Glisser depuis Emplacements vers une action de greffon destructrice | `ExcludePluginsActions` | `dolphinview.cpp:1531-1536` |
| 12 | Double collage après un couper | KIO vide le presse-papiers après un coller-couper | KIO, inféré |
| 13 | Coller dans un dossier non inscriptible | Action désactivée, avec la raison | `dolphinmainwindow.cpp:941-951` |
| 14 | Opérations sur un parent et son enfant sélectionnés ensemble (arbre) | Liste d'URL simplifiée (`simplifiedUrlList`), même chose pour le contenu MIME | `dolphinview.cpp:2675-2691`, `kfileitemmodel.cpp:537-548` |
| 15 | Renommage qui **cache** le fichier (nom commençant par `.`) | Confirmation « Rename and Hide » | `dolphinview.cpp:2310-2329` |
| 16 | Renommage qui change le type ou l'extension | Confirmation basée sur le type MIME (nom et contenu) | `dolphinview.cpp:2331-2376` |
| 17 | `/` dans un nom | `encodeFileName` le remplace par U+2215 au lieu de créer un chemin inattendu | `kitemlistroleeditor.cpp:79`, `:187` |
| 18 | Renommage validé par simple perte de focus | Comportement assumé ; annulable par `Ctrl+Z` | `kitemlistroleeditor.cpp:55-62` |
| 19 | Annuler une copie (qui supprime des fichiers) | Confirmation. Avertissement si le fichier a été modifié depuis la copie | KIO, inféré |
| 20 | Restauration depuis la corbeille vers un nom pris ou un dossier disparu | KIO refuse avec une explication, sans écraser | KIO, inféré |
| 21 | Exécution accidentelle d'un script ou d'un binaire | `behaviourOnLaunch=alwaysAsk`. `.desktop` non fiable signalé | `confirmationssettingspage.cpp:29` |
| 22 | Ouvrir des dizaines de fichiers d'un coup | Confirmation au-delà de 5 éléments | `dolphinview.cpp:1242-1254` |
| 23 | Dossier courant supprimé pendant qu'on y travaille | Remontée à l'ancêtre existant et avertissement : on n'opère plus sur un chemin fantôme | `dolphinviewcontainer.cpp:1096-1110` |
| 24 | Écriture de `.directory` écrasant une icône personnalisée ou d'autres clés | Seuls les groupes `[Dolphin]` et `[Settings]` sont modifiés ; le fichier n'est supprimé que s'il est vide | `viewproperties.cpp:597-614`, `:702-716` |
| 25 | Pollution de dossiers partagés ou système avec des `.directory` | Hors `$HOME`, dossiers lents ou non inscriptibles : stockage dans `~/.local/share` | `viewproperties.cpp:185-210` |
| 26 | Création multiple par double appui (réseau lent) | Un seul dialogue de création à la fois | `dolphinmainwindow.cpp:829-847` |
| 27 | Travailler en root sans s'en rendre compte | Avertissement permanent au démarrage **[v1]** | `dolphinviewcontainer.cpp:104` |
| 28 | Déplacer ou copier vers l'autre panneau par erreur (`Maj+F5`/`F6`, sans confirmation) | Annulable par `Ctrl+Z`, éléments sélectionnés à l'arrivée pour un contrôle visuel | `dolphintabwidget.cpp:354-380` |

**Exigences supplémentaires pour Satchel** (absentes de Dolphin parce que KIO les assure) :
- écriture atomique (fichier `.part` puis `rename`) ;
- `RENAME_NOREPLACE` pour tout renommage et toute fin de copie ;
- `fsync` avant de supprimer la source d'un déplacement entre systèmes de fichiers ;
- ne jamais suivre un lien symbolique lors d'une suppression récursive (`remove_dir_all` de la std est sûr depuis Rust 1.58.1, CVE-2022-21658) ;
- vérifier l'espace libre (`statvfs`) avant une grosse copie.

## 18. Récapitulatif des raccourcis (défauts Dolphin/KDE)

| Action | Raccourci | Priorité |
|---|---|---|
| Icônes / Compact / Détails | `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | MVP |
| Zoom + / − / défaut | `Ctrl++` / `Ctrl+-` / `Ctrl+0`, `Ctrl+molette` | MVP |
| Fichiers cachés | `Ctrl+H`, `Alt+.` | MVP |
| Renommer | `F2` | MVP |
| Corbeille / Supprimer | `Suppr` / `Maj+Suppr` | MVP |
| Nouveau dossier | `F10` | MVP |
| Dupliquer | `Ctrl+D` | MVP |
| Copier / Couper / Coller | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | MVP |
| Copier l'emplacement | `Ctrl+Alt+C` | v1 |
| Copier / Déplacer vers l'autre vue | `Maj+F5` / `Maj+F6` | MVP |
| Focus sur l'autre vue | `Ctrl+F3` | MVP |
| Propriétés | `Alt+Entrée` | v1 |
| Annuler / Rétablir | `Ctrl+Z` / `Ctrl+Maj+Z` | MVP |
| Recharger | `F5` | MVP |

Réf. `views/dolphinviewactionhandler.cpp:87`, `:155`, `:169`, `:177`, `:263`, `:390`, `:783-805`, `dolphinmainwindow.cpp:1893`, `:1905`, `:1910`.
