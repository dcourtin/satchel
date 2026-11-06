# Feuille de route Satchel

Légende : ☐ à faire · ◐ en cours · ☑ fait. Chaque jalon se termine par une version que l'utilisateur peut lancer et juger.
Mettre à jour ce fichier à la fin de chaque session : cocher, ajouter les décisions et les problèmes rencontrés.

## Jalon 0 — Préparation
- ☑ Livre de recettes (`docs/spec/00` à `05`), 2026-11-06
- ☑ Toolchain : git, rustup (Rust 1.99.1 stable), base-devel
- ☑ `git init`, `.gitignore` (inclure `docs/design/reference-forklift.png`, `target/`)
- ☑ Workspace Cargo : `satchel-core`, `satchel-app`, Slint 1.18.1, build sans avertissement
- ☐ Police Inter (facultative) : `sudo pacman -S inter-font`

## Jalon 1 — Prototype visuel (objectif : « le rendu me plaît »)
Spec : 05 en entier. Les données sont réelles mais en lecture seule.
- ◐ `ui/theme.slint` : jetons Mocha faits (variante Latte à faire)
- ◐ Fenêtre sans décoration avec coins arrondis (affichage validé sous KWin le 2026-11-06 ; déplacer, redimensionner et les boutons restent à tester à la main), boutons de fenêtre maison, déplacement et redimensionnement → **valider le risque 05 §8.1** sous KWin, sinon se rabattre sur la décoration KDE
- ☑ Barre d'outils unifiée (la recherche filtre le panneau actif ; favoris, copier, nouveau dossier et corbeille sont des emplacements vides)
- ☑ Barre latérale : Appareils (lecture de `/proc/mounts` + statvfs) et Favoris (lecture de `user-places.xbel`)
- ☑ Deux panneaux : en-tête (fil d'Ariane, sous-ligne), liseré du panneau actif, Tab pour changer
- ☑ Vue liste (ListView virtualisée) : nom, taille, date, tri par clic sur les colonnes, tri naturel (01 §4)
- ◐ Vue icônes : grille, vignettes (cache freedesktop, sinon décodage sur 4 threads). Pas encore les icônes MIME du thème. Navigation au clavier en grille à faire
- ☑ Navigation : double-clic ou Entrée pour entrer, Retour arrière ou Alt+↑ pour le parent, historique par panneau (03 §5)
- ☑ Inspecteur : aperçu d'image, nom, type, section General (lecture seule)
- ☐ Test de performance : 10 000 fichiers et plus, défilement fluide sur l'écran 4K à l'échelle 1.7 (05 §8.3)

## Jalon 2 — Cœur des fichiers, testé (objectif : « je peux lui confier mes fichiers »)
Spec : 02 §7–13 et §17. **Tout dans `satchel-core`, avec des tests.**
- ☐ Listing asynchrone et incrémental, surveillance avec `notify` (02 §5)
- ☐ Corbeille freedesktop (crate `trash` + limites de `ktrashrc`), suppression définitive confirmée (02 §8)
- ☐ Copier et déplacer : file d'attente, progression, annulation, copie temporaire puis `rename`, même ou autre périphérique (02 §12)
- ☐ Dialogue de conflit : écraser, ignorer, renommer, appliquer à tous (02 §12), à valider d'abord sur Dolphin
- ☐ Renommer (en ligne, 01 §8 + 02 §9), nouveau dossier, nouveau fichier, dupliquer (02 §10)
- ☐ Annulation des opérations (02 §13)
- ☐ Presse-papiers compatible KDE : `text/uri-list` + `application/x-kde-cutselection` via `wl-clipboard-rs` (02 §7)
- ☐ Ouvrir avec l'application par défaut, menu « Ouvrir avec » (`mimeapps.list`) (02 §6)

## Jalon 3 — Utilisable au quotidien (MVP)
- ☐ Sélection complète : clic, Ctrl, Maj, lasso, clavier, recherche en tapant (01 §1–2)
- ☐ Menus contextuels (02 §14)
- ☐ Glisser-déposer interne, puis avec les autres applis, si Slint le permet (01 §7, 02 §11), **à vérifier**
- ☐ Table des raccourcis (03 §2), F5 et F6 pour copier ou déplacer vers l'autre panneau (03 §4.7)
- ☐ Onglets par panneau (03 §3, adapté)
- ☐ Barre de filtre et recherche simple par nom (03 §6, 04 §4)
- ☐ Fichiers cachés, zoom, propriétés de vue par dossier (02 §2–4)
- ☐ Persistance : largeurs, onglets, session (03 §9 et suivants)
- ☐ Paquet Arch (`PKGBUILD`), fichier `.desktop`, icône

## v1
- ☐ Permissions dans l'inspecteur (grille R/W/X, octal, propriétaire et groupe) (05 §6)
- ☐ Tags : lecture et écriture de `user.xdg.tags`, section Tags, pastilles (04 §5, 05 §5)
- ☐ Périphériques udisks2 : monter, démonter, éjecter (04 §1.5)
- ☐ Dossiers dépliables dans la vue liste (01 §3.5)
- ☐ Écriture de `user-places.xbel` (ajouter ou réordonner des favoris) en préservant tout (04 §1.2)
- ☐ Recherche Baloo (`baloosearch6`) (04 §4)
- ☐ Thème clair (Latte), suivi du jeu de couleurs KDE

## Plus tard
- Connexions distantes (SFTP : `russh`/`openssh-sftp-client`, ou monter via KIOFuse), S3
- Synchronisation et comparaison de dossiers (onglet ⇅ de l'inspecteur)
- Vue en colonnes, terminal intégré, renommage par lot, mode administrateur
- Flou KWin derrière la barre latérale (05 §8.2)

## Notes du prototype (2026-11-06)
- Déjà là, en plus de la feuille de route : sélection clic, Ctrl et Maj, flèches et Maj+flèches, Début/Fin, recherche en tapant (1 s, cycle sur la même lettre), Ctrl+A, Ctrl+H (fichiers cachés), Ctrl+1 et Ctrl+2/3 (modes de vue), F11 (inspecteur), Échap, Ctrl+F (recherche).
- Il manque : défilement automatique vers l'élément courant au clavier, info-bulles, redimensionnement des séparateurs, persistance, police Inter.
- Capture d'écran pour vérifier : `spectacle -b -n -f -o fichier.png`, puis recadrer avec ffmpeg (Spectacle capture les deux écrans).

## Journal des décisions
- 2026-11-06 : stack Rust + Slint. Nom « Satchel ». Dépôt `~/dev/satchel`.
- 2026-11-06 : onglets par panneau, deux panneaux permanents, Tab change de panneau (voir `docs/spec/00-index.md`).
