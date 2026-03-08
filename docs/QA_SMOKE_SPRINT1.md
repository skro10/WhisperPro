# QA Smoke - Sprint 1 (WhisperPro)

## Objectif

Valider que le socle Sprint 1 est stable sur Windows: build, lancement desktop, capture micro locale, settings persistants, et diagnostic minimal.

## Pre-requis

- Windows 10/11 64-bit
- Node.js, npm, Rust/cargo installes
- Micro disponible et autorise dans Windows
- Repo a jour

## Evidence attendues

- Capture d'ecran (ou note) de chaque etape critique
- Chemin du WAV genere
- Chemin du log backend
- Verdict final: PASS / FAIL

## Checklist smoke (ordre recommande)

1. Ouvrir terminal dans `C:\Users\jerem\Desktop\WhisperPro`.
2. Lancer le script guide:
   - `powershell -ExecutionPolicy Bypass -File .\scripts\smoke-check-sprint1.ps1`
3. Verifier preflight outillage:
   - `node`, `npm`, `cargo` detectes.
4. Verifier build frontend:
   - `npm.cmd run build` passe sans erreur.
5. Verifier compile Rust:
   - `cargo.exe check --workspace` passe sans erreur.
6. Lancer l'app desktop:
   - `npm.cmd run tauri:dev` depuis `apps/desktop`.
7. Dans l'UI Dashboard:
   - cliquer `Demarrer test micro`, parler 5-10 sec,
   - cliquer `Arreter test micro`.
8. Verifier WAV:
   - le chemin affiche dans l'UI existe,
   - le fichier est lisible.
9. Verifier Settings persistants:
   - modifier `Langue principale` + `Raccourci global`,
   - cliquer `Sauvegarder les settings`,
   - fermer/reouvrir l'app,
   - valeurs conservees.
10. Verifier diagnostic:
   - `Fichier log` est renseigne,
   - `Derniere erreur backend` affiche une valeur coherent (souvent `Aucune`).

## Criteres PASS / FAIL

PASS:

- Toutes les etapes 3 a 10 sont valides.
- Aucun crash bloquant.
- Settings persistants confirmes apres relance.

FAIL:

- Une etape critique echoue (build, lancement, capture, persistence).
- Crash reproductible.

## Template rapport smoke

- Date:
- Testeur:
- Machine:
- Version Windows:
- Resultat global: PASS / FAIL
- Echecs observes:
- Chemin WAV test:
- Chemin log backend:
- Actions recommandees:
