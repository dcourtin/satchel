# Satchel

Gestionnaire de fichiers pour KDE Plasma (Wayland), en **Rust + Slint**. Il fonctionne en deux panneaux par défaut, avec une barre latérale et un panneau d'infos. Son design s'inspire de ForkLift (macOS), et ses comportements de Dolphin 26.08.3.

L'utilisateur, David, ne code pas en C++ et ne rédige pas les specs. **Claude développe.** La spec de référence est le « livre de recettes » de `docs/spec/`. Elle a été extraite du code de Dolphin, qui se trouve dans `~/dev/_ref/dolphin-src/dolphin-26.08.3/` : c'est une source en lecture seule, qu'on ne compile pas et dont on n'exécute rien.

## Avant de coder une fonctionnalité
1. Lire la section concernée dans `docs/spec/` (index : `docs/spec/00-index.md`), et `docs/spec/05-design.md` pour tout ce qui est visuel.
2. Si la spec ne suffit pas, retourner lire le code de Dolphin à la référence `fichier.cpp:ligne` indiquée dans la spec.
3. Vérifier où en est le projet dans `docs/ROADMAP.md`, et le mettre à jour à la fin de chaque session (cocher, noter les décisions).

## Règles d'or
- **Sécurité des données avant tout.** Toute opération qui écrit sur le disque (copier, déplacer, renommer, supprimer, modifier les permissions) passe par `crates/satchel-core`, a des **tests** sur un dossier temporaire, et n'écrase jamais rien sans décision explicite (conflit → on demande). La suppression va à la **corbeille** par défaut. Une copie se fait d'abord dans un fichier temporaire, puis un `rename` atomique la met en place.
- **L'interface ne bloque jamais.** Les entrées/sorties se font hors du fil de l'interface (pool de threads ou tokio). Les mises à jour remontent à Slint via `slint::invoke_from_event_loop` ou un `Weak` handle.
- **Être compatible avec l'écosystème KDE et freedesktop** plutôt que de réinventer : `user-places.xbel` pour les favoris, la spec Trash, la spec Thumbnail (`~/.cache/thumbnails`), `mimeapps.list` pour « ouvrir avec », le presse-papiers `text/uri-list` + `application/x-kde-cutselection` (sans tiret, vérifié dans `kfileitemclipboard.cpp:50`), et `user.xdg.tags` pour les tags.
- Les couleurs et les tailles passent **uniquement** par le `global Theme` Slint (`ui/theme.slint`). Aucune couleur en dur dans les composants.
- L'interface est en français (fichiers de traduction prévus, `@tr()` dans Slint dès le départ). Le code et les commentaires sont en anglais.

## Pièges Slint (constatés le 2026-11-06, ne pas réintroduire)
1. **Ne jamais recréer un modèle, ni réaffecter une struct qui contient des modèles, pendant un événement souris.** Ça détruit l'élément qui tient la souris : Slint continue de lui envoyer les clics, et l'appli paraît figée. Les modèles (`rows`, `crumbs`, `sidebar`) sont branchés **une seule fois**, puis mis à jour sur place avec `update_model()`. Les conditions `if` ne dépendent que de propriétés scalaires dédiées (`left-mode`…). Les actions déclenchées par un clic passent par `defer()`.
2. **Après `drag_window()` ou `drag_resize_window()`**, KWin prend la souris et le relâchement n'arrive jamais à Slint. Il faut appeler `release_pointer_grab()`, qui envoie un `PointerReleased` synthétique. Sinon chaque clic suivant relance un déplacement de fenêtre.
- Diagnostic : `SATCHEL_TRACE=1 cargo run -p satchel-app` journalise les événements bruts de winit et les rappels de l'appli.
- Pour vérifier visuellement : `spectacle -b -n -f -o /tmp/x.png`, puis recadrer avec ffmpeg (les deux écrans sont capturés).

## Organisation prévue
```
Cargo.toml            # workspace
crates/satchel-core/  # FS : listing, tri naturel, opérations, conflits, corbeille, watch, xattr tags — sans UI, testé
crates/satchel-app/   # binaire : état de l'app, contrôleurs, pont Slint
ui/                   # .slint : theme, sidebar, toolbar, pane, listview, iconview, inspector
docs/spec/            # le livre de recettes
docs/ROADMAP.md
```

## Commandes
- `cargo run -p satchel-app`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Environnement de l'utilisateur
- Arch Linux, Plasma 6.8 Wayland, AMD RX 9060 XT. Deux écrans : un 1080p à l'échelle 1.0 et un 4K à l'échelle 1.7. Il faut tester les deux.
- Thème Catppuccin Mocha, accent mauve. La police Inter n'est pas installée par défaut : on se rabat sur Noto Sans.
- sudo ne fonctionne pas depuis le shell de Claude. Donner les commandes à lancer dans Konsole.
