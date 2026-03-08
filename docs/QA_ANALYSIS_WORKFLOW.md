# QA Analysis Workflow

Date: 2026-03-08
Projet: C:\Users\jerem\Desktop\WhisperPro

## But

Transformer les resultats manuels de la campagne 20 cycles en un rapport exploitable (taux global, detail par app, incidents).

## Etapes

1. Executer la campagne et remplir le CSV:

```powershell
cd C:\Users\jerem\Desktop\WhisperPro
powershell -ExecutionPolicy Bypass -File .\scripts\qa-campaign-20-cycles.ps1
```

2. Analyser le dernier CSV automatiquement:

```powershell
cd C:\Users\jerem\Desktop\WhisperPro
powershell -ExecutionPolicy Bypass -File .\scripts\analyze-qa-campaign.ps1
```

3. Optionnel: analyser un CSV specifique:

```powershell
cd C:\Users\jerem\Desktop\WhisperPro
powershell -ExecutionPolicy Bypass -File .\scripts\analyze-qa-campaign.ps1 -CsvPath "C:\Users\jerem\Desktop\WhisperPro\artifacts\qa\qa-campaign-20-cycles-YYYYMMDD-HHMMSS.csv"
```

## Sorties

- Rapport Markdown genere dans:
  - `artifacts/qa/qa-campaign-report-YYYYMMDD-HHMMSS.md`

## Regles de decision

- PASS si:
  - injection OK >= 95%
  - widget states OK = 100%
- Sinon: FAIL et corrections ciblees par application.
