# 03 — Coquille de l'application : actions, onglets, vue double, navigation

Source étudiée : Dolphin 26.08.3, `src/` (références `fichier:ligne` relatives à ce dossier). Ce document décrit le **comportement observable** de Dolphin, puis ce que Satchel en retient. Les comportements qui viennent de KIO ou de KF6 et non du code de Dolphin (fil d'Ariane `KUrlNavigator`, raccourcis `KStandardShortcut`) sont signalés **(KF6)**. On ne les a pas lus dans la source et il faut les vérifier à l'implémentation.

Étiquettes : **[MVP]** = gestionnaire local à deux panneaux, utilisable au quotidien ; **[v1]** = première version complète ; **[Plus tard]** = optionnel ou lointain.

> **Différence structurante.** Dans Dolphin, la vue scindée est une option qu'on active par onglet avec F3 (`SplitView` = `false` par défaut, `settings/dolphin_generalsettings.kcfg:81`). Dans Satchel, **les deux panneaux sont le mode par défaut et permanent**. Les encadrés « ⚠ Deux panneaux par défaut » indiquent chaque endroit où cela change la recette.

---

## 1. Vocabulaire

- **Fenêtre** : une fenêtre principale. Dolphin lance un processus par fenêtre, voir §9.
- **Onglet** : il contient **un ou deux panneaux**. Dans Dolphin, la paire gauche/droite appartient à l'onglet (`dolphintabpage.cpp:26-70`).
- **Panneau** (*view container*) : il réunit une vue de fichiers, sa barre d'adresse, sa barre de filtre, sa barre d'état, sa barre de messages, ses barres du mode sélection et sa barre admin. De haut en bas : recherche, bandeau admin, message, barre haute du mode sélection, vue, barre basse du mode sélection, filtre, barre d'état (`dolphinviewcontainer.cpp:58-67`).
- **Panneau primaire / secondaire** : le primaire est à gauche (à droite en RTL), le secondaire à droite.
- **Panneau actif** : il reçoit les actions. Il y en a exactement un par onglet.

**Notes pour Rust/Slint**
- Modèle : `Window { tabs: Vec<Tab>, current: usize }`, `Tab { left: Pane, right: Pane, active: Side, splitter: f32, custom_label: Option<String> }` et `Pane { history: History, filter: FilterState, selection_mode: bool, view_state… }`.
- Chaque panneau a **son propre historique**, voir §5.1. Ne jamais partager l'historique entre panneaux.

---

## 2. Table des actions et raccourcis par défaut

Sources : `dolphinui.rc` (structure des menus), `DolphinMainWindow::setupActions()` (`dolphinmainwindow.cpp:1785-2325`), `setupDockWidgets()` (`:2327-2592`), `views/dolphinviewactionhandler.cpp` et `admin/workerintegration.cpp:38`. La mention « KStd » désigne un raccourci qui vient de `KStandardAction`/`KStandardShortcut` (KF6). L'utilisateur peut tout reconfigurer dans le dialogue des raccourcis.

### 2.1 Fichier

| Nom interne | Libellé | Raccourci | Réf. | Prio |
|---|---|---|---|---|
| `new_menu` | Créer un nouveau ▸ (sous-menu) | — | mainwindow:1790 | [v1] |
| `create_dir` | Créer un dossier… | **Ctrl+Shift+N** (KStd) | viewactionhandler:85-87 | [MVP] |
| `create_file` | Créer un fichier… | — | viewactionhandler:102 | [v1] |
| `file_new` | Nouvelle fenêtre | **Ctrl+N** (KStd) | mainwindow:1801 | [v1] |
| `new_tab` | Nouvel onglet | **Ctrl+T** | mainwindow:1810-1819 | [MVP] |
| `file_close` | Fermer l'onglet | **Ctrl+W** (KStd) | mainwindow:1829 | [MVP] |
| `undo_close_tab` | Annuler la fermeture de l'onglet | **Ctrl+Shift+T** | mainwindow:2114-2120 | [MVP] |
| `add_to_places` | Ajouter aux emplacements | — | mainwindow:1821 | [v1] |
| `renamefile` | Renommer… | **F2** (KStd) | viewactionhandler:110 | [MVP] |
| `duplicate` | Dupliquer ici | **Ctrl+D** | viewactionhandler:152-155 | [v1] |
| `movetotrash` | Mettre à la corbeille | **Suppr** (KStd + `QKeySequence::Delete` forcé) | viewactionhandler:116-122 | [MVP] |
| `deletefile` | Supprimer définitivement | **Shift+Suppr** | viewactionhandler:129-135 | [MVP] |
| `delete_shortcut` | (cachée) Suppr → suppression définitive quand la corbeille est impossible (non local) | Suppr | viewactionhandler:145-148 | [v1] |
| `show_target` | Afficher la cible (lien) | — | mainwindow:2287 | [v1] |
| `properties` | Propriétés | **Alt+Entrée** (et Alt+Enter du pavé) | viewactionhandler:159-169 | [v1] |
| `file_quit` | Quitter | **Ctrl+Q** (KStd) | mainwindow:1837 | [MVP] |

### 2.2 Édition

| Nom interne | Libellé | Raccourci | Réf. | Prio |
|---|---|---|---|---|
| `edit_undo` / `edit_redo` | Annuler / Refaire (opérations fichier) | **Ctrl+Z** / **Ctrl+Shift+Z** (KStd) | mainwindow:1841-1842 | [MVP] |
| `edit_cut` | Couper | **Ctrl+X** (KStd) | mainwindow:1853 | [MVP] |
| `edit_copy` | Copier | **Ctrl+C** (KStd) | mainwindow:1862 | [MVP] |
| `copy_location` | Copier l'emplacement | **Ctrl+Alt+C** | viewactionhandler:172-177 | [MVP] |
| `edit_paste` | Coller (texte dynamique, ex. « Coller un dossier ») | **Ctrl+V** (KStd) | mainwindow:1870 | [MVP] |
| `show_filter_bar` | Filtrer… | **Ctrl+I** et **/** | mainwindow:1913-1923 | [MVP] |
| `toggle_filter` | Afficher/masquer le filtre (variante de barre d'outils) | — | mainwindow:1928 | [v1] |
| `edit_find` | Rechercher… | **Ctrl+F** (KStd) | mainwindow:1937 | [v1] |
| `toggle_search` | Afficher/masquer la recherche (barre d'outils) | — | mainwindow:1949 | [v1] |
| `toggle_selection_mode` | Sélectionner des fichiers et dossiers | **Espace** | mainwindow:1964-1979 | [Plus tard] |
| `copy_to_inactive_split_view` | Copier vers l'autre vue | **Shift+F5** | mainwindow:1882-1892 | [MVP] |
| `move_to_inactive_split_view` | Déplacer vers l'autre vue | **Shift+F6** | mainwindow:1894-1904 | [MVP] |
| `edit_select_all` | Tout sélectionner | **Ctrl+A** (KStd) | mainwindow:1993 | [MVP] |
| `invert_selection` | Inverser la sélection | **Ctrl+Shift+A** | mainwindow:1998-2005 | [MVP] |

### 2.3 Affichage

| Nom interne | Libellé | Raccourci | Réf. | Prio |
|---|---|---|---|---|
| `view_zoom_in` / `view_zoom_out` | Zoom + / − | **Ctrl++** (et Ctrl+=) / **Ctrl+−** (KStd) | viewactionhandler:255,266 | [v1] |
| `view_zoom_reset` | Taille par défaut | **Ctrl+0** | viewactionhandler:258-263 | [v1] |
| `icons` / `compact` / `details` | Mode Icônes / Compact / Détails | **Ctrl+1** / **Ctrl+2** / **Ctrl+3** | viewactionhandler:783,794,805 | [MVP] (Détails + Icônes) |
| `show_preview` | Afficher les aperçus | **F12** | viewactionhandler:281 | [v1] |
| `show_hidden_files` | Afficher les fichiers cachés | **Ctrl+H**, **Alt+.** (KStd) | viewactionhandler:376-390 | [MVP] |
| `sort`, `group_by`, `additional_info`, `view_mode` | Sous-menus Trier, Grouper, Infos supplémentaires, Mode | — | viewactionhandler:234-375 | [MVP] (Trier) |
| `act_as_admin` | Agir en administrateur | **Ctrl+Shift+Alt+A** | workerintegration.cpp:36-39,127-134 | [Plus tard] |
| `split_view_menu` | Scinder / Fermer la vue gauche/droite | **F3** | mainwindow:2018-2029 | voir §4 |
| `split_view` | Bouton de barre d'outils (même action, avec menu « Détacher ») | — | mainwindow:2017 | — |
| `popout_split_view` | Détacher la vue gauche/droite dans une fenêtre | **Shift+F3** | mainwindow:2034-2040 | [Plus tard] |
| `focus_inactive_split_view` | Activer l'autre vue | **Ctrl+F3** | mainwindow:1906-1911 | [MVP] |
| `split_stash` | Stash (dossier virtuel `stash:/` dans la vue de droite) | **Ctrl+S** (visible seulement si le service `org.kde.kio.StashNotifier` existe) | mainwindow:2042-2050 | [Plus tard] |
| `view_redisplay` | Recharger | **F5** (KStd) | mainwindow:2052 | [MVP] |
| `stop` | Arrêter le chargement | — | mainwindow:2062 | [v1] |
| `editable_location` | Emplacement modifiable (bascule) | **F6** | mainwindow:2072-2077 | [MVP] |
| `replace_location` | Remplacer l'emplacement (éditer + tout sélectionner) | **Ctrl+L**, **Alt+D** | mainwindow:2079-2088 | [MVP] |
| `show_places_panel` | Panneau Emplacements | **F9** | mainwindow:2508 | [MVP] (barre latérale) |
| `show_information_panel` | Panneau Informations | **F11** | mainwindow:2359 | [v1] |
| `show_folders_panel` | Panneau Dossiers (arbre) | **F7** | mainwindow:2403 | [Plus tard] |
| `show_terminal_panel` | Panneau Terminal | **F4** | mainwindow:2443 | [Plus tard] |
| `focus_places_panel` | Activer/désactiver le focus des Emplacements | **Ctrl+P** | mainwindow:2563-2568 | [v1] |
| `focus_terminal_panel` | Focus Terminal | **Ctrl+Shift+F4** | mainwindow:2473-2477 | [Plus tard] |
| `lock_panels` | Verrouiller les panneaux | — | mainwindow:2329-2337 | [Plus tard] |

### 2.4 Aller à, signets, outils, configuration

| Nom interne | Libellé | Raccourci | Réf. | Prio |
|---|---|---|---|---|
| `go_back` | Précédent | **Retour arrière** (ajouté en tête), **Alt+←**, touche multimédia Back (KStd) | mainwindow:2090-2106 | [MVP] |
| `go_forward` | Suivant | **Alt+→** (KStd) | mainwindow:2141-2153 | [MVP] |
| `go_up` | Dossier parent | **Alt+↑** (KStd) | mainwindow:2160 | [MVP] |
| `go_home` | Dossier personnel | **Alt+Origine** (KStd) | mainwindow:2161 | [MVP] |
| `closed_tabs` | Onglets récemment fermés ▸ | — | mainwindow:2108-2112 | [v1] |
| `bookmarks` / `add_bookmark` / `edit_bookmarks` | Signets ▸ / Ajouter un signet / Éditer | `add_bookmark` : **Ctrl+B** (KStd) | dolphinbookmarkhandler.cpp:34-36 | [Plus tard] |
| `open_preferred_search_tool` | Ouvrir l'outil de recherche préféré | **Ctrl+Shift+F** | mainwindow:2181-2188 | [Plus tard] |
| `open_terminal` | Ouvrir un terminal (dossier courant) | **Shift+F4** | mainwindow:2197-2204 | [MVP] |
| `open_terminal_here` | Ouvrir un terminal ici (dossier des éléments sélectionnés) | **Shift+Alt+F4** | mainwindow:2206-2214 | [v1] |
| `compare_files` | Comparer les fichiers (Kompare, exactement 2 sélectionnés **sur les deux panneaux confondus**) | — | mainwindow:2169, 404-423 | [Plus tard] |
| `manage_disk_space` | Gérer l'espace disque ▸ | — | mainwindow:2175 | [Plus tard] |
| `options_show_menubar` | Afficher la barre de menus | **Ctrl+M** (KStd) | mainwindow:2226 | n/a |
| `options_configure_keybinding` | Configurer les raccourcis | **Ctrl+Alt+,** (KStd) | mainwindow:2241 | [v1] |
| `options_configure` | Configurer Satchel… | **Ctrl+Shift+,** (KStd) | mainwindow:2242 | [MVP] |
| `help_contents` / `help_whats_this` | Aide / Qu'est-ce que c'est ? | **F1** / **Shift+F1** (KStd) | KXmlGui | [Plus tard] |

### 2.5 Actions hors menus

| Nom interne | Effet | Raccourci | Réf. | Prio |
|---|---|---|---|---|
| `activate_tab_0…8` | Aller à l'onglet 1 à 9 | **Alt+1** à **Alt+9** | mainwindow:2250-2263 | [MVP] |
| `activate_last_tab` | Aller au dernier onglet | **Alt+0** | mainwindow:2265-2270 | [MVP] |
| `activate_next_tab` | Onglet suivant (circulaire) | **Ctrl+PgSuiv**, **Ctrl+]** (KStd), **Ctrl+Tab** | mainwindow:2244-2246, 2272-2277 | [MVP] |
| `activate_prev_tab` | Onglet précédent (circulaire) | **Ctrl+PgPréc**, **Ctrl+[** (KStd), **Ctrl+Shift+Tab** | mainwindow:2247-2249, 2279-2284 | [MVP] |
| `open_in_new_tab(s)` | Menu contextuel : ouvrir les dossiers sélectionnés dans de nouveaux onglets | — | mainwindow:2293-2302, 520-538 | [MVP] |
| `open_in_new_window` | Menu contextuel : ouvrir dans une nouvelle fenêtre | — | mainwindow:2303, 540-555 | [v1] |
| `open_in_split_view` | Menu contextuel : ouvrir dans l'autre vue | — | mainwindow:2308-2314, 557-581 | [MVP] |

Règles transverses :
- Les actions **Aller à l'onglet N**, **suivant**, **précédent** et **dernier** sont désactivées quand il n'y a qu'un seul onglet (`mainwindow:1707-1716`). **[MVP]**
- **Espace** n'active le mode sélection que si la vue a le focus et qu'aucune recherche par frappe n'est en cours. Sinon, Espace reste une touche normale (`mainwindow:601-611`, `views/dolphinview.cpp:2110-2113`).
- **Raccourci d'une action désactivée** : Dolphin affiche dans le panneau actif un message d'avertissement qui explique pourquoi l'action est désactivée, par exemple « Cannot paste: The clipboard is empty. », « Cannot copy to other view: No files selected. » ou « …The other view already contains these items. » (`mainwindow:184-187, 941-951, 2681-2731`, `disabledactionnotifier.h`). **[v1]** Très bon pour la découvrabilité.
- **Clic du milieu** sur Précédent, Suivant, Parent ou Personnel dans la barre d'outils : ouvre la destination dans un nouvel onglet en arrière-plan (`mainwindow:958-969, 1276-1299`, `middleclickactioneventfilter.cpp`). Même chose avec un clic du milieu sur une entrée du menu déroulant de l'historique (`mainwindow:1028-1034`). **[v1]**
- Les boutons Précédent et Suivant sont des boutons à **menu différé** (appui long). Le menu liste au plus **12** entrées d'historique, avec icône et libellé : chemin complet si l'option « chemin complet » est active, sinon « NomDeL'emplacement/sous/chemin » (`mainwindow:110, 971-1055`). **[v1]**

> ⚠ **Deux panneaux par défaut.**
> - **F3** n'a plus de sens comme « scinder ». Proposition : F3 bascule un **mode panneau unique** temporaire (le panneau actif occupe toute la largeur, l'autre est conservé) **[v1]**. Ou le laisser libre.
> - **Shift+F5 / Shift+F6** sont toujours disponibles. Ce sont les opérations principales d'un gestionnaire à deux panneaux : **[MVP]**.
> - Garder F5 = Recharger et F6 = barre d'adresse modifiable, comme Dolphin et Plasma. Ne pas reprendre F5 = copier / F6 = déplacer de Total Commander, cela contredirait les habitudes KDE. Une option de schéma de raccourcis « commander » est possible **[Plus tard]**.
> - **Tab** : voir §4.6.

**Notes pour Rust/Slint**
- Prévoir un registre d'actions central (`ActionId` → libellé, icône, raccourcis par défaut, `enabled(state) -> Result<(), DisabledReason>`). Les menus, la barre d'outils, le menu contextuel et la palette de commandes en dépendent. La raison de désactivation sert au message affiché quand on presse le raccourci.
- Persister les surcharges de raccourcis dans `~/.config/satchel/shortcuts.toml`. Les valeurs par défaut sont celles de cette table.

---

## 3. Onglets

### 3.1 Barre d'onglets
- **Masquée automatiquement quand il n'y a qu'un onglet** (`AlwaysShowTabBar` = `false`, `dolphintabwidget.cpp:30`, kcfg:45). **[MVP]**
- Bouton de fermeture sur chaque onglet (`ShowCloseButtonOnTabs` = `true`, `dolphintabwidget.cpp:31`). **[MVP]**
- Onglets **déplaçables par glisser** (`dolphintabbar.cpp:45`). **[v1]**
- Quand on ferme l'onglet courant, c'est l'**onglet précédent** qui est sélectionné (`setSelectionBehaviorOnRemove(SelectPreviousTab)`, `dolphintabbar.cpp:44`). **[MVP]**
- La barre d'onglets ne prend jamais le focus clavier (`NoFocus`, `dolphintabbar.cpp:48-49`).
- Style de largeur (`TabStyle`) : `AutoSize` par défaut, `FullWidth` (onglets qui se partagent toute la largeur, au moins 25 px chacun, sans flèches de défilement) ou `FixedSize` (225 px) (`dolphintabbar.cpp:60-83, 234-248`). **[Plus tard]**
- Des flèches de défilement apparaissent quand les onglets débordent (`dolphintabwidget.cpp:29`).

### 3.2 Titre, icône et infobulle d'un onglet
- Titre = **libellé du panneau actif** (`caption()`) : le nom de l'Emplacement si l'URL correspond exactement à un Emplacement, sinon le dernier segment du chemin, ou `/` pour la racine (`dolphinviewcontainer.cpp:586-630`).
- En vue scindée : `Actif | (Inactif)` ou `(Inactif) | Actif`, toujours dans l'ordre gauche | droite. Les parenthèses entourent le panneau **inactif** (`dolphintabwidget.cpp:533-558`).
- Le titre est tronqué à **40 caractères** avec « … » au milieu (`rsqueeze`, `:557`). Une ellipse est ajoutée à droite, ou à gauche si l'option « chemin complet dans la barre de titre » est active (`:108-123`).
- Un **titre personnalisé** (menu « Renommer l'onglet ») remplace le titre calculé et survit à la navigation (`:125-136, 497-501`). **[v1]**
- Infobulle = chemin complet du panneau actif. Icône = icône de dossier de l'URL (`:451-473`). **[MVP]**

### 3.3 Ouvrir un onglet
| Geste | Comportement | Réf. | Prio |
|---|---|---|---|
| **Ctrl+T** / bouton | Nouvel onglet **sur le dossier du panneau actif**, toujours **en dernière position** quel que soit le réglage, activé. Il reprend l'état « éditable » de la barre d'adresse. Le focus va à la vue. | mainwindow:459-466, tabwidget:148-171 | [MVP] |
| Double-clic sur une zone vide de la barre d'onglets | Duplique l'onglet courant | tabbar:158-173 | [v1] |
| Double-clic sur un onglet | Duplique cet onglet | tabbar:158-173 | [v1] |
| Clic du milieu sur un dossier | Nouvel onglet **en arrière-plan**. **Shift**+clic du milieu : onglet activé | views/dolphinview.cpp:1282-1305 | [MVP] |
| **Ctrl**+clic (activation) sur un dossier | Onglet en arrière-plan. **Ctrl+Shift** : onglet activé. **Shift** : nouvelle fenêtre | dolphinviewcontainer.cpp:758-779 | [MVP] |
| Activation de plusieurs dossiers sélectionnés | Un onglet par dossier (Shift seul : fenêtres). Les fichiers sont ouverts avec leur application | views/dolphinview.cpp:1255-1280 | [v1] |
| Menu contextuel « Ouvrir dans un nouvel onglet » | Un onglet par dossier sélectionné. S'il n'y en a aucun, l'onglet s'ouvre sur le dossier courant | mainwindow:520-538 | [MVP] |
| Clic du milieu sur un fichier | Ouvre avec la **2ᵉ** application associée (Shift : la 3ᵉ). Pour une archive sans 2ᵉ application : l'ouvre en onglet | dolphinviewcontainer.cpp:789-833 | [Plus tard] |

- **Position d'un onglet ouvert en arrière-plan** : juste **après l'onglet courant** par défaut (`OpenNewTabAfterLastTab` = `false`, `tabwidget:185-218`). Le courant ne change pas, donc plusieurs ouvertures successives s'insèrent toutes en `courant+1` et **s'empilent en ordre inverse**. Pour Satchel, il vaut mieux insérer après le dernier onglet ouvert depuis le courant, comme le font les navigateurs **[v1]**.
- Un onglet ouvert en arrière-plan **ne vole pas le focus** : le widget qui avait le focus le récupère (`tabwidget:212-216`).

### 3.4 Fermer
- **Ctrl+W**, bouton ×, **clic du milieu sur l'onglet** (déclenché au relâchement, seulement si on relâche sur le même onglet que celui pressé) (`tabbar:133-156`). **[MVP]**
- **Fermer le dernier onglet ferme la fenêtre** (`tabwidget:306-322`). **[MVP]**
- Menu contextuel d'un onglet : Nouvel onglet · Détacher l'onglet · ─ · Renommer l'onglet · ─ · Fermer les autres onglets · Fermer les onglets à gauche (grisé sur le premier) · Fermer les onglets à droite (grisé sur le dernier) · Fermer l'onglet (`tabbar:175-232`). **[v1]** (Détacher : **[Plus tard]**)
- **Détacher** : lance `dolphin <url1> [<url2> --split] --new-window`, puis ferme l'onglet. Les historiques sont perdus (`tabwidget:396-415`).
- **Quitter avec plusieurs onglets** : un dialogue s'affiche seulement si `ConfirmClosingMultipleTabs` (`true`) **et** si la restauration de session est désactivée. Boutons : « Quitter » (par défaut), « Fermer l'onglet courant », « Annuler », avec une case « Ne plus demander » (`mainwindow:632-680`). Comme la restauration est active par défaut, **le dialogue n'apparaît pas en configuration par défaut**. **[v1]**

### 3.5 Rouvrir un onglet fermé
- Chaque fermeture d'onglet (pas la fermeture de la fenêtre) mémorise l'**état complet** de l'onglet : scindé ou non, URL des deux panneaux, état éditable des barres d'adresse, élément courant, sélection, position de défilement, dossiers dépliés, panneau actif, position du séparateur, titre personnalisé (`tabwidget:318`, `tabpage:302-330`, `views/dolphinview.cpp:1792-1817`). **L'historique précédent/suivant n'est pas conservé.**
- **Ctrl+Shift+T** rouvre le plus récent. L'onglet rouvert est inséré **après l'onglet courant** et activé, pas à sa position d'origine (`tabwidget:348-352`). **[MVP]**
- Menu « Onglets récemment fermés » : au plus **6** entrées, la plus récente en premier, libellées par le chemin du panneau actif avec son icône. Il contient aussi « Vider les onglets récemment fermés ». Le menu et l'action sont grisés quand la liste est vide (`dolphinrecenttabsmenu.cpp:17-89`, `mainwindow:1647-1650`). **[v1]**
- La liste vit **en mémoire seulement** : elle est perdue quand on quitte.

### 3.6 Glisser-déposer sur la barre d'onglets
- Survoler un onglet avec des URL **pendant 800 ms** active cet onglet, pour pouvoir déposer dans sa vue (`tabbar:51-54, 85-116, 250-269`). **[v1]**
- **Déposer sur un onglet** = déposer dans le **dossier du panneau actif** de cet onglet. C'est le dépôt standard, avec le menu Copier/Déplacer/Lier selon les modificateurs (`tabwidget:424-440`). **[v1]**
- **Déposer sur la zone vide** de la barre : chaque URL qui est un **dossier** ouvre un nouvel onglet **à la fin**. Les fichiers sont ignorés (`tabwidget:441-449`). **[v1]**

### 3.7 Navigation entre onglets
Voir §2.5. Le parcours est circulaire (`tabwidget:336-346`). Aller à l'onglet N ne fait rien si N n'existe pas (`:324-329`).

> ⚠ **Deux panneaux par défaut : modèle d'onglets à trancher.**
> - **Option A (Dolphin)** : un onglet = une paire de panneaux. La barre d'onglets est unique, au-dessus des deux panneaux, avec des titres « A | (B) ». C'est simple et fidèle à la source.
> - **Option B (ForkLift, Total Commander)** : **chaque panneau a sa propre barre d'onglets**, et Ctrl+T ouvre un onglet dans le panneau actif. Cela correspond mieux à un usage permanent à deux panneaux, où l'on change de dossier d'un côté sans toucher l'autre.
> - **Recommandation : Option B pour le MVP.** Toutes les règles de §3 s'appliquent alors **par panneau**. Avec un seul onglet dans un panneau, sa barre est masquée. Glisser un onglet d'un panneau vers l'autre est **[Plus tard]**.

---

## 4. Vue double (split view)

### 4.1 Ouvrir
- **F3**, ou le bouton « Scinder ». Le nouveau panneau (secondaire, à droite) s'ouvre sur le **dossier sélectionné** si exactement **un** dossier ou une archive navigable est sélectionné. Sinon il s'ouvre sur **le même dossier** que le panneau actif (`mainwindow:1082-1095`, `tabpage:105-108`).
- **Le nouveau panneau devient actif** et prend le focus (`tabpage:121`).
- Animation : le panneau s'élargit de 1 px jusqu'à la **dernière largeur choisie par l'utilisateur**, ou jusqu'à **50 %** si aucune n'a été choisie. Courbe OutCubic, durée = 2 × la durée d'animation du style. L'animation est désactivée si le facteur d'animation global vaut 0 (`tabpage:94-97, 123-130, 578-601`).
- Menu contextuel « Ouvrir dans la vue scindée » sur un dossier : si la vue est déjà scindée, **on bascule sur l'autre panneau** et on y charge le dossier. Sinon, on ouvre la vue scindée sur ce dossier (`mainwindow:557-581`). **[MVP]**

### 4.2 Fermer
- **F3** quand la vue est scindée. Le libellé devient « Fermer la vue gauche » ou « Fermer la vue droite », avec l'icône correspondante (`mainwindow:2831-2920`).
- **Quel panneau se ferme** : réglage `CloseSplitViewChoice`, `ActiveView` par défaut (kcfg:118-126). Valeurs : le panneau **actif** se ferme (défaut), le panneau **inactif** se ferme, ou toujours le panneau **de droite**. Le panneau qui reste devient primaire et actif (`tabpage:132-184`).
- Animation de fermeture : courbe InCubic jusqu'à toute la largeur. Pendant l'animation, le panneau fermé est désactivé et ne réagit plus (`tabpage:171-181`).
- Si l'on fermait le panneau cible d'un mode sélection « Copier/Déplacer vers l'autre vue », ce mode se termine (`selectionmode/bottombar.cpp:63-70`).
- **Shift+F3** (Détacher) : ouvre le panneau désigné par le même réglage dans une **nouvelle fenêtre**, puis ferme la vue scindée (`mainwindow:1097-1119`).

### 4.3 Panneau actif : comment il change
- **Cliquer dans un panneau** ou lui donner le focus le rend actif : le `FocusIn` du conteneur appelle `setActive(true)` (`views/dolphinview.cpp:1116-1120`). **[MVP]**
- Cliquer dans la barre d'adresse d'un panneau l'active aussi (`dolphinviewcontainer.cpp:303`).
- **Ctrl+F3** : si la vue est scindée, active l'autre panneau. Sinon, **ouvre** la vue scindée (`tabwidget:386-394`). **[MVP]**
- Activer un élément (double-clic ou glisser-déposer) dans un panneau inactif rend ce panneau actif (`dolphinviewcontainer.cpp:760-763`).
- Un changement d'URL d'origine externe (redirection) n'active pas un panneau inactif (`dolphinviewcontainer.cpp:971-984`).
- Changer d'onglet conserve le panneau actif de chaque onglet (`tabpage:403-421`).

### 4.4 Indication visuelle du panneau actif
- Dans Dolphin, le **fond du panneau inactif est le même fond de vue avec une opacité de 150/255** (≈ 59 %). Le panneau actif a un fond opaque (`views/dolphinview.cpp:1073-1088`). Quand la fenêtre perd le focus, la palette « Inactive » est utilisée pour les deux panneaux (`:1104-1107`).
- La barre d'adresse du panneau actif reçoit un indicateur de focus accentué, mais **seulement quand la vue est scindée** (`setHighlightFocusIndicator`, `dolphinnavigatorswidgetaction.cpp:150-153`).
- Le titre de l'onglet met le panneau inactif entre parenthèses (§3.2).

> ⚠ **Deux panneaux par défaut.** C'est **[MVP]** et l'indication doit rester lisible en permanence. Le design (`05-design.md`) prévoit un **liseré d'accent** au-dessus du panneau actif. On peut l'ajouter à la légère atténuation du fond inactif, à garder subtile en Catppuccin Mocha (par exemple la couleur `mantle` plutôt que `base`). La sélection du panneau inactif reste visible, avec une couleur désaturée.

### 4.5 Synchronisation des URL
- **Aucune synchronisation** : chaque panneau navigue indépendamment, avec son propre historique (`dolphinviewcontainer.cpp:73, 122-125`). **[MVP]**
- Seule exception, le démarrage ou l'ouverture avec `--split` : les URL passées sont **appariées** deux par deux, gauche puis droite (`tabwidget:220-251`). Avec `--split` et une seule URL, les deux panneaux montrent la même (`main.cpp:214-217`).
- Les dimensions des barres d'adresse suivent celles des panneaux : chaque fil d'Ariane est aligné au-dessus de son panneau (`dolphinnavigatorswidgetaction.cpp:45-99`).
- Option « synchroniser la navigation » (le panneau inactif suit le panneau actif dans le sous-dossier de même nom) : **absente de Dolphin**. **[Plus tard]**

### 4.6 Touche Tab
- **Dolphin n'a aucun raccourci Tab pour changer de panneau.** Le signal `toggleActiveViewRequested` est déclaré (`views/dolphinview.h:732`) et connecté à `switchActiveView` (`tabpage:573`, `:537-547`), mais **il n'est jamais émis** dans 26.08.3. Tab suit donc la chaîne de focus Qt standard. Pour changer de panneau au clavier, on utilise **Ctrl+F3**.
- > ⚠ **Deux panneaux par défaut.** **Tab = changer de panneau actif** (et Shift+Tab aussi, puisqu'il n'y a que deux panneaux) **[MVP]**. C'est la convention des gestionnaires à deux panneaux. Exceptions : Tab garde son rôle normal dans les champs texte (barre d'adresse éditable, filtre, renommage en ligne, recherche), et pendant le renommage en ligne Tab passe à l'élément suivant (comportement de `kitemlistroleeditor.cpp:90-100`). On garde aussi Ctrl+F3 comme alias.

### 4.7 Copier / déplacer vers l'autre vue
- **Shift+F5** copie et **Shift+F6** déplace la **sélection du panneau actif** vers le **dossier courant du panneau inactif** (`tabwidget:354-384`). **[MVP]**
- **Pas de confirmation** : l'opération démarre directement (`KIO::copy` / `KIO::move` avec drapeaux par défaut, `views/dolphinview.cpp:948-968`). Les conflits de nom sont gérés par le dialogue standard d'écrasement de KIO (voir le doc des opérations). L'opération est **enregistrée pour l'annulation** (Ctrl+Z).
- Après la copie, **les éléments créés sont sélectionnés dans le panneau de destination** et le premier devient l'élément courant (`views/dolphinview.cpp:954-956`). **[v1]**
- Conditions d'activation (`mainwindow:2681-2731`) :
  - la vue n'est pas scindée → action grisée, sans message ;
  - la sélection est vide → grisée, message « …No files selected. » ;
  - un élément sélectionné se trouve **déjà dans le dossier de destination** → grisée (« …The other view already contains these items. ») ;
  - la destination n'est pas inscriptible → grisée (« …You do not have permission to write into the destination folder. ») ;
  - Déplacer exige aussi que la source supporte le déplacement.
- Particularité : si l'action est déclenchée sans sélection (par le menu), Dolphin entre en **mode sélection** avec une barre « Sélectionnez les fichiers et dossiers à copier/déplacer » et un bouton « Copier vers l'autre vue » (`mainwindow:1128-1146`). **[Plus tard]**
- Équivalents : glisser-déposer d'un panneau à l'autre (menu Copier/Déplacer/Lier) **[MVP]**, et Couper/Coller via le presse-papiers **[MVP]**.

### 4.8 Séparateur
- Les deux panneaux ne peuvent pas être réduits à zéro (`setChildrenCollapsible(false)`, `tabpage:39`). **[MVP]**
- **Double-clic sur la poignée** : retour à **50/50**. Un « double relâchement » rapide a le même effet (`tabpage:609-628`). **[MVP]**
- La position est mémorisée par onglet et restaurée avec la session (`tabpage:322-323, 389-394`).

**Notes pour Rust/Slint**
- Ratio du séparateur en `f32` (0,5 par défaut), avec un minimum en px par panneau. Persister le ratio par fenêtre si le modèle d'onglets B est retenu (§3), sinon par onglet.
- `Tab` : à chaque appui, si le focus est dans une vue de fichiers (pas dans un `TextInput`), basculer `active` et donner le focus à la vue de l'autre panneau.

---

## 5. Navigation

### 5.1 Historique précédent / suivant (par panneau)
- Chaque panneau a son propre historique (`urlNavigatorInternalWithHistory`, `dolphinviewcontainer.cpp:73, 260-268`). Les actions agissent sur le **panneau actif** (`mainwindow:1249-1274`). **[MVP]**
- Précédent et Suivant sont **grisés** aux extrémités de l'historique (`mainwindow:428-446`).
- **L'état de la vue est mémorisé par entrée d'historique** : juste avant de quitter un dossier, Dolphin enregistre l'élément courant, la sélection, le défilement et les dossiers dépliés, puis les **restaure au retour** (Précédent ou Suivant) (`dolphinviewcontainer.cpp:888-891, 909-910, 1119-1134`). **[MVP]** C'est essentiel pour retrouver sa position.
- Les URL de redirection ont un état vide et **sont sautées** quand on revient en arrière (`mainwindow:1249-1259`, `dolphinviewcontainer.cpp:955-985`).
- Les **boutons latéraux de la souris** (Précédent/Suivant) et le balayage horizontal sur écran tactile naviguent dans l'historique. Un balayage vers le haut remonte au dossier parent (`views/dolphinview.cpp:1588-1592`, `kitemviews/kitemlistcontroller.cpp:1238-1246`). **[MVP]** (boutons souris)
- **Retour arrière** = Précédent. Il est placé en premier raccourci pour être affiché dans les menus (`mainwindow:2103-2106`). **[MVP]**
- La taille maximale de l'historique est fixée par KUrlNavigator (KF6). Viser **100 entrées par panneau**.

### 5.2 Dossier parent et dossier personnel
- **Alt+↑** remonte au parent. L'action est grisée à la racine, quand `upUrl == url` (`mainwindow:2746-2762`). Le dossier qu'on vient de quitter est sélectionné et défilé à l'écran dans le parent (KUrlNavigator émet `urlSelectionRequested`, `dolphinviewcontainer.cpp:939-943`). **[MVP]**
- **Alt+Origine** va au dossier personnel configuré (`HomeUrl`, par défaut `$HOME`, kcfg:73-76, `global.cpp:37-40`). **[MVP]**
- Clic du milieu sur Parent ou Personnel dans la barre d'outils : ouvre dans un nouvel onglet (§2.5).

### 5.3 Barre d'adresse : fil d'Ariane
Comportements de `KUrlNavigator` (KF6), configurés par `dolphinurlnavigator.cpp:29-60`.
- Par défaut, mode **fil d'Ariane** (`EditableUrl` = `false`). Chaque segment est un bouton cliquable. **[MVP]**
- Le chemin est **relatif à l'Emplacement le plus proche** : par exemple « ⌂ Dossier personnel › Documents ». L'option `ShowFullPath` (`false`) affiche le chemin depuis `/` (kcfg:33-35). **[MVP]**
- **Flèche ▸ entre deux segments** (KF6) : menu déroulant des sous-dossiers de ce niveau. Le sous-dossier qui fait partie du chemin courant est mis en évidence. Les dossiers cachés n'y apparaissent que si « afficher les fichiers cachés » est actif dans le panneau, et ils sont triés en dernier si la vue les trie en dernier (`dolphinviewcontainer.cpp:288-289, 1082-1094`). **[v1]**
- **Clic du milieu** ou **Ctrl+clic** sur un segment : nouvel onglet. **Ctrl+Shift+clic** : onglet activé. **Shift+clic** : nouvelle fenêtre. Ce sont les mêmes règles que pour les dossiers de la vue (les commentaires « keep in sync with KUrlNavigator::slotNavigatorButtonClicked » sont dans `dolphinviewcontainer.cpp:768`, `views/dolphinview.cpp:1265`, et le branchement dans `mainwindow:2820-2822`). **[v1]**
- **Glisser-déposer sur un segment** : dépose dans ce dossier, avec le menu standard (`dolphinviewcontainer.cpp:298-300`). Pendant un glisser, survoler un segment ouvre le dossier (KF6). **[v1]**
- **Badge « lecture seule »** (cadenas 12 px, infobulle « This folder is not writable for you. ») à la fin du fil quand le dossier local n'est pas inscriptible. Il est masqué pendant le chargement et réévalué à la fin (`dolphinurlnavigator.cpp:55-59`, `dolphinviewcontainer.cpp:692-694, 709-711`). **[v1]**
- Le **sélecteur d'Emplacements** (icône à gauche du fil) n'est visible que si le panneau Emplacements est masqué (`dolphinurlnavigatorscontroller.cpp:30-39`). **[v1]**
- Boutons contextuels à droite du fil : « Vider la corbeille » dans `trash:/` (grisé si elle est vide), « Ajouter un dossier réseau » dans `remote:/`, et un bouton KDE Connect (`dolphinnavigatorswidgetaction.cpp:207-268`). **[v1]** (corbeille)
- **Échap** dans le fil d'Ariane (mode non éditable) rend le focus à la vue (`dolphinurlnavigator.cpp:149-156`). **[MVP]**

### 5.4 Barre d'adresse : mode éditable
- **F6** bascule entre fil d'Ariane et champ texte, pour le panneau actif (`mainwindow:1191-1196`). **[MVP]**
- **Ctrl+L / Alt+D** passe en mode éditable, donne le focus au champ et **sélectionne tout le texte**. Si le champ a déjà le focus avec tout le texte sélectionné, une nouvelle pression **revient au fil d'Ariane et rend le focus à la vue** (`mainwindow:1198-1214`). **[MVP]**
- **Entrée** valide. Si le mode éditable n'est pas le mode permanent configuré, la barre **revient au fil d'Ariane** (`dolphinurlnavigator.cpp:142-147`, `dolphinviewcontainer.cpp:895-897`). Le focus passe ensuite à la vue (`dolphinviewcontainer.cpp:912-917`). **[MVP]**
- **Cliquer sur le fond vide de la vue** referme le mode éditable s'il n'est pas permanent (`mainwindow:773-782`). **[v1]**
- Ctrl+A dans le champ (avec le focus) sélectionne le texte et non les fichiers (`mainwindow:1062-1075`). **[MVP]**
- Saisie : tout format accepté par `QUrl::fromUserInput`, comme `~/x`, un chemin absolu ou une URL avec schéma (KF6).
- **Complétion** : mode `CompletionPopupAuto` par défaut (liste déroulante qui complète automatiquement). Le mode choisi par l'utilisateur (clic droit dans le champ) est **partagé par toutes les barres** et persisté (`dolphinurlnavigator.cpp:37`, `dolphinurlnavigatorscontroller.cpp:46-65`, kcfg:29-31). **[MVP]** (complétion des chemins locaux)
- L'état éditable (texte en cours, curseur, sélection, focus) est **conservé quand on change d'onglet** et recopié dans un nouvel onglet (`dolphinurlnavigator.cpp:86-113`, `tabwidget:148-171`). **[v1]**
- Erreurs de saisie (`dolphinviewcontainer.cpp:893-937`) :
  - schéma `http(s)` : l'URL s'ouvre dans le navigateur, avec le message « Dolphin does not support web pages, the web browser has been launched » ;
  - autre protocole non listable : application par défaut, avec le message « Protocol not supported… » ;
  - protocole inconnu : message d'erreur « Invalid protocol '%1' », puis retour automatique à l'URL précédente ;
  - **fichier saisi à la place d'un dossier** : le panneau ouvre le dossier parent et affiche « Trying to open a file as a folder: … » avec un bouton **« Ouvrir comme fichier »** (`:730-756`) **[v1]** ;
  - **dossier local inexistant** : erreur avec un bouton **« Créer le dossier manquant »**, grisé si le premier parent existant n'est pas inscriptible (`:1039-1065`) **[v1]**.

### 5.5 Dossier courant supprimé ou démonté
- Si le dossier affiché est supprimé, le panneau remonte au **plus proche ancêtre existant** et affiche « Current location changed, <dossier> is no longer accessible. » (`dolphinviewcontainer.cpp:1096-1110, 1136-1144`). **[MVP]**
- Au démontage d'un périphérique, tous les panneaux affichés sous ce point de montage passent au dossier personnel (`mainwindow:1770-1783`). **[v1]**

### 5.6 Titre de fenêtre
- Titre = libellé du **panneau actif**. Avec l'option « chemin complet dans la barre de titre » (`false`), c'est le chemin complet (`mainwindow:1718-1724`, `dolphinviewcontainer.cpp:574-584`). **[MVP]**

**Notes pour Rust/Slint**
- `History { entries: Vec<(Url, ViewState)>, index }` avec `ViewState { current: Option<PathBuf>, selection: Vec<PathBuf>, scroll: (f32, f32), expanded: Vec<PathBuf> }`. Sauvegarder l'état sur `about_to_change`, le restaurer après le chargement.
- Le fil d'Ariane est un composant Slint : un `HorizontalLayout` de boutons et de flèches ▸, chaque flèche ouvrant un `PopupWindow` de sous-dossiers lus de façon asynchrone. Quand c'est trop long, élider par la gauche avec « … » cliquable.

---

## 6. Barre de filtre

- Elle s'ouvre avec **Ctrl+I** ou **/**, avec une animation de hauteur. Elle est placée **sous la vue, au-dessus de la barre d'état**. Elle reçoit le focus et son texte est entièrement sélectionné (`dolphinviewcontainer.cpp:642-654`, `filterbar/filterbar.cpp:185-190`). **[MVP]**
- Elle appartient à **chaque panneau** et filtre seulement celui-ci. Elle est fermée par défaut (`FilterBar` = `false`). Le réglage « afficher la barre de filtre au démarrage » l'ouvre dans les nouveaux panneaux (`dolphinviewcontainer.cpp:108-117`). **[MVP]**
- Composition, de gauche à droite (`filterbar.cpp:26-97`) :
  1. **Cadenas** « Conserver le filtre en changeant de dossier » (bascule). **[v1]**
  2. **Champ** avec le texte indicatif « Filter… » et un bouton d'effacement. **[MVP]**
  3. **Respecter la casse** (bascule, désactivée par défaut). **[v1]**
  4. **Mode** : Texte simple · **Motif glob (par défaut)** · Expression régulière. **[v1]** (MVP : texte simple insensible à la casse, `*` et `?` acceptés)
  5. **Fermer** (×).
- La casse et le mode sont **mémorisés** dans le fichier d'état, groupe `[FilterBar]` (`filterbar.cpp:92-104`).
- **Motif invalide** (glob ou regex) : fond du champ en couleur « négative » et icône d'erreur avec l'infobulle « Invalid expression » (`filterbar.cpp:152-183`). **[v1]**
- Filtrage **en direct** à chaque frappe (`textChanged`, `filterbar.cpp:44`). L'infobulle de survol est masquée et la barre d'état est mise à jour (`dolphinviewcontainer.cpp:864-881`).
- Clavier (`filterbar.cpp:192-225`) **[MVP]** :
  - **Échap** : efface le texte s'il y en a, sinon ferme la barre ;
  - **Entrée** : rend le focus à la vue sans fermer la barre ;
  - **↑ ↓ PgPréc PgSuiv** : rendent le focus à la vue **et transmettent la touche**, ce qui permet de taper puis de descendre directement dans les résultats.
- **Changer de dossier efface le filtre**, sauf si le cadenas est verrouillé (`dolphinviewcontainer.cpp:120`, `filterbar.cpp:135-140`). Déverrouiller le cadenas efface aussi le texte (`:142-150`).
- Fermer la barre efface le texte, déverrouille le cadenas et rend le focus à la vue (`filterbar.cpp:106-113`, `dolphinviewcontainer.cpp:852-857`).
- Cliquer dans les Emplacements sur l'emplacement **déjà affiché** efface le filtre et recharge la vue (`mainwindow:1628-1645`). **[v1]**
- Ordre de tabulation : cadenas → casse → mode → fermer → champ (`filterbar.cpp:90`).

---

## 7. Mode sélection **[Plus tard]**

Ce mode est surtout pensé pour le tactile. Pour Satchel, dont l'usage est clavier/souris, il vaut mieux le garder pour plus tard. On note seulement l'essentiel.
- On l'active avec **Espace** (vue focalisée, pas de recherche par frappe en cours), avec le bouton « Sélectionner » ou par un **appui long** sur un élément (`mainwindow:1169-1175`, `kitemviews/kitemlistcontroller.cpp:1165-1178`).
- **Barre haute** : « Selection Mode: Click on files or folders to select or deselect them. » (version courte « Selection Mode » quand la place manque), un bouton d'aide contextuelle et un bouton « Exit Selection Mode » (`selectionmode/topbar.cpp:30-53`).
- Règles : un clic bascule la sélection d'un élément, un clic dans le vide **ne désélectionne pas**, le lasso **inverse** la sélection, les flèches ne changent pas la sélection, **Entrée**, **Retour** et **Ctrl+Espace** basculent la sélection, et **Échap** quitte le mode (`topbar.cpp:36-49`, `kitemlistcontroller.cpp:287-288, 446-449`).
- **Barre basse** générale : Copier, Couper, Renommer, Corbeille (Supprimer dans `trash:/`), puis les actions du menu contextuel, Inverser la sélection et Tout sélectionner. Ce qui ne tient pas passe dans un bouton « More » (`selectionmode/bottombarcontentscontainer.cpp:500-566`).
- Barres « guidées » : Couper, Copier, Supprimer, etc. **déclenchés sans sélection** ouvrent le mode avec une consigne (« Select the files and folders that should be copied. ») et un bouton principal « Copier %2 dans le presse-papiers » qui reste grisé tant que rien n'est sélectionné (`:168-310, 612-670`, `mainwindow:904-921`).
- Après Copier ou Couper depuis la barre, un **rappel « Coller depuis le presse-papiers »** reste affiché (`ShowPasteBarAfterCopying` = `true`), avec les boutons « Dismiss This Reminder » et « Don't Remind Me Again » (`bottombarcontentscontainer.cpp:181-196, 414-450`, `bottombar.cpp:47-50`).
- Changer de dossier quitte le mode (`dolphinviewcontainer.cpp:430-432`).

---

## 8. Barre d'état

### 8.1 Disposition
- **Une barre d'état par panneau** (`dolphinviewcontainer.cpp:146-199`).
- Réglage `ShowStatusBar` (kcfg:142-150), défaut **`Small`** :
  - **Small** (défaut) : une **pastille flottante** en bas à gauche de la vue, au-dessus de la barre de défilement horizontale. Sa largeur s'ajuste au texte, avec un cadre à coins arrondis de 5 px. Elle **n'affiche que le texte**, sans espace libre ni zoom, et **disparaît quand le texte est vide** (`dolphinviewcontainer.cpp:1146-1191`, `statusbar/dolphinstatusbar.cpp:272-301, 312-334, 442-470`) ;
  - **FullWidth** : une barre pleine largeur sous la vue, avec texte · [Zoom : curseur] (optionnel, `ShowZoomSlider` = `false`) · **espace disque** ;
  - **Disabled** : pas de barre.
- Menu contextuel (FullWidth seulement) : « Afficher le curseur de zoom » (`dolphinstatusbar.cpp:336-359`).

> ⚠ **Deux panneaux par défaut.** Le design de Satchel met le résumé (« 19 éléments, 42 Go libres » / « 1 sur 9 sélectionné (1,5 Mo) ») **dans l'en-tête de chaque panneau**, sous le fil d'Ariane. Cela remplace la barre d'état **[MVP]**. Les priorités de texte décrites ci-dessous (survol, message temporaire, texte par défaut) s'appliquent à cet emplacement.

### 8.2 Texte : contenu exact
La priorité d'affichage est : **message temporaire > élément survolé > texte par défaut** (`dolphinstatusbar.cpp:398-410`).
- **Texte par défaut, sans sélection** : on compte les éléments **visibles** du modèle (donc après filtre, et en incluant les sous-éléments dépliés en vue Détails) (`views/dolphinview.cpp:1681-1708, 766-792`) **[MVP]** :
  - dossiers et fichiers : `« N dossiers, M fichiers (taille) »` ;
  - seulement des fichiers : `« M fichiers (taille) »` ;
  - seulement des dossiers : `« N dossiers »` (sans taille) ;
  - rien : `« 0 dossiers, 0 fichiers »`.
  - Les formes au singulier sont « 1 dossier » et « 1 fichier ». La **taille est la somme des fichiers seulement**, ou la taille récursive si le protocole la fournit (`UDS_RECURSIVE_SIZE`). Elle est formatée par `KFormat::formatByteSize`, qui utilise les unités binaires de KDE, par exemple « 1,5 Mio » selon la locale.
- **Un seul élément sélectionné** : la description KIO de l'élément (`KFileItem::getStatusBarInfo()` : nom, type, taille ou cible de lien) (`views/dolphinview.cpp:748-751`). **[MVP]**
- **Au moins deux éléments sélectionnés** : `« N dossiers sélectionnés, M fichiers sélectionnés (taille) »`, avec les mêmes variantes que ci-dessus. La taille ne compte que les fichiers, **les dossiers ne sont pas mesurés** (`:732-755, 774-776`). **[MVP]**
- **Survol** d'un élément : sa description KIO remplace temporairement le texte par défaut. Le texte est élidé au milieu (`dolphinviewcontainer.cpp:843-850`, `dolphinstatusbar.cpp:405-407`). **[v1]**
- **Messages temporaires** (texte riche) affichés **1 s** : messages d'information de la vue, « opération terminée », retour de la recherche par frappe (`dolphinviewcontainer.cpp:152-155`, `dolphinstatusbar.cpp:37-38, 212-221`). **[v1]**
- Recherche sans résultat : « No items found. » (`dolphinviewcontainer.cpp:701-704`).
- Rythme de mise à jour : le texte change au plus toutes les **50 ms**. Le recalcul est **retardé de 300 ms** après un changement d'éléments ou de sélection, et **forcé** si rien n'a été mis à jour depuis plus de **2 s** (`dolphinstatusbar.cpp:37, 117-120`, `dolphinviewcontainer.cpp:171-174, 656-676`). **[MVP]** Cela évite le scintillement pendant les copies.

### 8.3 Espace libre
- FullWidth seulement : une **jauge de capacité** avec le pourcentage utilisé, et un bouton « **%1 libres** » (par exemple « 42,3 Gio libres »). Infobulle : « %1 libres sur %2 (%3 % utilisés) ». Le bouton ouvre le menu « Gérer l'espace disque ». La jauge est masquée si la taille du volume vaut 0, par exemple pour un protocole sans information (`statusbar/statusbarspaceinfo.cpp:113-145`). **[MVP]** (sous forme de « N Go libres » dans l'en-tête de panneau)
- Rafraîchissement : à chaque changement d'URL, sur F5, puis **toutes les 60 s** par point de montage, avec un cache partagé par point de montage (`statusbar/mountpointobservercache.cpp:70`, `dolphinviewcontainer.cpp:567-572`).

### 8.4 Progression
- Le chargement d'un dossier affiche une progression : texte (« Loading folder… », « Sorting… », « Searching… ») + barre + bouton **Stop**. **La barre n'apparaît qu'après 500 ms**, ce qui évite le clignotement sur les dossiers rapides. Le pourcentage est indéterminé au départ, puis réel (`dolphinviewcontainer.cpp:156-161, 678-695`, `dolphinstatusbar.cpp:106-109, 168-200, 371-387`). **[MVP]** (indicateur seulement) / **[v1]** (Stop)
- Pendant la progression, le zoom et l'espace libre sont masqués. En mode Small, la pastille prend toute la largeur (`dolphinstatusbar.cpp:286-290, 378`).
- Les **copies et déplacements** n'apparaissent pas ici : ils passent par les notifications de tâches de Plasma (KIO). Pour Satchel, un panneau de transferts est prévu dans un autre document.

### 8.5 Barre de messages (au-dessus de la vue)
- Une zone de message par panneau, de type information, avertissement ou erreur, avec un bouton de fermeture, des boutons d'action optionnels et une apparition animée. Le texte passe à la ligne s'il dépasse la largeur. Elle est **masquée automatiquement** quand on change de dossier et quand on recharge (`dolphinviewcontainer.cpp:96-99, 121, 486-523, 570`). **[MVP]**
- Avertissement permanent quand Dolphin tourne en root : « Running Dolphin as root can be dangerous… » (`:101-106`).

---

## 9. Ligne de commande, D-Bus, instance unique

### 9.1 Options (`main.cpp:132-161`)
| Option | Effet | Prio |
|---|---|---|
| `[URL…]` | Dossiers à ouvrir. Les chemins relatifs sont résolus depuis le dossier courant (`QUrl::fromUserInput(..., AssumeLocalFile)`, `global.cpp:22-35`). **Un onglet par URL.** | [MVP] |
| `--select` | Les arguments sont des **fichiers ou dossiers à sélectionner** : on ouvre leur dossier parent (un onglet par parent distinct), on sélectionne les éléments et le premier devient l'élément courant, défilé à l'écran (`tabwidget:253-299`) | [MVP] |
| `--split` | Ouvre en vue scindée. Les URL sont appariées deux par deux (§4.5) | [MVP] (implicite dans Satchel) |
| `--new-window` | Force une nouvelle fenêtre (pas de rattachement) | [v1] |
| `--sudo` / `--admin` | Guide l'utilisateur vers kio-admin : propose de l'installer s'il manque, puis affiche un mode d'emploi (`main.cpp:159-169, 276-278`, `admin/workerintegration.cpp:47-96`) | [Plus tard] |
| `--daemon` | Démarre seulement le service D-Bus `org.freedesktop.FileManager1`, sans fenêtre (`main.cpp:171-191`) | [v1] |

- Si Dolphin est lancé sous `sudo` ou `kdesu`, il **refuse de démarrer** et invite à utiliser `dolphin --sudo` (`main.cpp:55-68`).

### 9.2 Rattachement à une instance existante (`global.cpp:60-139`)
Sans `--new-window`, et seulement si des URL ont été passées :
1. Dolphin cherche les autres instances sur D-Bus (`org.kde.dolphin-<pid>`, interface `org.kde.dolphin.MainWindow`) et commence par **la fenêtre active**.
2. Pour chaque URL : si une instance l'a **déjà ouverte** (`isUrlOpen`), ou si l'élément y est **déjà visible** avec `--select` (`isItemVisibleInAnyView`, qui tient compte des dossiers dépliés en vue Détails, `tabwidget:604-646`), l'URL lui est confiée.
3. Sinon, si `OpenExternallyCalledFolderInNewTab` est actif (`false` par défaut), l'URL est confiée à **la fenêtre active**, qui l'ouvre dans un nouvel onglet. Sinon, l'URL va dans une **nouvelle fenêtre**.
4. L'instance qui reçoit des URL est **activée et mise au premier plan** grâce au jeton d'activation XDG (Wayland) ou au startup-id (X11) (`mainwindow:339-352`).
5. Quand une URL est déjà ouverte dans un onglet ou un panneau, Dolphin **active ce panneau au lieu d'en ouvrir un nouveau** (`tabwidget:220-251`). Avec `--select`, la sélection existante est d'abord effacée (`:268-275`).
- **Conséquence** : par défaut, `dolphin ~/x` **ouvre une nouvelle fenêtre**, sauf si `~/x` est déjà affiché quelque part. Sans argument, Dolphin ouvre toujours une nouvelle fenêtre.

> ⚠ **Pour Satchel** **[MVP]** : une seule instance par session, avec une fenêtre. `satchel <dir>` ouvre ou active un onglet dans le **panneau actif** de la fenêtre existante. Pour le cas « déjà ouvert », on active simplement le panneau qui l'affiche. **[v1]** : option « toujours une nouvelle fenêtre ».

### 9.3 D-Bus
- `org.freedesktop.FileManager1` sur `/org/freedesktop/FileManager1` (`dbusinterface.cpp:17-84`). C'est ce qu'utilisent les navigateurs (« Afficher dans le dossier ») et les autres applications :
  - `ShowFolders(uris, startupId)` : rattachement comme au §9.2, sinon nouvelle fenêtre. **[v1]**
  - `ShowItems(uris, startupId)` : la même chose en mode `--select`. **[v1]** C'est le cas le plus utile.
  - `ShowItemProperties(uris, startupId)` : dialogue Propriétés. **[Plus tard]**
  - `SortOrderForUrl(url) → (role, order)` : spécifique à Dolphin. **[Plus tard]**
  - Le service est enregistré **en file d'attente** (`QueueService`), pour ne pas voler le nom à un autre gestionnaire déjà présent.
- `org.kde.dolphin.MainWindow` (interne, par fenêtre) : `openDirectories(dirs, split)`, `openFiles(files, split)`, `activateWindow(token)`, `isActiveWindow()`, `isUrlOpen(url)`, `isItemVisibleInAnyView(url)`, `pasteIntoFolder()`, `changeUrl(url)` (`dolphinmainwindow.h:155-201`).

**Notes pour Rust/Slint**
- Utiliser `zbus` : un nom propre `org.kde.satchel` pour l'instance unique (si le nom est déjà pris, envoyer `Open(urls, select)` à l'instance existante puis quitter), et `org.freedesktop.FileManager1` réclamé avec `DoNotQueue`/`AllowReplacement` selon le choix de devenir ou non le gestionnaire par défaut.
- Activation Wayland : transmettre `XDG_ACTIVATION_TOKEN` (ou le `startupId` reçu par D-Bus) et appeler `xdg_activation_v1` dans l'instance cible. Sinon KWin refuse de donner le focus.

---

## 10. Démarrage et restauration

### 10.1 Dossier de départ
- Sans argument : le dossier personnel configuré (`HomeUrl`, `$HOME` par défaut) (`main.cpp:209-212`). **[MVP]**
- Réglages de démarrage (page « Dossiers et onglets ») : « Afficher au démarrage : dossiers, onglets et état de la fenêtre de la dernière fois » (**`RememberOpenedTabs` = `true`**) ou un dossier fixe (`HomeUrl`). S'y ajoutent les options `SplitView`, `EditableUrl`, `ShowFullPath` et `FilterBar` (kcfg:25-88, `settings/interface/folderstabssettingspage.cpp:53-96`). **[MVP]**
- Ces réglages ne sont appliqués aux fenêtres déjà ouvertes que s'ils ont été modifiés (`ModifiedStartupSettings`, `dolphinviewcontainer.cpp:530-541`, `dolphinurlnavigatorscontroller.cpp:16-28`).

> ⚠ **Deux panneaux par défaut.** Au premier lancement : le panneau gauche affiche `$HOME` et le panneau droit **aussi**, comme le fait Dolphin avec `--split`. **[MVP]** **[v1]** : réglage « dossier de départ du panneau droit » (par exemple `~/Téléchargements`).

### 10.2 Restauration de session (`RememberOpenedTabs`)
- **Enregistrement automatique** **22 s** après le dernier changement d'URL, de nombre d'onglets ou de panneau actif (minuterie relancée à chaque événement). L'écriture disque se fait dans un thread séparé. À la fermeture, si un enregistrement est en attente, il est fait **immédiatement** (`mainwindow:746-771, 784-811, 699-707`). **[MVP]**
- Contenu enregistré : nombre d'onglets, index de l'onglet actif, puis pour chaque onglet son état complet (§3.5, sauf l'historique) (`tabwidget:80-90`, `tabpage:302-330`). La géométrie et l'état de la fenêtre (barres, docks) sont gérés par KXmlGui (`setupGUI(Save…)`, `mainwindow:207`).
- Au lancement, la restauration n'a lieu **que s'il n'existe aucune autre instance**, ou en restauration de session Plasma. **Les URL passées en argument s'ouvrent ensuite en plus** des onglets restaurés (`main.cpp:247-272`). **[MVP]**
- Le format d'état est versionné (v2-v3 acceptées) et un état inconnu est ignoré sans erreur (`tabpage:341-347`). **[MVP]** Il faut versionner le format de Satchel dès le départ.
- La restauration de session Plasma (déconnexion puis reconnexion) passe par le même mécanisme (`app.isSessionRestored()`). **[v1]**

### 10.3 Divers au démarrage
- Au premier lancement, la barre de menus est masquée et remplacée par un menu hamburger (`mainwindow:213-232`). Hors sujet pour Satchel, qui dessine sa propre barre.
- Après l'affichage de la fenêtre, le focus clavier est sur la vue du panneau actif (`mainwindow:614-621`). **[MVP]**

**Notes pour Rust/Slint**
- État : `~/.local/state/satchel/session.ron` (ou `.toml`), avec `version: u32`, écriture atomique (fichier temporaire puis `rename`), regroupement des écritures (*debounce*) de quelques secondes, et écriture synchrone dans `on_close`. 22 s est un délai long : Satchel peut descendre à environ 3 s sans coût réel.

---

## 11. Mode administrateur **[Plus tard]**

- Il n'est disponible que si le module KIO `admin` (kio-admin) est installé. Sinon l'action n'existe pas (`admin/workerintegration.cpp:42-45, 127-135`).
- **Ctrl+Shift+Alt+A** (case à cocher « Agir en administrateur ») : si l'URL est en `file:`, Dolphin la convertit en `admin:` dans le **panneau actif**. Si elle est en `admin:`, il la reconvertit en `file:`. Pour tout autre schéma, l'action est grisée (`:137-189`).
- La première fois, un **avertissement** s'affiche : « Risks of Acting as an Administrator », avec les boutons « I Understand and Accept These Risks » / Annuler et une case « Do not warn me about these risks again » (`:149-174, 79-90`).
- Tant qu'un panneau est en `admin:`, un **bandeau rouge** (couleurs « négatives » du thème) s'affiche au-dessus de la vue. Il contient « Acting as an Administrator — Be careful! » (version courte : « Acting as Admin »), un bouton d'aide qui reprend l'avertissement et un bouton « Finish » qui active ce panneau puis quitte le mode (`admin/bar.cpp`, `dolphinviewcontainer.cpp:713-722`). Le bandeau disparaît dès qu'on quitte le schéma `admin:`.
- **Expiration de l'autorisation** : le panneau revient en `file:` et affiche « Administrator authorization has expired. » avec un bouton « Act as Administrator Again » (`admin/bar.cpp`, `hideTheNextTimeAuthorizationExpires`).
- **Dossier non lisible** (`ERR_CANNOT_ENTER_DIRECTORY` en `file:` alors que kio-admin est présent) : message « Authorization required to enter this folder. » avec un bouton « Agir en administrateur » (`dolphinviewcontainer.cpp:1021-1038`). **[v1]** même sans le mode admin : afficher au moins le message.
- Pour Satchel, cette fonction est prévue plus tard, via `pkexec` ou un assistant privilégié. Le panneau en mode admin doit être visuellement distinct, par exemple avec la couleur `red` de Catppuccin.

---

## 12. Autres comportements de coquille utiles

- **Recharger** (F5) recharge le panneau actif, masque le message éventuel et rafraîchit l'espace disque (`mainwindow:1148-1152`, `dolphinviewcontainer.cpp:567-572`). **[MVP]**
- **Ajouter aux emplacements** : ajoute le premier élément sélectionné, ou le dossier courant si rien n'est sélectionné, avec son nom et son icône (`mainwindow:468-491`). **[v1]**
- **Double-clic sur le fond vide de la vue** : action configurable (`DoubleClickViewAction`, `edit_select_all` par défaut, `none` possible, ou une commande personnalisée avec `{path}` remplacé par le chemin entre guillemets shell) (`mainwindow:3195-3232`, kcfg:18-24). **[Plus tard]** Pour Satchel, « remonter au dossier parent » serait un choix utile.
- **Signets** : menu Signets (KBookmarks) partagé avec les autres applications KDE, avec « Ajouter tous les onglets comme dossier de signets ». Ouvrir un groupe ouvre un onglet par signet (`dolphinbookmarkhandler.cpp:17-101`). **[Plus tard]** Les Emplacements de la barre latérale suffisent.
- Le **nom du panneau Emplacements** sert de libellé partout : titre d'onglet, historique, titre de fenêtre. Renommer un Emplacement met à jour ces libellés immédiatement (`dolphinviewcontainer.cpp:208-211, 1075-1080`). **[v1]**

---

## 13. Récapitulatif MVP (coquille)

1. Deux panneaux permanents, panneau actif unique avec indication visuelle (liseré + fond inactif atténué), **Tab** et **Ctrl+F3** pour changer de panneau, séparateur redimensionnable avec double-clic pour revenir à 50/50.
2. Historique précédent/suivant **par panneau**, avec restauration de la position et de la sélection ; parent (en sélectionnant le dossier quitté), dossier personnel, Retour arrière, boutons latéraux de la souris.
3. Fil d'Ariane par panneau, F6 / Ctrl+L / Alt+D pour l'édition, complétion des chemins, Échap pour rendre le focus à la vue, messages d'erreur de navigation.
4. Onglets (recommandation : **par panneau**) : Ctrl+T, Ctrl+W, clic du milieu, Ctrl+Shift+T, Alt+1…9, Ctrl+Tab, barre masquée quand il n'y a qu'un onglet.
5. Shift+F5 / Shift+F6 pour copier/déplacer vers l'autre panneau, sans confirmation, annulables, avec les mêmes règles d'activation.
6. Barre de filtre par panneau (Ctrl+I, /) avec Échap, Entrée et les flèches.
7. Résumé par panneau : nombre d'éléments, taille, sélection, espace libre, progression différée de 500 ms.
8. Instance unique + `satchel [--select] <urls>`, restauration de session versionnée.
