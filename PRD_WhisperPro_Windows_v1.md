# PRD - WhisperPro Windows (v1)

## 1. Vision Produit

Construire l'application de dictée/transcription Whisper la plus fiable sur Windows, avec:

- achat unique (9,99 EUR),
- fonctionnement local par défaut (offline possible),
- performance stable sur machines modestes,
- UX simple pour un usage quotidien pro.

Promesse utilisateur:
"Vous payez une fois, vos données restent sur votre PC, et la dictée fonctionne vraiment."

## 2. Objectifs et Non-objectifs

### 2.1 Objectifs (v1)

- Dictée temps réel multi-app fiable sur Windows.
- Transcription de fichiers audio/video en local.
- Installation simple + diagnostic automatique micro/GPU.
- Temps de réponse perçu rapide (latence faible en dictée courte).
- Aucun abonnement requis.

### 2.2 Non-objectifs (v1)

- Collaboration cloud multi-utilisateur.
- App mobile.
- Traduction multilingue avancée en temps réel.
- Diarization "réunion pro" avancée (reporté v2+).

## 3. Personas Cibles

- Freelancer/indépendant: veut dicter emails, docs, messages rapidement.
- Dev/tech: veut une solution locale, privée, configurable.
- Créateur de contenu: veut transcrire des interviews/podcasts hors ligne.

## 4. Proposition de Valeur

- Paiement unique: 9,99 EUR.
- Local-first: audio et transcriptions restent sur la machine (par défaut).
- Robuste: fallback automatique CPU si GPU indisponible.
- Transparence: diagnostics lisibles et actions correctives guidées.

## 5. KPIs de Succès (90 jours post-lancement)

- Activation:
  - >= 80% terminent onboarding.
  - >= 70% réussissent la première dictée en < 5 min.
- Qualité:
  - taux d'échec session dictée < 2%.
  - crash rate < 0,5% des sessions.
- Performance:
  - latence P50 premier texte < 1,2 s (modèle recommandé machine).
  - latence P95 < 2,5 s.
- Business:
  - conversion essai -> achat >= 6%.
  - refund rate < 8%.

## 6. Exigences Produit (Fonctionnelles)

### 6.1 Dictée globale Windows

- Raccourci global configurable (par défaut: Ctrl+Shift+Space).
- Modes:
  - Push-to-talk (maintenir).
  - Toggle (appuyer pour démarrer/arrêter).
- Insertion texte dans champ actif (apps natives + web).
- Commandes vocales minimales:
  - "nouvelle ligne", "virgule", "point", "point d'interrogation".

Critères d'acceptation:

- L'utilisateur dicte 20 phrases dans 3 apps différentes sans perte de focus.
- Le texte est inséré au curseur avec taux d'erreur d'insertion < 1/100 actions.

### 6.2 Transcription de fichiers

- Formats supportés: wav, mp3, m4a, mp4, mov.
- Import drag-and-drop.
- Export: txt et srt.
- Barre de progression + estimation temps restant.

Critères d'acceptation:

- Un fichier de 60 min se transcrit sans crash.
- Export TXT/SRT valide en un clic.

### 6.3 Gestion des modèles locaux

- Catalogue local: tiny, base, small (et variantes quantifiées recommandées).
- Téléchargement, suppression, changement de modèle.
- Recommandation auto selon benchmark machine.

Critères d'acceptation:

- Installation modèle guidée réussie en < 3 minutes (connexion standard).
- Bascule de modèle sans redémarrage app.

### 6.4 Diagnostic & fiabilité

- Test micro (permission + niveau).
- Détection GPU (NVIDIA/AMD/Intel) + fallback CPU auto.
- Page "Diagnostic" exportable (copie presse-papiers).
- Gestion d'erreurs claire (cause + action).

Critères d'acceptation:

- 90% des erreurs critiques affichent une action corrective explicite.
- Aucune erreur bloquante silencieuse.

### 6.5 Historique local et confidentialité

- Historique des transcriptions stocké localement.
- Bouton "effacer historique".
- Paramètre "ne jamais sauvegarder l'historique".

Critères d'acceptation:

- Suppression historique effective et vérifiable.
- Aucun upload réseau nécessaire pour la transcription locale.

### 6.6 Paramètres UX essentiels

- Choix langue principale.
- Auto-ponctuation on/off.
- Option "copier automatiquement dans presse-papiers".
- Sons de début/fin dictée.

## 7. Exigences Non Fonctionnelles

- OS: Windows 10/11 64-bit.
- Démarrage app < 3 s sur machine cible moyenne.
- Mémoire:
  - idle < 350 MB.
  - dictée small model < 2,5 GB RAM (selon backend).
- Résilience:
  - reprise propre après perte micro/périphérique audio.
- Sécurité:
  - licence signée côté serveur d'activation.
  - binaire signé (code signing).

## 8. Stack Technique Recommandée

- Shell desktop: Tauri (Rust) + frontend React + TypeScript.
- ASR engine: whisper.cpp (local) via wrapper Rust.
- Audio capture: WASAPI (bas niveau) + VAD.
- Stockage local:
  - SQLite (historique, settings, jobs).
  - fichiers modèles sur disque utilisateur.
- Jobs async:
  - queue interne Rust (transcriptions fichiers).
- Packaging:
  - MSIX + installateur EXE fallback.
- Observabilité:
  - logs locaux structurés.
  - crash reporter local opt-in (envoi manuel).

## 9. Architecture (Haut niveau)

- UI (React/Tauri)
  - Dictation Controller
  - File Transcription Manager
  - Settings/Diagnostics
- Core (Rust)
  - Audio Service (capture, VAD, buffering)
  - Inference Service (whisper.cpp, model routing)
  - Text Post-Processor (ponctuation commandes)
  - Input Injection Service (Windows API)
  - Job Queue Service
  - License Service
- Data
  - SQLite DB
  - Local model store
  - Local logs

## 10. Ecrans et UX Flows

### 10.1 Onboarding (3 étapes)

1. Welcome + promesse locale + achat unique.
2. Test micro + permission.
3. Benchmark rapide + recommandation modèle + téléchargement.

Sortie:

- CTA "Commencer à dicter".
- CTA secondaire "Importer un fichier".

### 10.2 Ecran principal (Dashboard)

- Bouton dictée (état: idle/listening/transcribing/error).
- Affichage raccourci global.
- Choix modèle actif + latence estimée.
- Zone "Dernières transcriptions".
- CTA "Transcrire un fichier".

### 10.3 Ecran Transcription Fichier

- Dropzone import.
- Liste jobs (en cours/terminé/erreur).
- Actions: pause, reprendre, annuler, exporter txt/srt.

### 10.4 Ecran Modèles

- Liste modèles installés/disponibles.
- Taille disque, vitesse estimée, précision relative.
- Télécharger/supprimer/définir par défaut.

### 10.5 Ecran Paramètres

- Général: langue, autostart, sons.
- Dictée: raccourci, auto-ponctuation, copier auto.
- Confidentialité: historique on/off, purge données.

### 10.6 Ecran Diagnostic

- Etat micro.
- Etat GPU/CPU backend.
- Versions composants.
- Bouton "Copier diagnostic".

## 11. Backlog Produit Priorisé

### Epic A - Core dictée

- A1: capture micro WASAPI.
- A2: VAD + segmentation audio.
- A3: inférence whisper.cpp.
- A4: post-processing texte + ponctuation de base.
- A5: injection texte multi-app.

### Epic B - Transcription fichiers

- B1: import multi-format.
- B2: pipeline jobs asynchrones.
- B3: export txt/srt.
- B4: reprise sur erreur.

### Epic C - Modèles & perf

- C1: gestion téléchargement/suppression modèles.
- C2: benchmark matériel initial.
- C3: fallback GPU -> CPU automatique.
- C4: cache warmup modèle.

### Epic D - UX & diagnostics

- D1: onboarding.
- D2: page diagnostic.
- D3: messages d'erreur actionnables.
- D4: télémétrie locale opt-in.

### Epic E - Licensing & release

- E1: achat + activation licence.
- E2: gestion offline grace period.
- E3: packaging/signer binaire.
- E4: auto-update canal stable.

## 12. Plan Sprint par Sprint (2 semaines/sprint)

### Sprint 1 - Fondations

- Setup monorepo (Tauri + React + Rust core).
- Capture micro basique.
- UI minimale dashboard.
- DB SQLite + settings.
- CI build Windows.

Definition of Done:

- Build installable interne.
- Enregistrement audio local fonctionnel.

### Sprint 2 - Dictée de bout en bout (MVP technique)

- Intégration whisper.cpp.
- Dictée push-to-talk.
- Injection texte dans app cible.
- Gestion erreurs audio basiques.

DoD:

- Dictée fonctionnelle dans Notepad + navigateur + Word.

### Sprint 3 - Modèles + onboarding

- Gestion modèles (download/switch/delete).
- Benchmark machine + recommandation.
- Onboarding 3 étapes.
- Paramètres dictée essentiels.

DoD:

- Nouvel utilisateur peut dicter en < 5 minutes.

### Sprint 4 - Transcription fichiers

- Import fichiers audio/video.
- Job queue + progression.
- Export txt/srt.
- Reprise en cas d'erreur.

DoD:

- 60 min audio transcrit sans crash majeur.

### Sprint 5 - Fiabilité Windows

- Diagnostic micro/GPU avancé.
- Fallback automatique CPU.
- Optimisation latence + mémoire.
- Logs structurés + copie diagnostic.

DoD:

- Crash rate interne < 1%.
- Latence P50 cible atteinte sur machines test.

### Sprint 6 - Monetisation & release candidate

- Licence achat unique + activation.
- UI paywall/activation propre.
- Installateur signé.
- Beta fermée + corrections critiques.

DoD:

- Release candidate prête pour 100 beta users.

### Sprint 7 - Lancement public

- Corrections beta prioritaires.
- Stabilisation update.
- Documentation support + FAQ.
- Publication store/site + analytics activation.

DoD:

- Version 1.0 GA publiée.

## 13. Plan QA et Matrice de Tests

### 13.1 Matrice matérielle minimale

- CPU:
  - Intel i5 gen recente,
  - AMD Ryzen 5.
- RAM:
  - 8 GB / 16 GB.
- GPU:
  - iGPU Intel,
  - NVIDIA RTX,
  - AMD Radeon.

### 13.2 Scenarios clés

- Dictée continue 30 min.
- Changement micro à chaud.
- Perte périphérique audio puis reprise.
- Passage GPU indisponible -> CPU.
- Export SRT sur longs fichiers.

### 13.3 Tests automatiques

- Unit tests Rust (pipeline audio, parsing, settings).
- Integration tests injection texte.
- E2E smoke UI (onboarding + dictée courte).

## 14. Checklist de Lancement

### Produit

- Tous les flux clés testés sur Windows 10/11.
- Messages d'erreur clairs et traduits FR/EN.
- Politique confidentialité claire (local-first).

### Technique

- Binaire signé.
- Installateur testé clean machine.
- Auto-update rollback vérifié.
- Monitoring crash post-release prêt.

### Business

- Paiement achat unique opérationnel.
- Emails transactionnels (licence, facture, support) prêts.
- Page pricing/comparatif publiée.

### Support

- FAQ "micro ne marche pas", "GPU non détecté", "texte incorrect".
- Template réponse support standardisé.
- SLA support initial défini (ex: 48h ouvrées).

### Marketing

- Landing page avec promesse claire:
  - achat unique 9,99 EUR,
  - local/offline,
  - démonstration en vidéo.
- Changelog public v1.0.

## 15. Risques et Mitigations

- Risque: variabilité perf selon PC.
  - Mitigation: benchmark auto + recommandation modèle + fallback CPU.
- Risque: qualité perçue selon accent/langue.
  - Mitigation: modèles adaptés + dictionnaire personnalisé + indicateur confiance.
- Risque: support élevé au lancement.
  - Mitigation: diagnostics exportables + FAQ technique orientée résolution.
- Risque: piratage licence.
  - Mitigation: activation légère + vérif périodique non intrusive.

## 16. Post-lancement (v1.1/v1.2)

- Commandes vocales avancées (édition texte).
- Mode réunion (timestamps + speakers).
- Traduction offline.
- API locale pour intégrations tierces.

## 17. Decisions Validees (08/03/2026)

- Moteur par défaut v1:
  - whisper.cpp pur (local-first), backend hybride seulement en option v2+.
- Politique licence:
  - 1 licence = 3 appareils utilisateur (activation légère avec anti-abus raisonnable).
- Distribution:
  - lancement principal via site officiel (checkout direct),
  - publication Microsoft Store planifiée après stabilisation v1.1.

