export type UiLanguage = "fr" | "en";

export const UI_LANGUAGE_STORAGE_KEY = "whisperpro_ui_language";

export const UI_LANGUAGE_OPTIONS: Array<{ value: UiLanguage; label: string }> = [
  { value: "fr", label: "Fran\u00E7ais" },
  { value: "en", label: "English" }
];

type ModelExperience = {
  bestFor: string;
  pros: string;
  cons: string;
};

type UiText = {
  topbarSubtitle: string;
  uiLanguageLabel: string;
  options: string;
  quit: string;
  startSpeaking: string;
  stopAndTranscribe: string;
  activeModel: string;
  noModelInstalled: string;
  model: string;
  shortcut: string;
  translation: string;
  noTranslation: string;
  yourText: string;
  original: string;
  translated: string;
  copy: string;
  translationPlaceholder: string;
  transcriptionPlaceholder: string;
  lastRecording: string;
  targetLanguage: string;
  history: string;
  clear: string;
  noHistory: string;
  unknown: string;
  save: string;
  reset: string;
  close: string;
  saving: string;
  downloading: string;
  cancel: string;
  unsavedChanges: string;
  sectionGeneral: string;
  sectionShortcutInput: string;
  sectionWidget: string;
  sectionModels: string;
  sectionAdvanced: string;
  language: string;
  customLanguage: string;
  computeMode: string;
  gpuDetected: string;
  gpuNotDetected: string;
  checkRuntime: string;
  repairAcceleration: string;
  keyboardShortcut: string;
  detectKeys: string;
  cancelCapture: string;
  detectHint: string;
  voicePunctuation: string;
  showWidget: string;
  widgetOpacity: string;
  widgetOpacityHint: string;
  library: string;
  installed: string;
  notInstalled: string;
  active: string;
  idealFor: string;
  advantages: string;
  limits: string;
  download: string;
  delete: string;
  activeModelPath: string;
  noModelReferenced: string;
  advancedPaths: string;
  whisperModelPath: string;
  copyImpossible: string;
  historyTextCopied: string;
  modelLabels: Record<string, string>;
  statusTranslationDone: string;
  statusTranscriptionDoneNoTranslation: string;
  statusConfigRequired: string;
  statusCaptureCancelled: string;
  statusShortcutDetectedPrefix: string;
  statusEngineChecked: string;
  statusEngineRepairFailed: string;
  statusRecordingInProgress: string;
  statusRecordingStartFailed: string;
  statusTranscriptionInProgress: string;
  statusTranscriptionDone: string;
  statusNoSpeechDetected: string;
  statusTranscriptionError: string;
  statusTranslationCopied: string;
  statusTextCopied: string;
  statusHistoryEmpty: string;
  statusHistoryEmptyPartial: string;
  statusEntryDeleted: string;
  statusPressShortcut: string;
  confirmResetOptions: string;
  confirmQuitApp: string;
  errorDeleteFilesIncompletePrefix: string;
  errorDeleteFileImpossiblePrefix: string;
  errorQuitAppPrefix: string;
  warningSelectedModelMismatchPrefix: string;
  warningSelectedModelMismatchConnector: string;
  overlayListening: string;
  overlayTranscribing: string;
  overlayError: string;
  settingsSaved: string;
  settingsResetDone: string;
  shortcutEmptyExample: string;
  noneValue: string;
  tipActiveModel: string;
  tipTranslation: string;
  tipRecognitionLanguage: string;
  tipComputeMode: string;
  tipShortcut: string;
  tipVoicePunctuation: string;
  tipWidget: string;
  tipWidgetOpacity: string;
  tipDownloadModel: string;
  tipDeleteModel: string;
  tipModelPath: string;
  tipWhisperCliPath: string;
  ariaHelpActiveModel: string;
  ariaHelpTranslation: string;
  ariaHelpLanguage: string;
  ariaHelpComputeMode: string;
  ariaHelpShortcut: string;
  ariaHelpPunctuation: string;
  ariaHelpWidget: string;
  ariaHelpWidgetOpacity: string;
  ariaHelpModelPath: string;
  ariaHelpWhisperCli: string;
  modelExperience: Record<string, ModelExperience>;
  translationLabels: Record<string, string>;
  computeModeLabels: {
    auto: string;
    cpu: string;
    gpu: string;
  };
};

export const UI_TEXT: Record<UiLanguage, UiText> = {
  fr: {
    topbarSubtitle: "Dict\u00E9e vocale locale, rapide et claire",
    uiLanguageLabel: "Interface",
    options: "Options",
    quit: "Quitter",
    startSpeaking: "Commencer \u00E0 parler",
    stopAndTranscribe: "Arr\u00EAter et transcrire",
    activeModel: "Mod\u00E8le actif",
    noModelInstalled: "Aucun mod\u00E8le install\u00E9",
    model: "Mod\u00E8le",
    shortcut: "Raccourci",
    translation: "Traduction",
    noTranslation: "Pas de traduction",
    yourText: "Votre texte",
    original: "Original",
    translated: "Traduction",
    copy: "Copier",
    translationPlaceholder: "La traduction appara\u00EEtra ici...",
    transcriptionPlaceholder: "La transcription appara\u00EEtra ici...",
    lastRecording: "Dernier enregistrement",
    targetLanguage: "Langue cible",
    history: "Historique",
    clear: "Vider",
    noHistory: "Aucune transcription enregistr\u00E9e pour le moment.",
    unknown: "Inconnu",
    save: "Enregistrer",
    reset: "R\u00E9initialiser",
    close: "Fermer",
    saving: "Sauvegarde...",
    downloading: "T\u00E9l\u00E9chargement en cours...",
    cancel: "Annuler",
    unsavedChanges: "Modifications non enregistr\u00E9es. Clique sur Enregistrer pour appliquer.",
    sectionGeneral: "G\u00E9n\u00E9ral",
    sectionShortcutInput: "Raccourci et saisie",
    sectionWidget: "Widget",
    sectionModels: "Mod\u00E8les",
    sectionAdvanced: "Avanc\u00E9",
    language: "Langue",
    customLanguage: "Personnalis\u00E9",
    computeMode: "Mode de calcul",
    gpuDetected: "GPU d\u00E9tect\u00E9.",
    gpuNotDetected: "GPU non d\u00E9tect\u00E9 (build CPU).",
    checkRuntime: "V\u00E9rification...",
    repairAcceleration: "R\u00E9parer l'acc\u00E9l\u00E9ration",
    keyboardShortcut: "Raccourci clavier",
    detectKeys: "D\u00E9tecter touches",
    cancelCapture: "Annuler capture",
    detectHint: 'Utilise "D\u00E9tecter touches" puis clique "Enregistrer".',
    voicePunctuation: "Activer les commandes vocales de ponctuation",
    showWidget: "Afficher le mini-widget",
    widgetOpacity: "Opacit\u00E9 du mini-widget",
    widgetOpacityHint: "Plus bas = plus discret. Valeur appliqu\u00E9e apr\u00E8s Enregistrer.",
    library: "Biblioth\u00E8que",
    installed: "Install\u00E9",
    notInstalled: "Non install\u00E9",
    active: "Actif",
    idealFor: "Id\u00E9al pour",
    advantages: "Avantages",
    limits: "Limites",
    download: "T\u00E9l\u00E9charger",
    delete: "Supprimer",
    activeModelPath: "Mod\u00E8le actif",
    noModelReferenced: "Aucun mod\u00E8le r\u00E9f\u00E9renc\u00E9.",
    advancedPaths: "Chemins avanc\u00E9s",
    whisperModelPath: "Mod\u00E8le Whisper (.bin)",
    copyImpossible: "Copie impossible",
    historyTextCopied: "Texte de l'historique copi\u00E9",
    modelLabels: {
      tiny: "Tiny (rapide)",
      base: "Base (\u00E9quilibre)",
      small: "Small (plus pr\u00E9cis)",
      medium: "Medium (qualit\u00E9 \u00E9lev\u00E9e)",
      "large-v3": "Large v3 (max qualit\u00E9)"
    },
    statusTranslationDone: "Transcription et traduction termin\u00E9es",
    statusTranscriptionDoneNoTranslation: "Transcription termin\u00E9e (traduction indisponible)",
    statusConfigRequired: "Configuration requise dans Options",
    statusCaptureCancelled: "Capture raccourci annul\u00E9e",
    statusShortcutDetectedPrefix: "Raccourci d\u00E9tect\u00E9",
    statusEngineChecked: "Moteur Whisper v\u00E9rifi\u00E9",
    statusEngineRepairFailed: "R\u00E9paration du moteur impossible",
    statusRecordingInProgress: "Enregistrement en cours...",
    statusRecordingStartFailed: "Impossible de d\u00E9marrer l'enregistrement",
    statusTranscriptionInProgress: "Transcription en cours...",
    statusTranscriptionDone: "Transcription termin\u00E9e",
    statusNoSpeechDetected: "Aucune parole d\u00E9tect\u00E9e",
    statusTranscriptionError: "Erreur pendant la transcription",
    statusTranslationCopied: "Traduction copi\u00E9e",
    statusTextCopied: "Texte copi\u00E9",
    statusHistoryEmpty: "Historique vide",
    statusHistoryEmptyPartial: "Historique vide (suppression fichiers partielle)",
    statusEntryDeleted: "Entr\u00E9e supprim\u00E9e",
    statusPressShortcut: "Appuie sur ta combinaison...",
    confirmResetOptions: "R\u00E9initialiser les options aux valeurs par d\u00E9faut ?",
    confirmQuitApp: "Fermer compl\u00E8tement WhisperPro ?",
    errorDeleteFilesIncompletePrefix: "Suppression des fichiers incompl\u00E8te",
    errorDeleteFileImpossiblePrefix: "Suppression fichier impossible",
    errorQuitAppPrefix: "Impossible de fermer l'application",
    warningSelectedModelMismatchPrefix: "Attention: mod\u00E8le s\u00E9lectionn\u00E9",
    warningSelectedModelMismatchConnector: "mais utilis\u00E9",
    overlayListening: "\u00C9coute",
    overlayTranscribing: "Transcription",
    overlayError: "Erreur",
    settingsSaved: "Options enregistr\u00E9es",
    settingsResetDone: "Options r\u00E9initialis\u00E9es",
    shortcutEmptyExample: "Raccourci vide. Exemple: Ctrl+Shift+Space",
    noneValue: "Aucun",
    tipActiveModel: "Choisis ici le mod\u00E8le utilis\u00E9 pour les prochaines transcriptions.",
    tipTranslation: "Choisis une langue cible pour traduire automatiquement apr\u00E8s la transcription.",
    tipRecognitionLanguage: "Langue principale de reconnaissance vocale.",
    tipComputeMode: "Auto: tente GPU puis CPU. CPU: compatible partout. GPU: plus rapide si disponible.",
    tipShortcut: "Clique \"D\u00E9tecter touches\", fais ta combinaison, puis \"Enregistrer\".",
    tipVoicePunctuation: "Transforme des mots comme \"virgule\", \"point\", \"point d\u2019interrogation\" en ponctuation.",
    tipWidget: "Affiche le widget de statut pendant la dict\u00E9e.",
    tipWidgetOpacity: "R\u00E8gle la visibilit\u00E9 du widget: plus bas = plus discret.",
    tipDownloadModel: "T\u00E9l\u00E9charge ce mod\u00E8le localement.",
    tipDeleteModel: "Supprime ce mod\u00E8le du disque.",
    tipModelPath: "Chemin du fichier mod\u00E8le utilis\u00E9 pour la transcription.",
    tipWhisperCliPath: "Chemin du binaire Whisper utilis\u00E9 par l'application.",
    ariaHelpActiveModel: "Aide mod\u00E8le actif",
    ariaHelpTranslation: "Aide traduction",
    ariaHelpLanguage: "Aide langue",
    ariaHelpComputeMode: "Aide mode de calcul",
    ariaHelpShortcut: "Aide raccourci clavier",
    ariaHelpPunctuation: "Aide ponctuation vocale",
    ariaHelpWidget: "Aide mini-widget",
    ariaHelpWidgetOpacity: "Aide opacit\u00E9 widget",
    ariaHelpModelPath: "Aide chemin mod\u00E8le",
    ariaHelpWhisperCli: "Aide chemin whisper-cli",
    modelExperience: {
      tiny: {
        bestFor: "PC modestes et prise de notes ultra rapide",
        pros: "Tr\u00E8s rapide, faible consommation CPU/GPU.",
        cons: "Pr\u00E9cision plus faible, erreurs plus fr\u00E9quentes sur phrases complexes."
      },
      base: {
        bestFor: "Usage quotidien simple",
        pros: "Bon \u00E9quilibre vitesse / qualit\u00E9.",
        cons: "Moins pr\u00E9cis que small/medium sur accents et contexte long."
      },
      small: {
        bestFor: "Utilisation g\u00E9n\u00E9rale avec bonne qualit\u00E9",
        pros: "Bonne pr\u00E9cision tout en restant assez rapide.",
        cons: "Plus lourd que base, temps de chargement plus long."
      },
      medium: {
        bestFor: "Qualit\u00E9 \u00E9lev\u00E9e pour contenu pro",
        pros: "Meilleure compr\u00E9hension du contexte, moins d'erreurs.",
        cons: "Demande plus de RAM/VRAM et peut \u00EAtre sensiblement plus lent."
      },
      "large-v3": {
        bestFor: "Transcriptions exigeantes et meilleure fid\u00E9lit\u00E9",
        pros: "Qualit\u00E9 maximale sur vocabulaire difficile et audio complexe.",
        cons: "Tr\u00E8s lourd, consomme davantage de ressources, moins adapt\u00E9 aux petites configs."
      }
    },
    translationLabels: {
      none: "Pas de traduction",
      en: "Anglais",
      fr: "Fran\u00E7ais",
      es: "Espagnol",
      de: "Allemand",
      it: "Italien",
      pt: "Portugais",
      nl: "N\u00E9erlandais",
      ru: "Russe",
      uk: "Ukrainien",
      pl: "Polonais",
      tr: "Turc",
      ar: "Arabe",
      hi: "Hindi",
      ja: "Japonais",
      ko: "Cor\u00E9en",
      zh: "Chinois",
      sv: "Su\u00E9dois"
    },
    computeModeLabels: {
      auto: "Auto (GPU puis CPU)",
      cpu: "CPU uniquement",
      gpu: "GPU uniquement"
    }
  },
  en: {
    topbarSubtitle: "Local voice dictation, fast and clear",
    uiLanguageLabel: "Interface",
    options: "Options",
    quit: "Quit",
    startSpeaking: "Start speaking",
    stopAndTranscribe: "Stop and transcribe",
    activeModel: "Active model",
    noModelInstalled: "No installed model",
    model: "Model",
    shortcut: "Shortcut",
    translation: "Translation",
    noTranslation: "No translation",
    yourText: "Your text",
    original: "Original",
    translated: "Translation",
    copy: "Copy",
    translationPlaceholder: "Translation will appear here...",
    transcriptionPlaceholder: "Transcription will appear here...",
    lastRecording: "Last recording",
    targetLanguage: "Target language",
    history: "History",
    clear: "Clear",
    noHistory: "No transcription saved yet.",
    unknown: "Unknown",
    save: "Save",
    reset: "Reset",
    close: "Close",
    saving: "Saving...",
    downloading: "Downloading...",
    cancel: "Cancel",
    unsavedChanges: "Unsaved changes. Click Save to apply them.",
    sectionGeneral: "General",
    sectionShortcutInput: "Shortcut and input",
    sectionWidget: "Widget",
    sectionModels: "Models",
    sectionAdvanced: "Advanced",
    language: "Language",
    customLanguage: "Custom",
    computeMode: "Compute mode",
    gpuDetected: "GPU detected.",
    gpuNotDetected: "GPU not detected (CPU build).",
    checkRuntime: "Checking...",
    repairAcceleration: "Repair acceleration",
    keyboardShortcut: "Keyboard shortcut",
    detectKeys: "Detect keys",
    cancelCapture: "Cancel capture",
    detectHint: 'Use "Detect keys", then click "Save".',
    voicePunctuation: "Enable punctuation voice commands",
    showWidget: "Show mini-widget",
    widgetOpacity: "Mini-widget opacity",
    widgetOpacityHint: "Lower = more discreet. Applied after Save.",
    library: "Library",
    installed: "Installed",
    notInstalled: "Not installed",
    active: "Active",
    idealFor: "Best for",
    advantages: "Advantages",
    limits: "Limits",
    download: "Download",
    delete: "Delete",
    activeModelPath: "Active model",
    noModelReferenced: "No model referenced.",
    advancedPaths: "Advanced paths",
    whisperModelPath: "Whisper model (.bin)",
    copyImpossible: "Copy failed",
    historyTextCopied: "History text copied",
    modelLabels: {
      tiny: "Tiny (fast)",
      base: "Base (balanced)",
      small: "Small (more accurate)",
      medium: "Medium (high quality)",
      "large-v3": "Large v3 (max quality)"
    },
    statusTranslationDone: "Transcription and translation completed",
    statusTranscriptionDoneNoTranslation: "Transcription completed (translation unavailable)",
    statusConfigRequired: "Configuration required in Options",
    statusCaptureCancelled: "Shortcut capture canceled",
    statusShortcutDetectedPrefix: "Shortcut detected",
    statusEngineChecked: "Whisper runtime verified",
    statusEngineRepairFailed: "Runtime repair failed",
    statusRecordingInProgress: "Recording in progress...",
    statusRecordingStartFailed: "Unable to start recording",
    statusTranscriptionInProgress: "Transcription in progress...",
    statusTranscriptionDone: "Transcription completed",
    statusNoSpeechDetected: "No speech detected",
    statusTranscriptionError: "Transcription error",
    statusTranslationCopied: "Translation copied",
    statusTextCopied: "Text copied",
    statusHistoryEmpty: "History is empty",
    statusHistoryEmptyPartial: "History cleared (file cleanup partial)",
    statusEntryDeleted: "Entry deleted",
    statusPressShortcut: "Press your key combination...",
    confirmResetOptions: "Reset options to default values?",
    confirmQuitApp: "Close WhisperPro completely?",
    errorDeleteFilesIncompletePrefix: "Incomplete file cleanup",
    errorDeleteFileImpossiblePrefix: "Unable to delete file",
    errorQuitAppPrefix: "Unable to close application",
    warningSelectedModelMismatchPrefix: "Warning: selected model",
    warningSelectedModelMismatchConnector: "but used",
    overlayListening: "Listening",
    overlayTranscribing: "Transcribing",
    overlayError: "Error",
    settingsSaved: "Options saved",
    settingsResetDone: "Options reset",
    shortcutEmptyExample: "Shortcut is empty. Example: Ctrl+Shift+Space",
    noneValue: "None",
    tipActiveModel: "Choose the model used for upcoming transcriptions.",
    tipTranslation: "Choose a target language to automatically translate after transcription.",
    tipRecognitionLanguage: "Primary speech recognition language.",
    tipComputeMode: "Auto: tries GPU then CPU. CPU: widest compatibility. GPU: faster when available.",
    tipShortcut: "Click \"Detect keys\", press your combo, then click \"Save\".",
    tipVoicePunctuation: "Turns words like \"comma\", \"period\" or \"question mark\" into punctuation.",
    tipWidget: "Shows the status mini-widget during dictation.",
    tipWidgetOpacity: "Adjust widget visibility: lower = more discreet.",
    tipDownloadModel: "Download this model locally.",
    tipDeleteModel: "Delete this model from disk.",
    tipModelPath: "Path to the model file used for transcription.",
    tipWhisperCliPath: "Path to the Whisper binary used by the app.",
    ariaHelpActiveModel: "Active model help",
    ariaHelpTranslation: "Translation help",
    ariaHelpLanguage: "Language help",
    ariaHelpComputeMode: "Compute mode help",
    ariaHelpShortcut: "Shortcut help",
    ariaHelpPunctuation: "Punctuation help",
    ariaHelpWidget: "Widget help",
    ariaHelpWidgetOpacity: "Widget opacity help",
    ariaHelpModelPath: "Model path help",
    ariaHelpWhisperCli: "Whisper-cli path help",
    modelExperience: {
      tiny: {
        bestFor: "Low-end PCs and ultra-fast note taking",
        pros: "Very fast, low CPU/GPU usage.",
        cons: "Lower accuracy, more mistakes on complex sentences."
      },
      base: {
        bestFor: "Simple daily usage",
        pros: "Good speed / quality balance.",
        cons: "Less accurate than small/medium on accents and long context."
      },
      small: {
        bestFor: "General usage with good quality",
        pros: "Good accuracy while staying fairly fast.",
        cons: "Heavier than base, longer load times."
      },
      medium: {
        bestFor: "High-quality transcription for pro usage",
        pros: "Better context understanding, fewer mistakes.",
        cons: "Needs more RAM/VRAM and can be noticeably slower."
      },
      "large-v3": {
        bestFor: "Demanding transcription and best fidelity",
        pros: "Maximum quality on difficult vocabulary and complex audio.",
        cons: "Very heavy, more resource usage, less suitable for small setups."
      }
    },
    translationLabels: {
      none: "No translation",
      en: "English",
      fr: "French",
      es: "Spanish",
      de: "German",
      it: "Italian",
      pt: "Portuguese",
      nl: "Dutch",
      ru: "Russian",
      uk: "Ukrainian",
      pl: "Polish",
      tr: "Turkish",
      ar: "Arabic",
      hi: "Hindi",
      ja: "Japanese",
      ko: "Korean",
      zh: "Chinese",
      sv: "Swedish"
    },
    computeModeLabels: {
      auto: "Auto (GPU then CPU)",
      cpu: "CPU only",
      gpu: "GPU only"
    }
  }
};
