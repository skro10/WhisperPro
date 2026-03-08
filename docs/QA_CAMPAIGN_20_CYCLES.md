# QA Campaign - 20 Cycles Dictee/Injection

Date: 2026-03-08
Projet: C:\Users\jerem\Desktop\WhisperPro

## Objectif

Valider la fiabilite reelle de la dictee globale et de l'injection texte sur Windows, sur un volume suffisant:

- 20 cycles complets au total
- repartis sur 3 applications cibles
- traces structurées pour analyse

## Applications cibles recommandees

- Notepad
- Chrome (zone texte simple)
- WordPad (ou equivalent editeur riche)

## Repartition recommandee

- Notepad: 8 cycles
- Chrome: 6 cycles
- WordPad: 6 cycles

## Definition d'un cycle

1. Activer zone de saisie cible.
2. Hotkey (ou bouton) demarrage dictee.
3. Dicter une phrase courte (5 a 12 mots).
4. Hotkey (ou bouton) arret dictee.
5. Verifier:
   - etats widget (`listening -> transcribing -> done`)
   - texte injecte (complet, sans troncature)
6. Noter resultat.

## Conditions de PASS campagne

- Taux succes global >= 95% (19/20 minimum)
- Aucun freeze widget
- Aucune injection vide inattendue
- Premier cycle injection correct (pas de collage stale clipboard)
- `Reset session` doit permettre de revenir en `idle` sans redemarrer l'app

## Matrice d'observation

Champs suivis par cycle:

- `cycle`: numero 1..20
- `target_app`: notepad/chrome/wordpad
- `expected_text`: phrase dictee
- `injected_text`: texte recu
- `widget_states_ok`: yes/no
- `injection_ok`: yes/no
- `notes`: details incident

## Script d'assistance

Lancer:

```powershell
cd C:\Users\jerem\Desktop\WhisperPro
powershell -ExecutionPolicy Bypass -File .\scripts\qa-campaign-20-cycles.ps1
```

Ce script:

- verifie outillage et build,
- cree un fichier CSV de campagne dans `artifacts/qa`,
- affiche le protocole de test manuel.

## Analyse post-campagne

1. Ouvrir le CSV genere.
2. Filtrer `injection_ok = no` ou `widget_states_ok = no`.
3. Regrouper par `target_app` pour detecter un pattern app-specifique.
4. Corriger, puis rejouer campagne complete.
