# WhisperPro 1.6.0

## Nouveautés
- Mode push-to-talk maintenu: appui sur le raccourci = démarrage, relâchement = arrêt + transcription.
- Mode texte sécurisé: option pour ne pas conserver l'historique local.
- Sélection du microphone d'entrée dans les options audio, avec sauvegarde dans les paramètres.
- Badge de mise à jour dans l'interface principale (détection de nouvelle version via GitHub releases).
- Ouverture fiable de la page de release GitHub depuis l'application.

## Améliorations UX/UI
- Refonte et allègement du panneau Options (organisation plus claire, actions mieux visibles).
- Harmonisation thème clair/sombre sur l'interface principale, options et widget.
- Ajout/ajustement d'ombres légères et hiérarchie visuelle plus cohérente.
- Toggle push-to-talk mieux intégré visuellement dans l'interface.
- Badge de mise à jour recentré dans la barre haute.
- Footer d'état consolidé (statut dictée, erreurs, téléchargement).
- Vu-mètre micro intégré dans le footer pour un retour discret et continu.

## Correctifs Fonctionnels
- Correctif volume des sons widget (preview + son d'apparition) pour respecter le son sélectionné et le volume configuré.
- Correctif robustesse du widget lors d'activations rapides répétées (comportement plus stable).
- Correctif bouton "Original" après traduction (affichage source/traduction cohérent).
- Correctif persistance de la langue UI au redémarrage.
- Correctif persistance du thème UI au redémarrage.
- Correctif erreur `save_widget_preferences missing required key widgetOpacity`.
- Correctifs liés au bandeau de modifications non appliquées dans Options.
- Correctifs d'encodage FR (accents/textes) dans plusieurs zones UI.
- Correctifs de traduction EN/FR pour plusieurs statuts manquants.

## Transcription & Dictée
- Renforcement de la gestion "aucune voix détectée" (statuts et retours plus clairs).
- Amélioration de la robustesse sur les longues dictées (réduction des pertes de segments).
- Ajustements de post-traitement ponctuation/retours ligne pour des phrases plus naturelles.
- Garde-fou explicite quand aucun modèle n'est installé/actif.

## Technique
- Poursuite de la modularisation frontend (composants + hooks orchestrateurs) pour réduire la complexité de `App.tsx`.
- Découpage backend Rust (state/settings/commands/audio/transcription/overlay/runtime/models) pour une base plus maintenable.
- Validation build et checks consolidés après refactor pour limiter les régressions.
