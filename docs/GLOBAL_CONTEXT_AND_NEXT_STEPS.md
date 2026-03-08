# WhisperPro - Contexte Global et Suite des Travaux

Date: 2026-03-08
Projet: C:\Users\jerem\Desktop\WhisperPro

## 1) Vision Produit

WhisperPro est une application desktop Windows de dictée/transcription locale basée sur Whisper, avec:

- achat unique,
- données gardées localement,
- focus robustesse Windows,
- UX simple et fiable.

Positionnement principal: alternative locale fiable aux apps à abonnement.

## 2) Etat Actuel (Ce qui est déjà fait)

### 2.1 Base projet

- Monorepo en place (`apps/desktop`, `crates/core`, `packages/ui`)
- Desktop app Tauri + React + TypeScript initialisée
- CI Windows en place
- Docs Sprint 1 et QA smoke ajoutées

### 2.2 Fonctionnalités validées

- Capture micro locale fonctionne
  - Démarrer/Arrêter test micro OK
  - WAV bien généré et exploitable
- Settings persistants via SQLite
  - langue
  - raccourci
  - chemin modèle `.bin`
  - chemin `whisper-cli.exe`
- Logging structuré local + diagnostic
  - dernière erreur backend
  - chemin du log
- Transcription locale via `whisper-cli` fonctionne
- Test environnement Whisper fonctionne
  - auto-détection des chemins
  - auto-correction des settings si trouvé

### 2.3 Chemins runtime utilisés

- Modèle par défaut:
  - `%LOCALAPPDATA%\WhisperPro\models\ggml-base.bin`
- CLI par défaut:
  - `%LOCALAPPDATA%\WhisperPro\bin\whisper-cli.exe`
- Logs:
  - `%LOCALAPPDATA%\WhisperPro\logs\whisperpro.log`
- DB:
  - `%LOCALAPPDATA%\WhisperPro\whisperpro.db`

### 2.4 UX onboarding (ajouts récents)

- Mode Usage: onboarding guide en 3 etapes avec progression.
- Etape 1: test micro rapide integre (4s) sans passer obligatoirement par Debug.
- Etape 2: verification environnement Whisper directe.
- Etape 3: action dictee directement dans l'onboarding.
- Settings/Diagnostic: actions rapides pour ouvrir dossier modele, dossier whisper-cli et logs.

### 2.5 QA campagne (ajouts recents)

- Nouveau protocole campagne 20 cycles multi-apps:
  - `docs/QA_CAMPAIGN_20_CYCLES.md`
- Nouveau script d'assistance:
  - `scripts/qa-campaign-20-cycles.ps1`
- Le script genere un CSV dans `artifacts/qa` pour tracer chaque cycle.
- Nouveau script d'analyse automatique:
  - `scripts/analyze-qa-campaign.ps1`
  - genere un rapport Markdown dans `artifacts/qa`.

### 2.6 Support diagnostic (ajouts recents)

- Nouveau bouton debug: `Generer snapshot diagnostic`.
- Produit un fichier texte avec:
  - chemins utiles
  - settings courants
  - etat runtime dictée
  - derniere erreur backend

### 2.7 Garde-fous UX (ajouts recents)

- En mode Usage, la dictee est bloquee si l'environnement Whisper est incomplet.
- Warning explicite avec actions directes:
  - verifier maintenant (controle rapide)
  - ouvrir Settings
- Controle rapide met aussi a jour les chemins detectes dans l'UI (modele/whisper-cli).

### 2.8 Recovery runtime (ajouts recents)

- Nouveau reset soft `Reset session` disponible en mode Usage et Debug.
- Effets:
  - stop capture si active (best-effort)
  - reset flags runtime dictée (`recording`, `busy`)
  - retour etat `idle`
  - nettoyage derniere erreur backend
- Objectif: recuperer une session sans redemarrer l'application.

### 2.9 Recovery guide + lisibilite etats (ajouts recents)

- Nouveau bouton `Reset complet guide` en mode Usage:
  - reset runtime
  - verification environnement Whisper
  - message de resultat unifie
- Affichage etat dictee en badge visuel (Usage + Debug):
  - En attente, Ecoute, Transcription, Termine, Occupe, Erreur

## 3) Décisions Techniques Importantes

1. Backend ASR actuel: `whisper-cli` (stable et fonctionnel).
2. Tentative `whisper-rs` faite mais non retenue pour l’instant à cause d’incompatibilités bindings/toolchain sur cette machine.
3. Priorité: avancer produit avec base fiable (whisper-cli), puis réévaluer `whisper-rs` plus tard si besoin.

## 4) Problèmes Rencontrés et Résolus

### Résolu

- npm bloqué par policy PowerShell -> usage `npm.cmd`
- erreurs BOM UTF-8 dans fichiers -> conversion UTF-8 sans BOM
- binaires Whisper téléchargés bloqués Windows -> `Unblock-File`
- absence modèle/CLI locale -> installation dans `%LOCALAPPDATA%\WhisperPro`

### Encours / Non bloquant

- `clang` pas directement dans PATH terminal courant (mais LLVM/libclang installés)
- `whisper-rs` non stable ici, donc contourné par `whisper-cli`

## 5) Roadmap Immédiate (ordre recommandé)

### Etape A - Point 1 prioritaire: Dictée globale Windows + injection texte

Objectif: utiliser un raccourci global pour capturer la voix et injecter le texte dans l’application active.

Sous-tâches:

1. Raccourci global Windows
- enregistrer un hotkey système (global)
- modes: push-to-talk puis toggle
- source: setting `shortcut`

2. Pipeline dictée live minimale
- démarrage capture à l’activation hotkey
- arrêt capture au relâchement (push-to-talk)
- transcription sur chunk court

3. Injection texte
- insertion au curseur dans fenêtre active
- gestion fallback presse-papiers si injection directe échoue
- test sur Notepad, navigateur, Word

4. Gestion états UX
- idle/listening/transcribing/error
- feedback visuel simple

5. Robustesse
- erreurs micro/permission explicites
- reprise après échec

Critères d’acceptation Etape A:

- dicter 20 phrases dans 3 apps sans crash,
- texte injecté correctement dans >= 99% des cas,
- hotkey configurable persistante.

### Etape B - Expérience utilisateur Sprint 2

1. Onboarding guidé unique
- test micro
- test environnement whisper
- premier essai dictée

2. Messages d’erreur orientés action
- exemple: "modèle manquant" + bouton ouvrir dossier

3. Commandes vocales de base
- "nouvelle ligne", "point", "virgule" (post-traitement simple)

### Etape C - Production readiness

1. File jobs transcription
- queue
- annuler/reprendre
- exports TXT/SRT UI

2. Packaging
- installateur propre
- signature

3. Licensing
- activation achat unique
- 3 appareils

## 6) Backlog Technique Détaillé (prochaines tâches concrètes)

### Bloc 1 - Global hotkey

- [ ] ajouter service Rust hotkey global
- [ ] relier au setting `shortcut`
- [ ] événements Tauri frontend pour affichage état

### Bloc 2 - Dictée live

- [ ] créer flux capture court (chunk)
- [ ] exécuter transcription auto à la fin du chunk
- [ ] concaténation texte simple

### Bloc 3 - Injection texte Windows

- [ ] service d’injection clavier unicode
- [ ] fallback presse-papiers
- [ ] tests de compatibilité apps

### Bloc 4 - QA ciblée

- [ ] script smoke dictée globale
- [ ] matrice de tests (NVIDIA/AMD/iGPU)
- [ ] logs dédiés injection/hotkey

## 7) Risques Proches et Mitigations

1. Injection texte hétérogène selon app
- mitigation: stratégie double (injection directe + clipboard fallback)

2. Hotkey en conflit avec raccourcis système
- mitigation: validation du raccourci + message conflit

3. Latence perçue en dictée
- mitigation: chunking court + modèle adapté + feedback visuel

## 8) Commandes utiles (dev)

```powershell
cd C:\Users\jerem\Desktop\WhisperPro\apps\desktop
npm.cmd run tauri:dev
```

```powershell
cd C:\Users\jerem\Desktop\WhisperPro
cargo.exe check --workspace
```

```powershell
cd C:\Users\jerem\Desktop\WhisperPro\apps\desktop
npm.cmd run build
```

## 9) Définition de prêt pour lancer Etape A

Avant de coder la dictée globale:

- [x] capture micro OK
- [x] transcription locale OK
- [x] settings persistants OK
- [x] diagnostic/logs OK
- [x] environnement whisper validé

Conclusion: prêt à implémenter le Point 1 (dictée globale Windows + injection texte).
