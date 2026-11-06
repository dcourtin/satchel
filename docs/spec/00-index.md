# Livre de recettes Satchel — index

La spec comportementale a été extraite de Dolphin 26.08.3 (`~/dev/_ref/dolphin-src/dolphin-26.08.3/`) le 2026-11-06. La spec du design est tirée de la référence ForkLift.

| Fichier | Contenu | MVP / v1 / Plus tard |
|---|---|---|
| [01-vues-elements.md](01-vues-elements.md) | Sélection, clavier, recherche en tapant, dispositions et zoom, tri naturel, regroupement, survol, glisser (démarrage), renommage en ligne, vignettes, animations, performances | 40 / 26 / 8 |
| [02-vue-et-operations.md](02-vue-et-operations.md) | Propriétés de vue par dossier, fichiers cachés et filtre, surveillance du dossier, ouverture, presse-papiers, corbeille et suppression, renommage, création, dépôt, **conflits**, annulation, menus contextuels, erreurs, **sécurité des données** | 91 / 43 / 12 |
| [03-application-onglets-actions.md](03-application-onglets-actions.md) | **Table des actions et raccourcis**, onglets, vue double, historique, fil d'Ariane, barre de filtre, barre d'état, ligne de commande et D-Bus, session | 102 / 68 / 28 |
| [04-panneaux-recherche-reglages.md](04-panneaux-recherche-reglages.md) | Emplacements (`user-places.xbel`), périphériques (udisks2), panneau d'infos (machine d'états, délais), arborescence, terminal, recherche, tags (`user.xdg.tags`), **tableau des réglages** | 94 / 94 / 33 |
| [05-design.md](05-design.md) | Anatomie de la fenêtre, jetons Catppuccin, typographie, barre unifiée, barre latérale, panneaux, vue liste et vue icônes, inspecteur, mouvement, **risques techniques** | — |

## Avertissements de fiabilité
- Les références `fichier.cpp:ligne` ont été relevées par des agents de lecture. Elles n'ont pas été revérifiées une à une : relire la source avant d'implémenter un détail fin.
- Les passages marqués **(KIO, inféré)**, **(KF6)** ou « à vérifier » décrivent un comportement de bibliothèques KDE dont la source n'est pas dans l'archive : dialogue de conflit, annulation, menu de dépôt, fil d'Ariane, emplacements, panneau d'infos (baloo-widgets). Il faut les valider sur Dolphin installé (`dolphin` lancé à la main) avant de les coder.
- Les étiquettes MVP des agents sont **larges**. La feuille de route (`../ROADMAP.md`) fait foi pour l'ordre réel.

## Décisions prises qui s'écartent de Dolphin
| Sujet | Dolphin | Satchel | Raison |
|---|---|---|---|
| Vue double | option (F3) | **toujours deux panneaux** (un panneau seul [v1]) | Modèle ForkLift |
| Onglets | un onglet contient la paire de panneaux | **onglets par panneau** | Modèle ForkLift, plus naturel avec deux panneaux permanents |
| Tab | ne change pas de panneau (signal jamais émis en 26.08.3) | **Tab passe à l'autre panneau** | Usage classique des gestionnaires à deux panneaux |
| Barre d'état | pastille flottante, sans espace libre | **sous-ligne de l'en-tête de panneau**, avec l'espace libre | Design 05 §5 |
| Instance | une nouvelle fenêtre par appel | **instance unique** (D-Bus) [v1] | Plus simple |
| Couleurs des tags | aucune | **liste et couleurs propres à Satchel** (les noms restent dans `user.xdg.tags`, comme chez KDE) | Design (pastilles) |
| `GlobalViewProps` | vrai par défaut | **par dossier par défaut**, avec un dossier modèle global | Liste à gauche et images en icônes à droite |
| Bug du popup de recherche (`search/popup.cpp:303`) | les filtres Baloo ne s'affichent jamais | ne pas reproduire | — |
