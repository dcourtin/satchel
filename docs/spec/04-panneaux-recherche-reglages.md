# 04 — Panneaux latéraux, recherche, tags et réglages

> Livre de recettes comportemental extrait de **Dolphin 26.08.3** (lecture seule du code, rien n'a été exécuté).
> Racine des références : `dolphin-26.08.3/src/`. Une référence `fichier.cpp:N` pointe vers la ligne du source.
> Certaines logiques ne sont **pas dans Dolphin** mais dans les frameworks KDE (KIO `KFilePlacesModel`/`KFilePlacesView`,
> baloo-widgets `FileMetaDataWidget`, KFileMetaData `UserMetaData`, le worker `filenamesearch` de kio-extras).
> Elles sont marquées **(KIO)**, **(baloo-widgets)**, **(KFMD)** ou **(kio-extras)** et décrites d'après l'API publique
> (`/usr/include/KF6/...`), les chaînes des bibliothèques installées et le fichier `~/.local/share/user-places.xbel` réel de la machine.
>
> Priorités : **[MVP]** = gestionnaire local deux panneaux + barre latérale + panneau d'infos, utilisable au quotidien ;
> **[v1]** = première version complète ; **[Plus tard]** = confort ou cas rares.

---

## 0. Architecture générale des panneaux

| # | Comportement | Réf. | Prio |
|---|---|---|---|
| 0.1 | Quatre panneaux : **Places** (gauche, F9), **Information** (droite, F11), **Folders** (gauche, F7), **Terminal** (bas, F4). Menu « Show Panels » qui regroupe les bascules, le verrouillage, « Show Hidden Places » et les actions de focus. | `dolphinmainwindow.cpp:2348-2361, 2395-2405, 2430-2445, 2499-2510, 2571` | [MVP] pour Places + Infos, [v1] Folders, [Plus tard] Terminal |
| 0.2 | Focus clavier : Ctrl+P bascule le focus vers ou depuis Places ; Ctrl+Shift+F4 vers ou depuis le Terminal. | `dolphinmainwindow.cpp:2567`, `:2477` | [v1] |
| 0.3 | Verrouillage de la disposition (`LockPanels`, true par défaut) : les docks n'ont pas de barre de titre. Déverrouillés, ils redeviennent des docks Qt déplaçables, et l'en-tête de la section « Places » est masqué pour éviter le doublon avec la barre de titre. | `dolphinmainwindow.cpp:1215-1230`, `dolphinplacesmodelsingleton.cpp:363-371` | Satchel : disposition fixe, pas de docks. Non retenu. |
| 0.4 | **Initialisation paresseuse** : Folders, Information et Terminal ne construisent leur contenu qu'au premier affichage. Tant qu'un panneau est caché, il ignore les changements d'URL et de sélection. | `folderspanel.cpp:130-140`, `informationpanel.cpp:124-138, 54-56`, `terminalpanel.cpp:165` | [MVP] (règle de perf : panneau caché = aucun travail) |
| 0.5 | Synchronisation : la fenêtre diffuse `urlChanged` à tous les panneaux. `Panel::setUrl` ignore une URL identique à la barre oblique finale près, et restaure l'ancienne si le panneau refuse le changement (`urlChanged()` renvoie false). | `panels/panel.cpp:44-63` | [MVP] |
| 0.6 | En vue partagée, le **survol** de la vue inactive met quand même à jour le panneau d'infos. La sélection, elle, ne suit que la vue active. | `dolphinmainwindow.cpp:1672-1673` | [MVP] : important pour les deux panneaux de Satchel |

---

## 1. Emplacements (Places)

Dolphin délègue presque tout à `KFilePlacesView` et `KFilePlacesModel` (KIO). `PlacesPanel` ajoute seulement quelques règles, et `DolphinPlacesModel` gère l'icône de la corbeille et l'en-tête de groupe. Le modèle est un **singleton partagé** par toutes les fenêtres et par la barre de recherche (`dolphinplacesmodelsingleton.cpp:402-420`).

### 1.1 Sections et ordre

L'ordre d'affichage suit l'énumération `KFilePlacesModel::GroupType` **(KIO)** (`kfileplacesmodel.h:95-102`) :

| Ordre | Groupe (clé XBEL) | Libellé FR KDE | Contenu | Correspondance Satchel | Prio |
|---|---|---|---|---|---|
| 1 | `Places` | Emplacements | Home, Desktop, Documents, Downloads, Music, Pictures, Videos, Trash et favoris utilisateur | **Favorites** | [MVP] |
| 2 | `Remote` | Distant | `remote:/` (Network) et signets sftp://, smb://, etc. | **Shares** | [v1] (affichage), [Plus tard] (navigation) |
| 3 | `RecentlySaved` | Récents | `recentlyused:/files`, `recentlyused:/locations` | Hors maquette | [Plus tard] |
| 4 | `SearchFor` | Rechercher | Recherches enregistrées (`baloosearch:`, `filenamesearch:`) | Hors maquette | [Plus tard] |
| 5 | `Devices` | Périphériques | Disques fixes (partitions internes montables) | **Devices** | [MVP] |
| 6 | `RemovableDevices` | Périphériques amovibles | USB, carte SD, optique, MTP | **Devices** (fusionnés) | [MVP] |
| 7 | `Tags` | Étiquettes | Une entrée `tags:/<nom>` par étiquette connue de Baloo | **Tags** (pastilles) | [v1] |

- Chaque groupe peut être masqué en entier (« Hide Section »). Cet état est stocké dans `<GroupState-<Groupe>-IsHidden>` (§1.2). **[v1]**
- Dans Dolphin, le Network (`remote:/`) est rangé dans *Places* (`isSystemItem`), pas dans Remote : voir le fichier réel. Satchel peut le déplacer visuellement vers *Shares* sans réécrire le XBEL.

### 1.2 Le fichier `~/.local/share/user-places.xbel` (partagé avec tout KDE)

Ce fichier est lu et écrit par Dolphin, les boîtes de dialogue de fichiers KDE, Gwenview, Kate, etc. Satchel doit le **lire et l'écrire sans rien perdre**. Structure vérifiée sur la machine (`kde_places_version` = 4) :

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE xbel>
<xbel xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"
      xmlns:kdepriv="http://www.kde.org/kdepriv"
      xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info">
 <info><metadata owner="http://www.kde.org">
   <kde_places_version>4</kde_places_version>
   <GroupState-Places-IsHidden>false</GroupState-Places-IsHidden>   <!-- idem Remote, Devices, RemovableDevices, Tags, RecentlySaved, SearchFor -->
   <withRecentlyUsed>true</withRecentlyUsed>
   <withBaloo>true</withBaloo>
 </metadata></info>
 <bookmark href="file:///home/david/T%C3%A9l%C3%A9chargements">       <!-- URL percent-encodée -->
  <title>Downloads</title>                                            <!-- titre stocké en anglais pour les entrées système -->
  <info>
   <metadata owner="http://freedesktop.org"><bookmark:icon name="folder-downloads"/></metadata>
   <metadata owner="http://www.kde.org">
    <ID>1793976122/3</ID>               <!-- horodatage/compteur, unique -->
    <isSystemItem>true</isSystemItem>   <!-- entrée par défaut : titre traduit à l'affichage -->
    <!-- possibles : <IsHidden>true</IsHidden>, <OnlyInApp>dolphin</OnlyInApp>, <UDI>…</UDI> (périphériques) -->
   </metadata>
  </info>
 </bookmark>
</xbel>
```

Règles de comportement **(KIO)** (clés confirmées dans `libKF6KIOFileWidgets`) :

| # | Règle | Prio |
|---|---|---|
| 1.2.1 | **L'ordre des `<bookmark>` dans le fichier est l'ordre d'affichage**, groupe par groupe. Réordonner revient à déplacer le nœud. | [MVP] |
| 1.2.2 | `isSystemItem=true` : entrée créée par KDE. Le titre anglais (« Downloads ») est **traduit à l'affichage**. Satchel doit faire de même (table de traduction des titres système), sinon l'utilisateur francophone verra de l'anglais. | [MVP] |
| 1.2.3 | `IsHidden=true` : entrée masquée. Elle reste visible en mode « Show Hidden Places » (affichée estompée, `dolphinmainwindow.cpp:2527`). | [v1] |
| 1.2.4 | `OnlyInApp=<app>` : entrée visible seulement dans cette application. Satchel ignore les entrées d'une autre application, mais doit les **conserver**. | [MVP] (préservation) |
| 1.2.5 | Les périphériques sont mémorisés par un bookmark avec `<UDI>` (et sans href utile) pour retenir leur ordre et leur état masqué. Avec le backend udisks2, l'UDI Solid est le chemin d'objet D-Bus (`/org/freedesktop/UDisks2/block_devices/sdb1`). | [v1] |
| 1.2.6 | `withBaloo` / `withRecentlyUsed` : drapeaux de migration. Ne pas y toucher. | [MVP] (préservation) |
| 1.2.7 | Le fichier est rechargé dès qu'il change sur le disque (KBookmarkManager surveille le fichier) : une modification dans Dolphin apparaît dans Satchel, et inversement. | [MVP] |
| 1.2.8 | Les nouvelles entrées utilisateur reçoivent un `<ID>` unique et pas d'`isSystemItem`. `addPlace(text, url, icon, appName, after)` les insère après un index donné (`kfileplacesmodel.h:289-303`). | [MVP] |
| 1.2.9 | Pas de fichier ou fichier vide : KIO régénère les entrées par défaut à partir des dossiers XDG (`~/.config/user-dirs.dirs`). Ici, les dossiers sont localisés (Bureau, Téléchargements, Musique, Images, Vidéos), mais les titres restent anglais avec `isSystemItem`. | [v1] |

**Notes pour Rust/Slint**
- Lecture et écriture : `quick-xml` en mode événements, ou `xmltree`/`roxmltree`+écriture manuelle, en **conservant tout nœud inconnu** (round-trip fidèle). Ne jamais régénérer le fichier à partir d'un modèle réduit. Écriture atomique (fichier temporaire + `rename`) et sauvegarde.
- Surveillance : `notify` (inotify) sur `~/.local/share/user-places.xbel`, pour recharger quand le fichier change. Comme le fichier est remplacé par `rename`, il faut surveiller le **dossier parent**.
- URL : `url` + `percent-encoding`. Dossiers XDG : crate `dirs`, ou lecture directe de `user-dirs.dirs`.
- Icônes nommées (`folder-downloads`) : thème d'icônes freedesktop via `freedesktop-icons` (ou `linicon`). Lire le thème courant dans `~/.config/kdeglobals` `[Icons] Theme=`.

### 1.3 Interaction avec une entrée

| # | Comportement | Réf. | Prio |
|---|---|---|---|
| 1.3.1 | Clic : la vue active navigue vers l'URL. Si la vue y est déjà, le filtre est vidé, la vue est **rechargée** (cas d'un périphérique remonté) et le focus revient à la vue. | `dolphinmainwindow.cpp:1628-1645` | [MVP] |
| 1.3.2 | Clic milieu ou Ctrl+clic : nouvel onglet. Ctrl+Shift : nouvel onglet activé. Menu : « Open in New Window ». **(KIO)** + signaux `tabRequested` / `activeTabRequested` / `newWindowRequested` | `dolphinmainwindow.cpp:2511-2517` | [v1] |
| 1.3.3 | Le menu contextuel ajoute « **Open in Split View** » (si l'URL est valide) et « Configure Trash… » (corbeille seulement). Dans Satchel, l'équivalent est « Ouvrir dans l'autre panneau ». | `placespanel.cpp:45-56, 183-205` | [MVP] |
| 1.3.4 | Menu contextuel **(KIO)** : Edit… (titre, URL, icône, case « Only show when using this application »), Hide, Hide Section, Show All Entries, Remove (entrées non périphériques), Add Entry…, Icon Size, et selon le cas Mount / Unmount / Eject / Safely Remove / Empty Trash / Properties. | — | Edit, Remove, Add : [MVP] ; Hide : [v1] |
| 1.3.5 | **Mise en évidence** : l'entrée surlignée est le `closestItem(url)` **(KIO)**, c'est-à-dire la place dont l'URL est le plus long préfixe de l'URL courante. Dans `~/Documents/x/y`, « Documents » est surligné. | `placespanel.cpp:90-93`, `kfileplacesmodel.h:381` | [MVP] |
| 1.3.6 | Taille d'icône : `[PlacesPanel] IconSize`, par défaut `KIconLoader::SizeSmallMedium` (**22 px**) ; -1 signifie automatique. Modifiable par le menu contextuel et **enregistrée immédiatement**. | `dolphin_placespanelsettings.kcfg:10-13`, `placespanel.cpp:60-69, 123-124` | [v1] |
| 1.3.7 | Pas de redimensionnement automatique des éléments (`setAutoResizeItemsEnabled(false)`). | `placespanel.cpp:39` | — |

### 1.4 Glisser-déposer

| # | Comportement | Réf. | Prio |
|---|---|---|---|
| 1.4.1 | **Réordonner** : un glisser interne (MIME `application/x-kfileplacesmodel-*`) déplace l'entrée dans **son groupe**. Il est toujours accepté, même au-dessus d'une place non inscriptible. | `placespanel.cpp:127-159` | [MVP] |
| 1.4.2 | **Ajouter un favori** : déposer un **dossier entre deux entrées** crée une place à cet endroit **(KIO)**. Un fichier qui n'est pas un dossier n'est pas accepté comme place. | `dolphinplacesmodelsingleton.cpp:340-349` | [MVP] |
| 1.4.3 | **Déposer des fichiers sur une place** copie, déplace ou lie vers sa cible, avec le même menu Copier/Déplacer/Lier que dans la vue (`DragAndDropHelper::dropUrls`). Les erreurs sont affichées, sauf l'annulation par l'utilisateur. | `placespanel.cpp:36-37, 171-181` | [MVP] |
| 1.4.4 | Une place dont le protocole n'est pas inscriptible (ou dont l'URL est invalide) refuse le dépôt : action `IgnoreAction`. | `placespanel.cpp:148-155` | [MVP] |
| 1.4.5 | **Ouverture au survol pendant un glisser** : maintenir le glisser **750 ms** au-dessus d'une place l'ouvre dans la vue, si `AutoExpandFolders` est vrai. Sinon délai 0, ce qui désactive l'ouverture. | `placespanel.cpp:117-121` | [v1] |
| 1.4.6 | Les glisser venant d'Ark (extraction d'archive) sont acceptés au survol, mais le dépôt est traité par le helper, pas par le modèle. | `dolphinplacesmodelsingleton.cpp:333-349` | [Plus tard] |

### 1.5 Périphériques (montage, démontage, éjection, chiffré)

| # | Comportement | Réf. | Prio |
|---|---|---|---|
| 1.5.1 | Les périphériques viennent de Solid (udisks2), pas du XBEL. Ils sont classés en `Devices` (fixes) ou `RemovableDevices` (amovibles ou hotplug) selon `FixedDeviceRole` **(KIO)**. Les partitions système ou ignorées (indice udisks `HintIgnore`/`HintSystem`) ne sont pas affichées. | `kfileplacesmodel.h:67-78` | [MVP] |
| 1.5.2 | Un périphérique non monté (`SetupNeededRole`) est affiché **estompé**. Un clic le **monte** (`requestSetup`), puis navigue vers le point de montage. L'erreur éventuelle passe par `errorMessage` et s'affiche dans la fenêtre. | `placespanel.cpp:77`, `kfileplacesmodel.h:276` | [MVP] |
| 1.5.3 | Volume **chiffré** (LUKS) : le clic déclenche le déverrouillage. KDE demande la phrase de passe dans une boîte dédiée, puis le système de fichiers déverrouillé est monté. | (KIO/Solid) | [v1] |
| 1.5.4 | Bouton d'éjection en surimpression sur la ligne (`TeardownOverlayRecommendedRole`) pour les volumes amovibles montés. Actions de menu : Unmount (`TeardownAllowed`), Eject (`EjectAllowed`, optique), Safely Remove (démontage puis mise hors tension du disque USB). | `kfileplacesmodel.h:143-160, 239-270` | [MVP] démontage + éjection |
| 1.5.5 | **Séquence de démontage depuis la barre** : (1) Dolphin intercepte la demande (`setTeardownFunction`). (2) Si le terminal intégré est dans le point de montage, il fait d'abord `cd ~`, et le démontage n'a lieu qu'une fois ce `cd` constaté. (3) Sinon, démontage immédiat. (4) En cas de succès, **toute vue ouverte dans le point de montage revient au dossier personnel**. | `placespanel.cpp:41-43, 105-113, 207-244`, `dolphinmainwindow.cpp:1726-1781, 388-391` | [MVP] (étape 4), [Plus tard] (étape 2) |
| 1.5.6 | Démontage demandé **depuis l'extérieur** (applet Plasma) : même redirection des vues vers Home et `cd ~` du terminal, mais Dolphin ne relance pas le démontage. | `placespanel.cpp:225-231`, `dolphinmainwindow.cpp:1741-1751` | [v1] |
| 1.5.7 | Le panneau d'infos affiché sur un média démonté se remet sur le dossier courant après 1 s (§2.6). | `informationpanel.cpp:353-362` | [v1] |
| 1.5.8 | Un terminal intégré caché fait `cd /` pour ne pas bloquer le démontage. | `terminalpanel.cpp:85-106` | [Plus tard] |

### 1.6 Espace libre

- Les périphériques montés pour lesquels `CapacityBarRecommendedRole` est vrai affichent une **barre de capacité** sous leur nom, calculée à partir de l'espace libre et total du système de fichiers **(KIO)**. **[MVP]** Dans la maquette ForkLift, c'est un petit indicateur sous le nom du volume.
- Côté Dolphin, l'espace libre de la barre d'état est rafraîchi **toutes les 60 s** par point de montage, avec un cache par point de montage (`statusbar/mountpointobservercache.cpp:70`). Satchel peut reprendre la même règle pour la barre latérale : rafraîchir à l'affichage, après chaque opération de fichier terminée sur ce volume, et toutes les 60 s. **[MVP]**

### 1.7 Corbeille

- L'icône bascule entre `user-trash` et `user-trash-full` selon que la corbeille est vide ou non (signal `emptinessChanged` d'un lister sur `trash:/`). Le source note une divergence possible avec `Trash::isEmpty()`, qui lit le fichier de config. `dolphinplacesmodelsingleton.cpp:351-361, 379-400` **[MVP]**
- Le menu contextuel propose « Empty Trash » **(KIO)** et « Configure Trash… », qui ouvre la page Trash des réglages (KCM `kcm_trash`). `placespanel.cpp:161-169`, `settings/trash/trashsettingspage.cpp:18` **[v1]**

### 1.8 Distant, Récents, recherches enregistrées

- Les entrées Remote (sftp, smb, webdav…) dépendent de KIO. Satchel MVP est local : il doit **conserver et afficher** ces entrées (estompées, avec une infobulle « non pris en charge ») plutôt que les supprimer. **[v1]**
- La barre de recherche peut enregistrer une recherche comme place, avec l'icône `folder-saved-search-symbolic` (§4.9). **[Plus tard]**

**Notes pour Rust/Slint (périphériques)**
- Crate `udisks2` (bâtie sur zbus) : `ObjectManager.GetManagedObjects` et les signaux `InterfacesAdded`/`InterfacesRemoved` pour le branchement à chaud. Méthodes `Filesystem.Mount/Unmount`, `Encrypted.Unlock(passphrase)` puis `Mount` sur le `CleartextDevice`, `Drive.Eject` et `Drive.PowerOff` (« Safely Remove »). Propriétés utiles : `Block.HintIgnore`, `HintSystem`, `HintName`, `HintIconName`, `IdLabel`, `Drive.Removable`, `MediaRemovable`, `Filesystem.MountPoints`.
- L'autorisation passe par polkit, pris en charge par l'agent polkit de Plasma. Rien à coder.
- Espace libre : `nix::sys::statvfs` ou `fs4::available_space`/`total_space`.
- La phrase de passe LUKS se demande dans une boîte Slint, sans la stocker. KWallet est possible plus tard, via D-Bus `org.kde.kwalletd6`.
- L'UDI stocké dans le XBEL (§1.2.5) est le chemin d'objet udisks2, ce qui permet de retrouver l'ordre et l'état masqué choisis dans Dolphin.

---

## 2. Panneau d'informations

### 2.1 Que montre-t-il ? (machine d'états)

Priorité d'affichage dans `showItemInfo()` (`informationpanel.cpp:222-262`) :

1. **Survol** : si un élément est survolé, que `showHovered` est vrai et que la sélection n'est pas « active », l'élément survolé est affiché.
2. **Sélection multiple** (plus d'un élément) : résumé multiple (§2.4).
3. **Sélection simple** : cet élément.
4. **Rien** : le **dossier courant**, via un `stat` avec **taille récursive** (`StatRecursiveSize`), ce qui affiche la taille totale du dossier (`informationpanel.cpp:253-269`).

| # | Règle | Réf. | Prio |
|---|---|---|---|
| 2.1.1 | Une nouvelle sélection efface le survol en cours (« la sélection l'emporte sur un survol périmé »). | `informationpanel.cpp:49-73` | [MVP] |
| 2.1.2 | Un survol est **ignoré pendant un clic gauche maintenu** (sélection à l'élastique en cours). | `informationpanel.cpp:81-85` | [MVP] |
| 2.1.3 | Quitter le survol (élément nul) revient à la sélection s'il y en a une, sinon au dossier. | `informationpanel.cpp:88-92`, `views/dolphinview.cpp:1502-1507` | [MVP] |
| 2.1.4 | Un changement de dossier vide la sélection mémorisée et annule les requêtes en cours. | `informationpanel.cpp:98-122` | [MVP] |
| 2.1.5 | Sélection vidée : affichage du dossier courant, s'il n'est pas déjà affiché. | `informationpanel.cpp:58-65` | [MVP] |

### 2.2 Délais (à reproduire tels quels)

| Minuteur | Valeur | Rôle | Réf. |
|---|---|---|---|
| `m_urlChangedTimer` | **200 ms** | Après un changement de dossier, laisse la liste s'afficher avant de calculer les métadonnées. | `informationpanel.cpp:405-408` |
| `m_infoTimer` | **300 ms** | Délai avant d'afficher l'élément **survolé**, et après un redimensionnement. | `informationpanel.cpp:400-403, 140-147` |
| `m_resetUrlTimer` | **1000 ms** | L'élément affiché a été supprimé, ou le média démonté : retour au dossier courant si l'URL est toujours invalide. | `informationpanel.cpp:410-413, 278-289` |
| `m_outdatedPreviewTimer` | **100 ms** | Si le nouvel aperçu n'est pas prêt, l'ancien est grisé (fichier) ou remplacé par l'icône (dossier). Cela évite le scintillement. | `informationpanelcontent.cpp:57-60, 390-410` |

L'invariant `urlChanged < info < reset` est vérifié par assertion (`informationpanel.cpp:415-416`). La **sélection**, elle, s'affiche **immédiatement**, sans délai. **[MVP]**

### 2.3 Contenu et mise en page (de haut en bas)

| # | Élément | Détails | Réf. | Prio |
|---|---|---|---|---|
| 2.3.1 | **Aperçu** | Carré : largeur du panneau moins 4 espacements, aussi grand que possible. Minimum 160 px de large (128+32) et 128 px de haut. Pour la vignette, Satchel prend la grande vignette de la maquette ForkLift. | `informationpanelcontent.cpp:65-69, 444-463` | [MVP] |
| 2.3.2 | **Nom** | Gras, centré, retour à la ligne n'importe où, **sélectionnable à la souris**, sans cadre. Hauteur ajustée au texte. | `informationpanelcontent.cpp:77-93, 432-442` | [MVP] |
| 2.3.3 | Séparateur | — | `:136` | — |
| 2.3.4 | **Métadonnées** | `Baloo::FileMetaDataWidget` **(baloo-widgets)** dans une zone défilante sans barre horizontale, police « plus petite lisible ». | `informationpanelcontent.cpp:98-102, 124-131, 301-318` | [MVP] |
| 2.3.5 | Mode « Configure… » | Libellé « Select which data should be shown: », cases à cocher par propriété, boutons Save/Cancel. Les choix sont enregistrés dans `~/.config/baloofileinformationrc`, groupe `[Show]`, sous la forme `<propriété>=true|false`. | `informationpanelcontent.cpp:104-122, 264-269` | [v1] |

**Propriétés affichées par baloo-widgets** (selon le type) : Type, Taille (pour un dossier : nombre d'éléments, ou taille récursive pour le dossier courant), Modifié, Créé, Accédé, Emplacement (lien cliquable, signal `urlActivated`, ce qui navigue), **Étiquettes** (éditables), **Note** (éditable, 5 étoiles, demi-étoiles), **Commentaire** (éditable), et les métadonnées extraites : dimensions et date de prise de vue pour les images, durée, artiste, album et piste pour l'audio, nombre de pages et auteur pour les documents, etc. Le format de date est long ou condensé (§2.7).

**Correspondance avec la maquette Satchel (inspirée de ForkLift)**
- Section **General** (repliable) : Type, Taille, Emplacement, Créé, Modifié, Accédé, plus les métadonnées extraites. **[MVP]** pour les champs du système de fichiers, **[v1]** pour les métadonnées extraites.
- Section **Permissions** (repliable) : grille R/W/X × Propriétaire/Groupe/Autres, plus l'octal. Le panneau d'infos de Dolphin ne l'affiche pas : c'est la boîte Propriétés de KIO qui le fait. Le format de référence vient de `ContentDisplay/UsePermissionsFormat` (Symbolic, Numeric ou Combined, §5.5). **[MVP]** en lecture ; **[v1]** en écriture (chmod), à condition d'être propriétaire.
- Étiquettes et note dans une section dédiée ou dans General. **[v1]**

### 2.4 Sélection multiple

- L'aperçu devient l'icône `dialog-information`. Le nom devient « **N items selected** » (pluriel géré). Le lecteur média est masqué. L'élément courant est remis à zéro. `informationpanelcontent.cpp:278-296` **[MVP]**
- Les métadonnées sont celles de la liste d'éléments **(baloo-widgets)** : taille totale cumulée, et étiquettes, note et commentaire **éditables pour tous à la fois**. Une valeur n'est affichée que si elle est commune à tous les éléments. **[MVP]** taille totale + nombre ; **[v1]** édition groupée.
- Survoler un élément pendant une sélection multiple affiche cet élément. Quitter le survol revient au résumé. **[MVP]**

### 2.5 Aperçu : image, animation, vidéo, audio

| # | Comportement | Réf. | Prio |
|---|---|---|---|
| 2.5.1 | La génération utilise la liste de plugins de miniatures de `kdeglobals` `[PreviewSettings] Plugins`, mise à l'échelle `Unscaled` à la taille de l'aperçu, avec le DPR de l'écran. **La limite de taille de fichier est ignorée pour les fichiers locaux non lents.** | `informationpanelcontent.cpp:179-190` | [MVP] |
| 2.5.2 | Une seule génération à la fois : la précédente est annulée (évite une course, bug 250787). | `:161-167, 193-199, 280-284` | [MVP] |
| 2.5.3 | Si la génération échoue, l'icône du type MIME est affichée avec ses **surimpressions** (lien, verrou, etc.). | `:355-363` | [MVP] |
| 2.5.4 | Un aperçu plus petit que la zone n'est pas agrandi : il est centré. Les surimpressions sont appliquées par-dessus. | `:365-388` | [MVP] |
| 2.5.5 | **Image animée** (tout format lisible par QMovie : GIF, APNG, WebP animé…) : animée en boucle dans l'aperçu. | `:216-218, 247-249`, `pixmapviewer.cpp:70-135` | [v1] |
| 2.5.6 | **Vidéo/audio** (`video/*` hors animation, `audio/*`) : sous l'aperçu, bouton lecture/pause et **barre de position**. Le curseur devient une main sur l'aperçu, et un **clic dessus bascule lecture/pause**. En vidéo, la lecture remplace l'aperçu (même taille) ; à la fin ou à l'arrêt, la vignette revient. | `informationpanelcontent.cpp:219-245`, `mediawidget.cpp:185-215, 300-345` | [v1] |
| 2.5.7 | `previewsAutoPlay` (faux par défaut) : lecture automatique. En vidéo avec lecture automatique, l'aperçu est masqué d'emblée (pas de scintillement). | `informationpanelcontent.cpp:235-241`, `mediawidget.cpp:160-182` | [v1] |
| 2.5.8 | Si on rafraîchit le même fichier alors qu'il est en lecture, en pause ou arrêté, l'état du lecteur est **conservé**. | `informationpanelcontent.cpp:228-234` | [v1] |
| 2.5.9 | Cliquer dans la barre de position place la tête **directement** au point cliqué (pas par pas). Les flèches gauche/droite déplacent de ±1 % de la durée. Si on cherche une position alors que le lecteur est arrêté, le média est chargé, positionné, puis mis en pause. | `mediawidget.cpp:84-135, 352-372` | [v1] |
| 2.5.10 | Masquer le panneau ou le lecteur **arrête la lecture**. | `mediawidget.cpp:262-268` | [v1] |
| 2.5.11 | URL de recherche non locale : pas d'aperçu, icône `baloo`. | `informationpanelcontent.cpp:204-212` | [Plus tard] |
| 2.5.12 | `previewsShown=false` : aperçu et lecteur masqués, animation arrêtée. | `:257-261` | [v1] |

### 2.6 Mise à jour en direct

Le panneau écoute les signaux D-Bus **`org.kde.KDirNotify`** (`informationpanel.cpp:418-426`) :
- `FileRenamed` : si l'élément affiché est renommé, la sélection simple est mise à jour vers la nouvelle URL. `:291-307`
- `FilesAdded(dir)` sur l'URL affichée : rafraîchissement différé (cas de la corbeille qui se remplit ou se vide). `:309-316`
- `FilesChanged` : rafraîchissement immédiat si l'URL affichée est concernée. `:326-334`
- `FilesRemoved` et `leftDirectory` (média démonté) : marqué invalide, puis retour au dossier courant après 1 s. `:336-362`
- Le modèle de la vue signale aussi les éléments modifiés (`fileItemsChanged`). `:318-324`

**[MVP]** avec inotify pour Satchel. **[v1]** pour écouter aussi KDirNotify, afin de voir les changements faits par Dolphin ou des workers KIO (qui ne passent pas forcément par inotify, par exemple dans la corbeille).

### 2.7 Menu contextuel du panneau et réglages

Menu au clic droit (ou appui long) : **Preview**, **Auto-Play media files**, **Show item on hover**, **Configure…** (désactivé pendant la configuration), **Condensed Date** (`informationpanel.cpp:155-220`). Chaque bascule est enregistrée immédiatement et rafraîchit le contenu. Les mêmes options existent dans Réglages › Interface › Panels, où Auto-play est grisé si Preview est décoché (`settings/interface/panelsettingspage.cpp:41-109`). **[v1]**

**Notes pour Rust/Slint**
- Miniatures : reprendre le **cache freedesktop** `~/.cache/thumbnails/{normal,large,x-large,xx-large}/<md5(uri)>.png`, partagé avec KDE (KIO l'écrit). Génération : `image` (+ `resvg` pour SVG, `pdfium-render` ou `poppler` pour PDF, `ffmpeg-next` ou gstreamer pour la vidéo). Toujours dans un thread, avec annulation par jeton de génération (règle 2.5.2).
- Médias : `gstreamer` (playbin) ; rendu vidéo dans Slint par copie de trames dans une `slint::Image` (appsink). C'est coûteux : **[v1]**.
- Métadonnées extraites : `kamadak-exif` / `nom-exif` (images), `lofty` (audio), `symphonia` ou `ffprobe` (durée vidéo).
- Taille récursive du dossier courant : calcul en tâche de fond annulable (`walkdir` + `rayon`), affichage progressif.
- KDirNotify : zbus, `match` sur l'interface `org.kde.KDirNotify` (signaux de session). Satchel peut aussi **émettre** `FilesAdded`/`FilesRemoved`/`FileRenamed` après ses propres opérations pour que Dolphin se mette à jour.
- Le délai de survol de 300 ms se fait avec un minuteur Slint (`Timer`, `SingleShot`), réarmé à chaque survol.

---

## 3. Panneaux Dossiers (arborescence) et Terminal

### 3.1 Folders

| # | Comportement | Réf. | Prio |
|---|---|---|---|
| 3.1.1 | Arbre des **dossiers seulement**. Fichiers cachés selon `[FoldersPanel] HiddenFilesShown` (faux), réglage **propre au panneau**. | `folderspanel.cpp:144-146`, kcfg `FoldersPanel` | [v1] |
| 3.1.2 | **Racine de l'arbre** : URL distante, racine du protocole (`/`) ; dans le dossier personnel avec `LimitFoldersPanelToHome` (vrai), le dossier personnel ; sinon `/`. | `folderspanel.cpp:313-338` | [v1] |
| 3.1.3 | Hors du dossier personnel avec la limite active : l'arbre affiche `/`. En revanche, **basculer l'option** (reloadTree) alors qu'on est hors du dossier personnel **fait sauter la vue vers Home**. | `folderspanel.cpp:68-72, 116-121, 332-347` | [v1] |
| 3.1.4 | Synchronisation vue → arbre : le dossier courant est sélectionné, **les parents sont dépliés** puis le défilement l'amène à l'écran. Si les parents ne sont pas encore chargés, la sélection a lieu à la fin du chargement. Sur la racine, la sélection est vidée. | `folderspanel.cpp:345-370, 282-301` | [v1] |
| 3.1.5 | Les URL de **recherche sont ignorées** : l'arbre garde l'ancien dossier, car des noms identiques sans chemin n'auraient aucun sens. | `folderspanel.cpp:101-114` | [v1] |
| 3.1.6 | Clic **simple** = navigation, **toujours**, même si le système est réglé en double-clic. Double-clic : naviguer et déplier. Sélection simple. Ouvrir automatiquement pendant un glisser = déplier seulement. | `folderspanel.cpp:151-155` | [v1] |
| 3.1.7 | Modificateurs : Ctrl = nouvel onglet ; Ctrl+Shift = nouvel onglet actif ; **Shift seul = déplier sans naviguer**. Clic milieu = nouvel onglet (Shift : actif). | `folderspanel.cpp:188-220` | [v1] |
| 3.1.8 | Entrée/Retour sont absorbés, sans effet. | `folderspanel.cpp:178-186` | — |
| 3.1.9 | Dépôt sur un dossier de l'arbre : même menu Copier/Déplacer/Lier que la vue. | `folderspanel.cpp:242-261` | [v1] |
| 3.1.10 | Renommage : en place si `RenameInline`, sinon boîte de dialogue. Les noms vides, `.` et `..` sont refusés. Le renommage est enregistré dans l'historique d'annulation. | `folderspanel.cpp:90-99, 263-280` | [v1] |
| 3.1.11 | Menu contextuel : Cut, Copy, Rename…, Move to Trash / Delete, Show Hidden Files, Limit to Home Directory, Automatic Scrolling, Properties. | `treeviewcontextmenu.cpp:48-130` | [v1] |
| 3.1.12 | `AutoScrolling` existe dans la config mais **n'est pas implémenté** (« TODO: Not supported yet »). | `folderspanel.cpp:79-83` | Non retenu |
| 3.1.13 | À la première ouverture, l'arbre est invisible pendant le chargement, puis apparaît en fondu après 250 ms (200 ms, InOutQuad). | `folderspanel.cpp:137-140, 282-311` | [Plus tard] |

> Satchel : la maquette n'a pas de panneau Folders. Option : un mode « arbre » de la vue (`DetailsMode/ExpandableFolders`, vrai), ou un panneau repliable dans la barre latérale. **[v1]**

### 3.2 Terminal

| # | Comportement | Réf. | Prio |
|---|---|---|---|
| 3.2.1 | Il intègre la KPart Konsole. Si elle est absente, un message « Konsole is not installed » s'affiche avec un bouton « Install Konsole » (`appstream://`). | `terminalpanel.cpp:165-223` | Satchel : [Plus tard] |
| 3.2.2 | **Vue → terminal** : à chaque changement d'URL, si le terminal existe, est visible, n'a **pas de programme au premier plan** et que « Follow Directory Switch » est actif (vrai par défaut), Dolphin envoie `Ctrl+E Ctrl+U` (vider la ligne, **protection contre `rm -rf *` en attente**), puis ` cd '<chemin>'\r`. L'espace initial évite l'historique du shell. La première fois, il envoie aussi ` clear\r`. | `terminalpanel.cpp:143-156, 271-298` | [Plus tard] |
| 3.2.3 | **Terminal → vue** : quand l'utilisateur fait `cd` dans le terminal, la vue suit, si la synchronisation est active. Les `cd` envoyés par Dolphin sont reconnus grâce à une file de chemins canoniques et ne sont pas renvoyés. | `terminalpanel.cpp:329-366` | [Plus tard] |
| 3.2.4 | URL non locale : `mostLocalUrl` si le protocole est `:local`. Sinon, montage **KIOFuse** par D-Bus (`org.kde.KIOFuse` `/org/kde/KIOFuse`, `mountUrl`) et `cd` dans le point FUSE. En sens inverse, `remoteUrl` traduit un chemin FUSE en URL. | `terminalpanel.cpp:236-262, 300-327, 341-365` | [Plus tard] |
| 3.2.5 | Fermeture du shell (`exit`) : le panneau se masque. Fermer la fenêtre avec un programme en cours demande confirmation (`ConfirmClosingTerminalRunningProgram`). | `terminalpanel.cpp:73-78`, `dolphinmainwindow.cpp:704` | [Plus tard] |
| 3.2.6 | Raccourcis voisins : Shift+F4 « Open Terminal » (terminal externe dans le dossier courant), Shift+Alt+F4 « Open Terminal Here » (sur la sélection). | `dolphinmainwindow.cpp:2196-2214` | **[MVP]** (terminal externe) |

**Notes pour Rust/Slint** : pour le MVP, lancer le terminal externe défini dans `kdeglobals` `[General] TerminalApplication` (souvent `konsole`), avec `--workdir <dir>`. Un terminal intégré plus tard demanderait `alacritty_terminal` + `portable-pty`, avec rendu de la grille dans Slint. Il faudra alors reprendre les règles 3.2.2 (vider la ligne, espace initial, pas de `cd` si un programme tourne : `tcgetpgrp` du pty ≠ pid du shell).

---

## 4. Recherche

### 4.1 Ouvrir, fermer, saisir

| # | Comportement | Réf. | Prio |
|---|---|---|---|
| 4.1.1 | **Ctrl+F** (« Search… ») : affiche la barre de recherche de la vue active et y place le focus, avec le texte tout sélectionné. Une action « Toggle Search Bar » sans raccourci existe pour la barre d'outils. | `dolphinmainwindow.cpp:929-933, 1937-1950` | [MVP] |
| 4.1.2 | Ctrl+Shift+F : ouvre l'outil de recherche externe préféré (KFind…) sur le dossier courant. | `dolphinmainwindow.cpp:2187` | [Plus tard] |
| 4.1.3 | **Recherche pendant la frappe** : chaque modification relance un minuteur de **500 ms**, puis la recherche démarre. **Entrée** lance la recherche tout de suite et **renvoie le focus à la vue**. | `search/bar.cpp:158-163, 308-321` | [MVP] |
| 4.1.4 | **Échap** : si le champ est vide, la barre se ferme ; sinon le texte est effacé, ce qui fait revenir au dossier d'origine. **Flèche bas** : focus vers la vue des résultats. | `bar.cpp:279-298` | [MVP] |
| 4.1.5 | Requête vide, sans critère : **retour au dossier d'origine** (pas de recherche « tout »). | `bar.cpp:323-333, 38-44` | [MVP] |
| 4.1.6 | Bouton × « Quit searching » : ferme la barre et **restaure l'URL d'origine**. Si le focus était dans la barre, il revient à la vue. | `bar.cpp:84-91, 190-203` | [MVP] |
| 4.1.7 | Naviguer ailleurs (URL non-recherche) pendant qu'une recherche est configurée **ferme la barre**, car elle montrerait une recherche périmée. Ouvrir une URL de recherche (place enregistrée, historique) **rouvre la barre**, préremplie à partir de l'URL. | `dolphinviewcontainer.cpp:896-905, 366-393` | [v1] |
| 4.1.8 | Si la barre est ouverte sans recherche, naviguer met à jour le dossier « Ici ». | `dolphinviewcontainer.cpp:633-640`, `bar.cpp:173-182` | [MVP] |
| 4.1.9 | Texte indicatif : « Search… », ou « Search in file contents… » si on cherche dans le contenu avec l'outil simple. | `bar.cpp:210-216` | [MVP] |

### 4.2 Où chercher : « Here » / « Everywhere »

- Deux boutons à bascule sous le champ. **Here** = le dossier ouvert avant la recherche et ses sous-dossiers. Infobulle : « Limit the search to <chemin> and its subfolders ». **Everywhere** = outil simple : `/` récursif (« Search all directories from the root up ») ; Baloo : tous les emplacements indexés. `bar.cpp:100-130, 226-243` **[MVP]**
- Le choix est **mémorisé** dans `[Search] Location` (`FromHere` par défaut). `dolphin_searchsettings.kcfg:76-79` **[MVP]**
- La barre ouverte sur une URL non locale ou spéciale prend le dossier personnel comme chemin de secours. `dolphinviewcontainer.cpp:388-392`

### 4.3 Quoi chercher : noms ou contenu

- Menu « **Filter** » › « Search in: » ○ File names / ○ File contents. Avec Baloo, le libellé devient « File names and contents ». Mémorisé dans `[Search] What` (`FileNames` par défaut). `search/popup.cpp:72-105, 306-315` **[MVP]** noms, **[v1]** contenu.

### 4.4 Outil : « Simple search » ou « File Indexing » (Baloo)

| # | Règle | Réf. | Prio |
|---|---|---|---|
| 4.4.1 | Deux outils : **Simple search** (worker `filenamesearch`, parcours récursif) et **File Indexing** (Baloo). Préférence `[Search] SearchTool`, par défaut **`Filenamesearch`**. Baloo n'est **jamais** utilisé par défaut. | `dolphin_searchsettings.kcfg:84-87`, `popup.cpp:44-54` | [MVP] simple |
| 4.4.2 | **Retour forcé à l'outil simple** si l'emplacement n'est pas indexé (Baloo désactivé ou dossier exclu), ou si l'on cherche dans le contenu alors que Baloo est en « indexation basique seulement ». | `dolphinquery.cpp:38-64, 271-285` | [v1] |
| 4.4.3 | Dans ces cas, le bouton radio Baloo est grisé et une infobulle explique pourquoi. Un bouton ouvre `systemsettings kcm_baloofile` (ou `kcmshell6`). | `popup.cpp:164-184, 273-301` | [v1] |
| 4.4.4 | Outil simple + contenu : le script `kio-filenamesearch-grep` utilise **ripgrep-all** (`rga`), puis **ripgrep** (`rg`), puis grep. Il est personnalisable en le copiant dans `~/.local/share/kio_filenamesearch/` **(kio-extras)**. | `popup.cpp:129-141` | [v1] |
| 4.4.5 | Baloo ne suit pas les liens symboliques et peut ignorer les fichiers cachés selon sa config (texte d'aide). | `popup.cpp:291-301` | info |
| 4.4.6 | ⚠ **Bizarrerie de 26.08.3** : `popup.cpp:303-304` coche le bouton Baloo puis le **décoche aussitôt** (`setChecked(false)`). Conséquence : les sélecteurs Baloo (type, date, note, étiquettes) **ne s'affichent jamais dans le menu Filter**, et le libellé reste « File contents ». Les « chips » (§4.5) fonctionnent toujours si l'URL contient ces critères. Cela ressemble à une régression ou à un correctif temporaire. **Ne pas reproduire.** | `popup.cpp:303-319` | — |
| 4.4.7 | Bas du menu : « For more advanced searches: » avec un bouton Open KFind (ou Install KFind…). | `popup.cpp:245-260, 327-381` | [Plus tard] |

### 4.5 Critères (Baloo seulement)

Une grille dans le menu Filter, et pour chaque critère actif un « **chip** » (pastille amovible, avec ×) sur la 2ᵉ ligne de la barre. Ces critères ne sont visibles que si l'outil n'est pas Simple search (`bar.cpp:245-276`).

| Critère | Valeurs | Encodage dans la requête Baloo | Réf. | Prio |
|---|---|---|---|---|
| **File Type** | Any Type, Archive, Audio, Video, Image, Document, Spreadsheet, Presentation, Text, Folder (types KFileMetaData). **Un seul à la fois.** | `query.addType(name)` | `selectors/filetypeselector.cpp:17-84`, `dolphinquery.cpp:194-196, 301-303` | [v1] |
| **Modified since** | Sélecteur de date : « Any Date », mots-clés (aujourd'hui, semaine…), calendrier. Pas de date future. Affichage en date relative courte. | `modified>=YYYY-MM-DD` (et `modified>X` est relu comme X+1 jour) | `selectors/dateselector.cpp:19-66`, `dolphinquery.cpp:210-212, 324-329` | [v1] |
| **Rating** | Any, 1+…4+, 5 étoiles ; stocké de **0 à 10** (2 par étoile). | `rating>=N` | `selectors/minimumratingselector.cpp:21-28`, `dolphinquery.cpp:214-216` | [Plus tard] |
| **Tags** | Menu à cases de toutes les étiquettes connues (`tags:/`), triées sans casse. **Toutes requises (ET)**, affichées « a && b ». Le menu reste ouvert pour cocher plusieurs étiquettes. | `tag:nom` ou `tag:"nom avec espace"` | `selectors/tagsselector.cpp:31-203`, `dolphinquery.cpp:218-224` | [v1] |

- Dans un chip, on ne peut pas choisir « Any… » ni décocher le dernier tag : seul le × retire le critère. `filetypeselector.cpp:24`, `tagsselector.cpp:150-151`
- La **taille** n'est **pas** un critère de recherche dans Dolphin. Pour Satchel, on peut l'ajouter (taille ≥, ≤) à la recherche simple. **[Plus tard]**
- La liste des tags est chargée de façon paresseuse : le bouton reste grisé tant que `tags:/` n'a rien renvoyé, et la liste est rafraîchie à chaque ouverture du menu. `tagsselector.cpp:86-114`

### 4.6 Format des URL de recherche (à reconnaître dans le XBEL et l'historique)

- `filenamesearch:?search=<terme>&checkContent=yes&url=<dossier encodé>&title=<titre>`. `url=file:///` signifie « Everywhere ». `dolphinquery.cpp:130-146, 232-251`
- `baloosearch:` : sérialisation de `Baloo::Query`. Termes : `filename:"x"` (noms seulement) ; texte nu (contenu) ; `modified>=`, `rating>=`, `tag:` ; `AND` ignoré entre les termes. Les termes inconnus sont conservés tels quels. `includeFolder` vide = partout. `dolphinquery.cpp:288-354`
- `tags:/<nom>` : recherche de tout ce qui porte l'étiquette, partout (`#` et `/` sont autorisés dans le nom). `dolphinquery.cpp:156-168`
- Une URL est considérée comme une recherche si son schéma **contient** « search ». `dolphinviewcontainer.cpp:52-55`

### 4.7 Titre des résultats (onglet, barre de titre, place enregistrée)

Construit dans `dolphinquery.cpp:357-445`, par ordre de priorité. Le nom du dossier est celui de la place si le dossier en est une, sinon son nom, sinon le chemin avec `~` :
« Search results for “X” in Dossier » / « Files containing “X” in Dossier » / « Search items tagged “a” (and “b”) in Dossier » / « <Type> search results in Dossier » / « Search results in Dossier ». Les variantes sans « in Dossier » servent pour Everywhere. **[MVP]** (titre de l'onglet du panneau)

### 4.8 Affichage des résultats

| # | Comportement | Réf. | Prio |
|---|---|---|---|
| 4.8.1 | Pendant la recherche, la barre d'état affiche « **Searching…** » avec une progression indéterminée (les workers de recherche ne donnent pas de pourcentage). | `dolphinviewcontainer.cpp:678-686` | [MVP] |
| 4.8.2 | Aucun résultat : « **No items found.** » au lieu de « 0 items ». | `dolphinviewcontainer.cpp:697-706` | [MVP] |
| 4.8.3 | Les résultats ont des **propriétés de vue propres** (contexte `search`, stockées par hachage d'URL). Par défaut : vue Détails avec les colonnes **Nom, Chemin, Modifié**. Les recherches /images et /videos s'affichent en icônes avec aperçus, /audio en détails avec Artiste, Album, Durée. | `views/viewproperties.cpp:160-170, 241-259`, `dolphinviewcontainer.cpp:331` | [MVP] (colonne Chemin) |
| 4.8.4 | Le panneau Folders ignore les URL de recherche (3.1.5). Le panneau d'infos affiche l'élément survolé ou sélectionné normalement. | — | — |

### 4.9 Enregistrer une recherche

Une icône 💾 dans le champ (« Save this search… ») est active seulement si la recherche est configurée. Elle ajoute une place avec le titre de la requête, l'URL de recherche et l'icône `folder-saved-search-symbolic`. `bar.cpp:60-65, 217, 335-344` **[Plus tard]**

**Notes pour Rust/Slint (recherche)**
- Recherche simple : `ignore::WalkParallel` (moteur de ripgrep) ou `jwalk`, avec correspondance par nom insensible à la casse (sous-chaîne, et motifs `*`/`?` via `globset`). Les résultats sont **diffusés** par lots vers le modèle Slint (`VecModel` alimenté depuis un canal, rafraîchi toutes les ~100 ms), avec annulation à chaque nouvelle frappe. Pour le contenu : `grep-searcher` + `grep-regex` (crates de ripgrep), ou appel à `rga`/`rg --files-with-matches` s'ils sont installés, comme kio-extras.
- Ignorer les points de montage réseau et `/proc`, `/sys` en mode Everywhere. Option « cachés » alignée sur la vue.
- Baloo sans C++ : `baloosearch6 [-d dossier] [-t type] "requête"` (un chemin par ligne). Indexation activée et dossiers indexés : lire `~/.config/baloofilerc` `[General]` (`Indexing-Enabled`, `folders`, `exclude folders`, `only basic indexing`, `index hidden folders`). Ici, le fichier ne contient que `dbVersion=2`, donc les valeurs par défaut s'appliquent : indexation du dossier personnel active. La base LMDB `~/.local/share/baloo/index` n'a pas d'API stable : ne pas la lire directement.
- Rouvrir une recherche depuis une URL : parser `filenamesearch:` (requête URL) et, au minimum, les termes `filename:`/`tag:`/`modified>=` de `baloosearch:`, comme `DolphinQuery`.

---

## 5. Étiquettes (tags)

### 5.1 Stockage (format KDE, partagé)

| # | Règle | Prio |
|---|---|---|
| 5.1.1 | Les étiquettes sont stockées dans l'**attribut étendu `user.xdg.tags`** du fichier (KFileMetaData `UserMetaData::tags()/setTags()` **(KFMD)** ; les clés `xdg.tags`, `xdg.comment`, `xdg.origin.url`, `baloo.rating` sont présentes dans `libKF6FileMetaData`). Valeur : chaîne UTF-8, **noms séparés par des virgules**, sans espace ajouté. | [MVP] lecture, [v1] écriture |
| 5.1.2 | Une virgule ne peut pas figurer dans un nom d'étiquette (aucun échappement). Satchel doit **interdire `,`** dans un nom. | [v1] |
| 5.1.3 | Hiérarchie : un `/` dans le nom (« Projets/Satchel ») est présenté comme une arborescence par le worker `tags:/`. | [Plus tard] |
| 5.1.4 | Attributs voisins (même famille) : **`user.baloo.rating`** (entier 0–10, demi-étoiles), **`user.xdg.comment`** (texte), `user.xdg.origin.url` (URL de téléchargement), `user.xdg.origin.email.{subject,sender,message-id}`. | [v1] note + commentaire |
| 5.1.5 | Dans Dolphin, la colonne « Tags » affiche les étiquettes **triées** (ordre naturel numérique) et jointes par « , ». Une étiquette seule est affichée telle quelle. `kitemviews/private/kbaloorolesprovider.cpp:20-34, 142-176` | [v1] |
| 5.1.6 | Les xattr `user.*` exigent un système de fichiers qui les supporte (ext4, btrfs, xfs : oui ; FAT/exFAT, et souvent NTFS ou les montages réseau : non). En cas d'échec d'écriture (`ENOTSUP`), le dire à l'utilisateur, ne pas l'ignorer. | [v1] |
| 5.1.7 | Une copie ou un déplacement **doit préserver les xattr** (KIO le fait pour les fichiers locaux). C'est à vérifier dans l'implémentation de copie de Satchel. | [v1] |

### 5.2 Liste globale des étiquettes et section Tags de la barre latérale

- KDE n'a **aucun registre** d'étiquettes : la liste vient de **l'index Baloo**, à travers le worker `tags:/`. La section Tags de Places et le sélecteur de recherche lisent `tags:/` (`tagsselector.cpp:31-45`). Sans Baloo, KDE n'affiche aucune étiquette.
- Cliquer sur une étiquette ouvre `tags:/<nom>`, c'est-à-dire tous les fichiers portant cette étiquette (§4.6).
- **Les couleurs d'étiquette n'existent pas dans KDE.** Les pastilles de couleur de Satchel (façon Finder ou ForkLift) sont une donnée **propre à Satchel**. Il faut les stocker dans la config Satchel (table nom → couleur, ordre, favoris), **jamais dans `user.xdg.tags`**, afin que Dolphin continue de lire les mêmes noms. Un nom sans couleur reçoit une couleur neutre.

**Notes pour Rust/Slint (tags)**
- Crate `xattr` : `get(path, "user.xdg.tags")`, puis `split(',')`, filtrage des vides et `trim`. `set` pour écrire, `remove` quand la liste devient vide (KFMD supprime l'attribut).
- Liste des étiquettes : **[MVP]** Satchel tient son **propre index** (fichier JSON ou `rusqlite`) des étiquettes vues ou posées, enrichi au fil des parcours. **[v1]** fusion avec Baloo via `baloosearch6 'tag:x'`, ou lister `tags:/` avec `kioclient ls tags:/` (kde-cli-tools). **[Plus tard]** indexation propre en tâche de fond.
- Pour ouvrir une étiquette : avec Baloo actif, `baloosearch6 "tag:<nom>"` ; sinon, parcours du dossier personnel en lisant le xattr (lent : progression et annulation).
- Toujours relire le xattr après écriture : un autre outil (Dolphin) peut l'avoir modifié. Le surveiller avec inotify `IN_ATTRIB`.

---

## 6. Réglages

Fichiers : `~/.config/dolphinrc` (groupes ci-dessous) ; `~/.config/kdeglobals` (aperçus, globaux KDE) ; `~/.config/kiorc` (confirmations KIO) ; `.directory` ou `~/.local/share/dolphin/view_properties/` (propriétés par dossier). Colonne **Satchel ?** : **Oui** (reprendre), **Adapter**, **Lire KDE** (lire la valeur KDE comme défaut), **Non**. Les options de KParts, du mode téléphone (sélection tactile) et de la télémétrie sont ignorées.

### 6.1 Panneaux

| Clé (`dolphinrc`) | Défaut | Effet | Satchel ? | Prio |
|---|---|---|---|---|
| `[PlacesPanel] IconSize` | 22 (SmallMedium), -1 = auto | Taille des icônes de la barre latérale | Oui | [v1] |
| `[InformationPanel] previewsShown` | true | Affiche l'aperçu dans le panneau d'infos | Oui | [MVP] |
| `[InformationPanel] previewsAutoPlay` | false | Lecture automatique audio/vidéo | Oui | [v1] |
| `[InformationPanel] showHovered` | true | Le panneau suit le survol (délai 300 ms) | Oui | [MVP] |
| `[InformationPanel] dateFormat` | 0 = LongFormat (1 = ShortFormat, « Condensed ») | Format des dates dans le panneau | Oui | [v1] |
| `[FoldersPanel] HiddenFilesShown` | false | Dossiers cachés dans l'arbre | Adapter | [v1] |
| `[FoldersPanel] LimitFoldersPanelToHome` | true | Racine de l'arbre = Home quand on y est | Adapter | [v1] |
| `[FoldersPanel] AutoScrolling` | true | Non implémenté | Non | — |
| `[General] LockPanels` | true | Disposition des panneaux figée | Non | — |
| `baloofileinformationrc [Show] <prop>` | défauts baloo-widgets | Propriétés visibles dans le panneau d'infos | Adapter (config Satchel) | [v1] |

### 6.2 Recherche

| Clé | Défaut | Effet | Satchel ? | Prio |
|---|---|---|---|---|
| `[Search] Location` | `FromHere` | Here ou Everywhere, mémorisé au clic | Oui | [MVP] |
| `[Search] What` | `FileNames` | Noms ou contenu | Oui | [MVP] |
| `[Search] SearchTool` | `Filenamesearch` | Outil préféré (Baloo seulement si choisi et possible) | Adapter | [v1] |

### 6.3 Démarrage, fenêtres, onglets, vue partagée

| Clé `[General]` | Défaut | Effet | Satchel ? | Prio |
|---|---|---|---|---|
| `RememberOpenedTabs` | true | Restaure dossiers, onglets et état de la fenêtre au lancement | Oui (par panneau) | [MVP] |
| `HomeUrl` | dossier personnel | Dossier de démarrage si on ne restaure pas | Oui | [MVP] |
| `SplitView` | false | Nouvelle fenêtre en vue partagée | Adapter : Satchel est **toujours** à deux panneaux | — |
| `CloseSplitViewChoice` | ActiveView | Quel côté se ferme | Non | — |
| `OpenExternallyCalledFolderInNewTab` | false | Instance unique, dossiers externes ouverts en onglet | Adapter | [v1] |
| `AlwaysShowTabBar` | false | Barre d'onglets visible avec un seul onglet | Oui | [v1] |
| `ShowCloseButtonOnTabs` | true | Bouton × sur les onglets | Oui | [v1] |
| `TabStyle` | AutoSize (FullWidth, FixedSize) | Largeur des onglets | Adapter | [Plus tard] |
| `OpenNewTabAfterLastTab` | false | Nouvel onglet à la fin plutôt qu'après l'onglet courant | Oui | [v1] |
| `ShowFullPathInTitlebar` | false | Chemin complet dans le titre | Oui | [v1] |
| `ConfirmClosingMultipleTabs` | true | Confirme la fermeture d'une fenêtre à plusieurs onglets | Oui | [v1] |
| `Version`, `ModifiedStartupSettings`, `ViewPropsTimestamp` | — | Internes (migration) | Non | — |

### 6.4 Navigation et comportement

| Clé `[General]` | Défaut | Effet | Satchel ? | Prio |
|---|---|---|---|---|
| `EditableUrl` | false | Barre d'adresse en mode texte plutôt qu'en fil d'Ariane | Oui | [MVP] |
| `ShowFullPath` | false | Le fil d'Ariane montre tout le chemin plutôt que depuis la place | Oui | [v1] |
| `UrlCompletionMode` | CompletionPopupAuto | Complétion dans la barre d'adresse | Oui | [v1] |
| `FilterBar` | false | Barre de filtre visible au démarrage | Oui | [v1] |
| `GlobalViewProps` | true | Même mode de vue pour tous les dossiers, ou un mode par dossier | Oui | [v1] |
| `BrowseThroughArchives` | false | Ouvre les archives comme des dossiers | Adapter | [Plus tard] |
| `RenameInline` | true | Renommage en place, ou boîte de dialogue | Oui | [MVP] |
| `AutoExpandFolders` | false | Ouverture des dossiers et places pendant un glisser (750 ms) | Oui | [v1] |
| `ShowToolTips` | false | Infobulle d'informations au survol dans la vue | Adapter (doublon du panneau d'infos) | [Plus tard] |
| `DoubleClickViewAction` | `edit_select_all` | Action d'un double-clic sur le fond de la vue | Adapter | [Plus tard] |
| `SortingChoice` | 0 = NaturalSorting (CaseInsensitive, CaseSensitive) | Tri naturel des noms | Oui | [MVP] |
| `HideXTrashFile` | false | Masque aussi les fichiers `application/x-trash` (`*~`, `.bak`) | Oui | [v1] |
| `DynamicView` | false | Bascule automatique en icônes pour les dossiers d'images | Non (au début) | [Plus tard] |
| `ShowSelectionToggle` | true | Coche de sélection au survol des icônes | Adapter | [Plus tard] |
| `ShowPasteBarAfterCopying` | true | Barre « coller » du mode sélection (tactile) | Non | — |
| `kdeglobals [KDE] SingleClick` | false (Plasma 6) | Clic simple ou double pour ouvrir (global KDE) | **Lire KDE** | [MVP] |

### 6.5 Affichage du contenu et barre d'état

| Clé | Défaut | Effet | Satchel ? | Prio |
|---|---|---|---|---|
| `[ContentDisplay] DirectorySizeMode` | ContentCount (ContentSize, None) | Taille d'un dossier : nombre d'éléments, taille récursive, ou rien | Oui | [MVP] |
| `[ContentDisplay] RecursiveDirectorySizeLimit` | 10 | Profondeur maximale du calcul récursif | Oui | [v1] |
| `[ContentDisplay] UseShortRelativeDates` | true | « Aujourd'hui 14:02 » plutôt qu'une date courte | Oui | [MVP] |
| `[ContentDisplay] UsePermissionsFormat` | 0 = Symbolic (Numeric, Combined) | Format des permissions (rwxr-xr-x / 755 / les deux). **Base de la grille R/W/X + octal.** | Oui | [MVP] |
| `[ContentDisplay] ElidingMode` | Middle (Right) | Troncature des noms longs | Oui | [MVP] |
| `[General] ShowStatusBar` | 0 = Small (FullWidth, Disabled) | Barre d'état | Adapter | [MVP] |
| `[General] ShowZoomSlider` | false | Curseur de zoom dans la barre d'état | Oui | [v1] |
| `[General] EnlargeSmallPreviews` | true | Agrandit les petites images en miniatures | Oui | [v1] |

### 6.6 Modes de vue (`[IconsMode]`, `[CompactMode]`, `[DetailsMode]`)

| Clé | Défaut | Satchel ? | Prio |
|---|---|---|---|
| `IconsMode/IconSize` · `PreviewSize` | 32 (Medium) · 64 (Huge) | Oui | [v1] |
| `IconsMode/TextWidthIndex` · `MaximumTextLines` | 1 · 3 (0 = illimité) | Oui | [v1] |
| `CompactMode/IconSize` · `PreviewSize` · `MaximumTextWidthIndex` | 16 · 32 · 0 (illimité) | Oui | [v1] |
| `DetailsMode/IconSize` · `PreviewSize` | 16 (Small) · 32 (Large) | Oui | [MVP] |
| `DetailsMode/ColumnPositions` | 0..8 | Oui | [MVP] |
| `DetailsMode/LeftPadding` · `RightPadding` | 20 · 20 | Adapter | [MVP] |
| `DetailsMode/HighlightEntireRow` | true | Oui | [MVP] |
| `DetailsMode/ExpandableFolders` | true (arbre dans la vue Détails) | Oui | [v1] |
| `*/UseSystemFont` · `ViewFont` | true · police système | Oui | [v1] |

### 6.7 Propriétés par dossier (`.directory` ou `view_properties/`, groupe `[Dolphin]`)

`ViewMode` = Icons ; `PreviewsShown` = true ; `SortRole` = text ; `SortOrder` = croissant ; `SortFoldersFirst` = true ; `SortHiddenLast` = false ; `GroupedSorting` = false ; `GroupRole` vide ; `VisibleRoles` et `HeaderColumnWidths` vides ; `ZoomLevel` = -1 ; `[Settings] HiddenFilesShown` = false (`settings/dolphin_directoryviewpropertysettings.kcfg:9-100`). → Satchel : **Adapter**. Il stocke ces réglages dans sa propre base, sans écrire de `.directory` chez l'utilisateur. Les dossiers spéciaux ont des défauts propres : Recherche (§4.8.3), Corbeille (Détails : Nom, Chemin, Date de suppression), Récents (tri par date d'accès décroissant) (`views/viewproperties.cpp:240-275`). **[v1]**

### 6.8 Aperçus (`kdeglobals [PreviewSettings]`, partagé avec KDE)

| Clé | Défaut | Effet | Satchel ? | Prio |
|---|---|---|---|---|
| `Plugins` | liste par défaut de KIO | Types pour lesquels générer des miniatures | Lire KDE | [v1] |
| `MaximumSize` | 0 Mo dans la page Dolphin (`previewssettingspage.cpp:30`). Si absent, KIO applique sa propre limite. | Taille maximale d'un fichier local à prévisualiser (sauf panneau d'infos, §2.5.1) | Lire KDE | [MVP] |
| `MaximumRemoteSize` | 0 (pas d'aperçu distant) | Idem pour les fichiers distants | Lire KDE | [Plus tard] |
| `EnableRemoteFolderThumbnail` | false | Miniatures de dossiers distants | Non | — |

### 6.9 Confirmations

| Clé | Défaut | Satchel ? | Prio |
|---|---|---|---|
| `kiorc [Confirmations] ConfirmTrash` | **false** (pas de confirmation pour la corbeille) | Lire KDE | [MVP] |
| `kiorc [Confirmations] ConfirmDelete` | true | Lire KDE | [MVP] |
| `kiorc [Confirmations] ConfirmEmptyTrash` | true | Lire KDE | [MVP] |
| `kiorc [Executable scripts] behaviourOnLaunch` | `alwaysAsk` (open, execute) | Lire KDE | [v1] |
| `dolphinrc [Notification Messages] ConfirmOpenManyFolders` · `ConfirmOpenManyTerminals` | true · true | Oui | [v1] |
| `dolphinrc [Notification Messages] ConfirmRenameFileType` | true (avertit si l'extension change) | Oui | [MVP] |
| `dolphinrc [General] ConfirmClosingTerminalRunningProgram` | true | Si terminal intégré | [Plus tard] |

(`settings/interface/confirmationssettingspage.cpp:25-29, 136-151, 192-198`)

### 6.10 Menu contextuel (`[ContextMenu]`)

| Clé | Défaut | Satchel ? | Prio |
|---|---|---|---|
| `ShowAddToPlaces` | true | Oui (« Ajouter aux favoris ») | [MVP] |
| `ShowCopyToOtherSplitView` · `ShowMoveToOtherSplitView` | true · true | **Oui : cœur du double panneau** | [MVP] |
| `ShowOpenTerminal` | true | Oui | [MVP] |
| `ShowCopyLocation` | true | Oui | [MVP] |
| `ShowDuplicateHere` | true | Oui | [v1] |
| `ShowOpenInNewTab` · `ShowOpenInNewWindow` · `ShowOpenInSplitView` | true · true · true | Adapter (« Ouvrir dans l'autre panneau ») | [v1] |
| `ShowSortBy` · `ShowViewMode` | true · true | Oui | [v1] |
| `ShowCopyMoveMenu` | false (sous-menus Copier vers / Déplacer vers) | Oui | [Plus tard] |

### 6.11 Divers

`[VersionControl] enabledPlugins` (vide, plugins Git/SVN) → **Non** au MVP, [Plus tard]. Corbeille : KCM `kcm_trash`, qui écrit `~/.config/ktrashrc` (taille maximale, durée de rétention). Satchel peut les **lire** pour l'affichage et ouvrir `systemsettings kcm_trash` pour les modifier. **[v1]**

---

## 7. Ce qu'on récupère de l'écosystème KDE sans C++

| Besoin | Source | Méthode Rust |
|---|---|---|
| Favoris, places, état masqué, ordre des périphériques | `~/.local/share/user-places.xbel` | `quick-xml` en round-trip + `notify` (§1.2) |
| Dossiers XDG localisés | `~/.config/user-dirs.dirs` | `dirs` ou lecture directe |
| Périphériques, montage, LUKS, éjection | D-Bus système `org.freedesktop.UDisks2` | `udisks2` / `zbus` |
| Espace libre | `statvfs` | `nix` / `fs4` |
| Corbeille (spec freedesktop) | `~/.local/share/Trash/{files,info}` | crate `trash` ; état vide avec `notify` |
| Étiquettes, note, commentaire | xattr `user.xdg.tags`, `user.baloo.rating`, `user.xdg.comment` | `xattr` |
| Recherche indexée | `baloosearch6`, `~/.config/baloofilerc` | `std::process::Command` + parseur INI |
| Liste des étiquettes connues de KDE | `kioclient ls tags:/` ou `baloosearch6` | `Command` (optionnel) |
| Notifications de changement inter-applications | D-Bus session `org.kde.KDirNotify` (FilesAdded/Changed/Removed, FileRenamed) | `zbus` (écoute **et** émission) |
| Montage d'URL distantes en FUSE | D-Bus `org.kde.KIOFuse` `/org/kde/KIOFuse` : `mountUrl(url)` → chemin, `remoteUrl(path)` | `zbus` : voie possible pour les **Shares** sans KIO |
| Miniatures | `~/.cache/thumbnails/*` (spec freedesktop) | lecture et écriture PNG, `md5` de l'URI |
| Thème d'icônes, polices, couleurs, clic simple, terminal | `~/.config/kdeglobals` (`[Icons] Theme`, `[KDE] SingleClick`, `[General] TerminalApplication`, `[Colors:*]`) | parseur KConfig (INI + gestion de `[$e]`, `[$i]` et des échappements `\s`) |
| Confirmations et aperçus partagés | `~/.config/kiorc`, `kdeglobals [PreviewSettings]` | idem |
| Réglages KDE (Baloo, corbeille) | `systemsettings kcm_baloofile`, `systemsettings kcm_trash` | `Command` |

**Pièges à retenir**
1. Le XBEL est **partagé** : une écriture qui perd des nœuds casse les places de toutes les applications KDE. Round-trip strict, écriture atomique.
2. Les titres `isSystemItem` sont en anglais dans le fichier : il faut les traduire à l'affichage.
3. Les couleurs d'étiquette n'existent pas chez KDE : ce sont des données Satchel.
4. La liste globale des étiquettes dépend de Baloo : sans index, il faut que Satchel tienne le sien.
5. Recherche par défaut = **outil simple, noms de fichiers, depuis ici**, avec un délai de frappe de 500 ms ; Baloo n'est qu'un choix explicite.
6. Panneau d'infos : survol 300 ms, changement de dossier 200 ms, sélection immédiate ; dossier courant affiché avec sa taille récursive.
