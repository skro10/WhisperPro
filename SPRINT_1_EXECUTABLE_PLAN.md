# Sprint 1 - Plan Executable (2 semaines)

## Objectif Sprint

Livrer une base installable Windows de WhisperPro avec capture audio locale fonctionnelle, UI minimale, settings persistés, et CI build.

## Scope Verrouillé

- Monorepo initial (desktop + core + ui).
- Capture micro basique (WASAPI) et sauvegarde locale d'un enregistrement test.
- Dashboard minimal avec statut audio.
- Persistance settings via SQLite.
- Pipeline CI Windows (build + tests de base).

## Tickets Sprint 1 (ordre recommandé)

### WP-001 - Initialiser la structure monorepo

- Type: Chore
- Priorité: P0
- Estimation: 1 jour
- Description:
  - Créer structure `apps/desktop`, `crates/core`, `packages/ui`.
  - Configurer workspace Rust + package manager JS.
- Critères d'acceptation:
  - Build local sans erreur.
  - Lancement app desktop "hello" fonctionnel.

### WP-002 - Intégrer Tauri + React + TypeScript

- Type: Feature
- Priorité: P0
- Estimation: 1 jour
- Description:
  - Setup Tauri v2 avec frontend React TS.
  - Créer shell navigation minimale (Dashboard, Settings).
- Critères d'acceptation:
  - `npm run tauri dev` démarre sur Windows.
  - Changement d'écran Dashboard/Settings fonctionnel.

### WP-003 - Service capture micro WASAPI (prototype)

- Type: Feature
- Priorité: P0
- Estimation: 2 jours
- Description:
  - Implémenter capture audio PCM via crate Rust dédiée.
  - Exposer commandes Tauri: `start_capture`, `stop_capture`.
- Critères d'acceptation:
  - Enregistrement de 10 sec possible.
  - Fichier WAV généré localement et lisible.

### WP-004 - UI Dashboard minimale dictée

- Type: Feature
- Priorité: P1
- Estimation: 1 jour
- Description:
  - Ajouter bouton `Démarrer/Arrêter test micro`.
  - Afficher état: idle, recording, error.
- Critères d'acceptation:
  - Le bouton pilote correctement le service Rust.
  - Les erreurs sont visibles dans l'UI.

### WP-005 - Persistance settings SQLite

- Type: Feature
- Priorité: P1
- Estimation: 1 jour
- Description:
  - Créer DB locale + table `settings`.
  - Sauvegarder langue et raccourci global par défaut.
- Critères d'acceptation:
  - Settings conservés après redémarrage.
  - Migration initiale exécutée automatiquement.

### WP-006 - Logging structuré local

- Type: Feature
- Priorité: P1
- Estimation: 0,5 jour
- Description:
  - Logger Rust vers fichier local (niveau info/error).
  - Endpoint Tauri pour récupérer les dernières erreurs.
- Critères d'acceptation:
  - Logs accessibles et datés.
  - Dernière erreur affichable dans Settings.

### WP-007 - CI Windows

- Type: Chore
- Priorité: P0
- Estimation: 1 jour
- Description:
  - Workflow CI (lint + test + build desktop debug).
  - Cache dépendances Rust/Node.
- Critères d'acceptation:
  - Pipeline vert sur PR propre.
  - Artefact build disponible.

### WP-008 - QA smoke sprint 1

- Type: Test
- Priorité: P0
- Estimation: 0,5 jour
- Description:
  - Script test manuel: installation, lancement, capture, relance app.
  - Documenter bugs bloquants.
- Critères d'acceptation:
  - Checklist smoke complétée.
  - Aucun bug P0 ouvert en fin de sprint.

## Definition of Done Sprint 1

- Installable interne Windows disponible.
- Capture audio locale fonctionnelle depuis l'UI.
- Settings persistés dans SQLite.
- CI Windows opérationnelle.
- Documentation setup développeur prête.

## Risques Sprint 1 et mitigations

- Risque: friction setup toolchain Rust/Tauri.
  - Mitigation: script bootstrap unique + versions figées.
- Risque: différences drivers audio Windows.
  - Mitigation: test rapide sur 2 machines différentes min.

## Commandes de bootstrap (première passe)

```powershell
# 1) Initialisation frontend/desktop
npm create tauri-app@latest apps/desktop

# 2) Core Rust
cargo new crates/core --lib

# 3) Workspace Rust (racine)
# Ajouter Cargo.toml workspace avec members = ["crates/core", "apps/desktop/src-tauri"]

# 4) Lancement dev
cd apps/desktop
npm install
npm run tauri dev
```

## Livrables fin Sprint 1

- Repo initial prêt pour Sprint 2 (intégration whisper.cpp).
- Démo interne: bouton enregistrement + fichier WAV + logs.
- Note de passage Sprint 1 -> Sprint 2 avec dettes techniques.
