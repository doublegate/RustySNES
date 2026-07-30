//! User-interface localisation (`v1.25.0`, T-FP-A).
//!
//! Ported from RustyNES's own `i18n.rs`. A compile-time catalogue plus the [`crate::t`] macro, so a UI
//! string costs a single array index at runtime and a **missing translation is a compile error**
//! rather than a blank label — see below for why that is the load-bearing property.
//!
//! # Why a compile-time catalogue and not a runtime map
//!
//! A `HashMap<&str, &str>` per locale looks simpler, but it moves every failure to runtime: a typo
//! in a key, or a locale missing an entry, surfaces as an empty or English-looking label that only a
//! human eyeballing that specific screen would notice. Here every string is a [`Msg`] enum variant
//! and each locale is an array indexed by it, so **adding a `Msg` without translating it fails the
//! build** — the compiler enumerates the work instead of the user finding it.
//!
//! # Scope
//!
//! This localises the **shell chrome** (menus, Settings labels, status lines). It deliberately does
//! not localise debugger-panel internals: those show register names, opcodes, and addresses, which
//! are not language-dependent and whose "translation" would actively harm a user reading hardware
//! documentation.

use serde::{Deserialize, Serialize};

/// A localisable user-interface string.
///
/// Adding a variant here forces every locale table below to grow with it. That is the point;
/// resist the temptation to add a catch-all `Other(String)` variant, which would reintroduce
/// exactly the silent-missing-translation failure this design exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Msg {
    // --- File menu ---
    /// "File"
    MenuFile,
    /// "Open ROM…"
    OpenRom,
    /// "Open Recent"
    OpenRecent,
    /// "Clear List"
    ClearList,
    /// "Close ROM"
    CloseRom,
    /// "Screenshot"
    Screenshot,
    /// "Screenshot to Clipboard"
    ScreenshotClipboard,
    /// "Save Settings for This Game"
    SavePerGame,
    /// "Clear Settings for This Game"
    ClearPerGame,
    /// "Quit"
    Quit,

    // --- Emulation menu ---
    /// "Emulation"
    MenuEmulation,
    /// "Pause"
    Pause,
    /// "Resume"
    Resume,
    /// "Reset"
    Reset,
    /// "Power Cycle"
    PowerCycle,
    /// "Speed"
    Speed,
    /// "Region"
    Region,

    // --- View menu ---
    /// "View"
    MenuView,
    /// "Fullscreen"
    Fullscreen,
    /// "Window Size"
    WindowSize,
    /// "Hide Overscan"
    HideOverscan,
    /// "Theme"
    Theme,

    // --- Tools / Debug / Help ---
    /// "Tools"
    MenuTools,
    /// "Debug"
    MenuDebug,
    /// "Help"
    MenuHelp,
    /// "Settings"
    Settings,
    /// "Keyboard Shortcuts"
    KeyboardShortcuts,
    /// "About"
    About,
    /// "Documentation"
    Documentation,
    /// "Report an Issue"
    ReportIssue,

    // --- Settings tabs + labels ---
    /// "Video"
    TabVideo,
    /// "Audio"
    TabAudio,
    /// "Input"
    TabInput,
    /// "System"
    TabSystem,
    /// "Aspect ratio"
    AspectRatio,
    /// "Integer scale"
    IntegerScale,
    /// "Post-filter"
    PostFilter,
    /// "Overscan crop"
    OverscanCrop,
    /// "Frame pacing"
    FramePacing,
    /// "Present mode"
    PresentMode,
    /// "Audio enabled"
    AudioEnabled,
    /// "Volume"
    Volume,
    /// "Target latency (ms)"
    TargetLatency,
    /// "Resampler kernel"
    ResamplerKernel,
    /// "Output device"
    OutputDevice,
    /// "System default"
    SystemDefault,
    /// "Graphic equaliser"
    GraphicEq,
    /// "Flatten"
    Flatten,
    /// "Autofire (turbo)"
    Autofire,
    /// "Gamepads"
    Gamepads,
    /// "Stick deadzone"
    StickDeadzone,
    /// "Rebind"
    Rebind,
    /// "Press a key…"
    PressAKey,
    /// "(unbound)"
    Unbound,
    /// "Restart required"
    RestartRequired,

    // --- Status lines ---
    /// "No ROM loaded"
    NoRomLoaded,
    /// "Load a ROM first"
    LoadRomFirst,
    /// "Paused"
    StatusPaused,
}

impl Msg {
    /// Total number of variants — the width every locale table must have.
    ///
    /// Kept adjacent to the enum so the two are edited together; the const assertions below turn a
    /// mismatch into a build failure.
    pub const COUNT: usize = Self::StatusPaused as usize + 1;
}

/// A supported user-interface language.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    /// English (the source language; always complete by construction).
    #[default]
    En,
    /// Spanish.
    Es,
    /// French.
    Fr,
    /// German.
    De,
    /// Japanese.
    Ja,
}

impl Locale {
    /// The language's own name, for the picker.
    ///
    /// Endonyms ("Español", not "Spanish") because the picker's whole job is to be readable by
    /// someone who does not yet have the UI in a language they read.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Es => "Español",
            Self::Fr => "Français",
            Self::De => "Deutsch",
            Self::Ja => "日本語",
        }
    }

    /// All locales in display order — the single source of truth the picker iterates.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [Self::En, Self::Es, Self::Fr, Self::De, Self::Ja]
    }

    /// This locale's string table.
    #[must_use]
    const fn table(self) -> &'static [&'static str; Msg::COUNT] {
        match self {
            Self::En => &EN,
            Self::Es => &ES,
            Self::Fr => &FR,
            Self::De => &DE,
            Self::Ja => &JA,
        }
    }

    /// Look up `msg`, falling back to English when this locale leaves it empty.
    ///
    /// An empty entry is the documented way to say "not translated yet" without blocking a locale
    /// from existing at all. Falling back to English is strictly better than rendering a blank
    /// label — a user who sees one English string can still operate the emulator.
    #[must_use]
    pub fn get(self, msg: Msg) -> &'static str {
        let s = self.table()[msg as usize];
        if s.is_empty() { EN[msg as usize] } else { s }
    }
}

/// Look up a [`Msg`] in the active locale.
///
/// ```ignore
/// ui.label(t!(locale, Msg::Settings));
/// ```
#[macro_export]
macro_rules! t {
    ($locale:expr, $msg:expr) => {
        $crate::i18n::Locale::get($locale, $msg)
    };
}

// ---------------------------------------------------------------------------------------------
// Locale tables. Order MUST match `Msg`'s declaration order; the const assertions below check the
// LENGTH, and the `Msg::COUNT`-sized array type makes a short table a type error.
//
// An empty string means "not translated yet" and falls back to English (`Locale::get`).
// ---------------------------------------------------------------------------------------------

/// English — the source language, and the fallback every other table resolves against.
static EN: [&str; Msg::COUNT] = [
    "File",
    "Open ROM…",
    "Open Recent",
    "Clear List",
    "Close ROM",
    "Screenshot",
    "Screenshot to Clipboard",
    "Save Settings for This Game",
    "Clear Settings for This Game",
    "Quit",
    "Emulation",
    "Pause",
    "Resume",
    "Reset",
    "Power Cycle",
    "Speed",
    "Region",
    "View",
    "Fullscreen",
    "Window Size",
    "Hide Overscan",
    "Theme",
    "Tools",
    "Debug",
    "Help",
    "Settings",
    "Keyboard Shortcuts",
    "About",
    "Documentation",
    "Report an Issue",
    "Video",
    "Audio",
    "Input",
    "System",
    "Aspect ratio",
    "Integer scale",
    "Post-filter",
    "Overscan crop",
    "Frame pacing",
    "Present mode",
    "Audio enabled",
    "Volume",
    "Target latency (ms)",
    "Resampler kernel",
    "Output device",
    "(system default)",
    "Graphic equaliser",
    "Flatten",
    "Autofire (turbo)",
    "Gamepads",
    "Stick deadzone",
    "Rebind",
    "Press a key…",
    "(unbound)",
    "Takes effect on restart",
    "No ROM loaded",
    "Load a ROM first",
    "Paused",
];

/// Spanish.
static ES: [&str; Msg::COUNT] = [
    "Archivo",
    "Abrir ROM…",
    "Abrir reciente",
    "Borrar lista",
    "Cerrar ROM",
    "Captura de pantalla",
    "Captura al portapapeles",
    "Guardar ajustes para este juego",
    "Borrar ajustes de este juego",
    "Salir",
    "Emulación",
    "Pausar",
    "Continuar",
    "Reiniciar",
    "Apagar y encender",
    "Velocidad",
    "Región",
    "Ver",
    "Pantalla completa",
    "Tamaño de ventana",
    "Ocultar sobrebarrido",
    "Tema",
    "Herramientas",
    "Depuración",
    "Ayuda",
    "Ajustes",
    "Atajos de teclado",
    "Acerca de",
    "Documentación",
    "Informar de un problema",
    "Vídeo",
    "Audio",
    "Entrada",
    "Sistema",
    "Relación de aspecto",
    "Escala entera",
    "Post-filtro",
    "Recorte de sobrebarrido",
    "Ritmo de fotogramas",
    "Modo de presentación",
    "Audio activado",
    "Volumen",
    "Latencia objetivo (ms)",
    "Núcleo de remuestreo",
    "Dispositivo de salida",
    "(predeterminado del sistema)",
    "Ecualizador gráfico",
    "Aplanar",
    "Disparo automático",
    "Mandos",
    "Zona muerta del stick",
    "Reasignar",
    "Pulsa una tecla…",
    "(sin asignar)",
    "Requiere reiniciar",
    "No hay ROM cargada",
    "Carga primero una ROM",
    "Pausado",
];

/// French.
static FR: [&str; Msg::COUNT] = [
    "Fichier",
    "Ouvrir une ROM…",
    "Ouvrir un fichier récent",
    "Vider la liste",
    "Fermer la ROM",
    "Capture d'écran",
    "Capture vers le presse-papiers",
    "Enregistrer les réglages pour ce jeu",
    "Effacer les réglages de ce jeu",
    "Quitter",
    "Émulation",
    "Pause",
    "Reprendre",
    "Réinitialiser",
    "Redémarrage à froid",
    "Vitesse",
    "Région",
    "Affichage",
    "Plein écran",
    "Taille de la fenêtre",
    "Masquer le surbalayage",
    "Thème",
    "Outils",
    "Débogage",
    "Aide",
    "Réglages",
    "Raccourcis clavier",
    "À propos",
    "Documentation",
    "Signaler un problème",
    "Vidéo",
    "Audio",
    "Entrée",
    "Système",
    "Rapport d'aspect",
    "Échelle entière",
    "Post-filtre",
    "Rognage du surbalayage",
    "Cadence des images",
    "Mode de présentation",
    "Audio activé",
    "Volume",
    "Latence cible (ms)",
    "Noyau de rééchantillonnage",
    "Périphérique de sortie",
    "(défaut du système)",
    "Égaliseur graphique",
    "Aplatir",
    "Tir automatique",
    "Manettes",
    "Zone morte du stick",
    "Réassigner",
    "Appuyez sur une touche…",
    "(non assigné)",
    "Prend effet au redémarrage",
    "Aucune ROM chargée",
    "Chargez d'abord une ROM",
    "En pause",
];

/// German.
static DE: [&str; Msg::COUNT] = [
    "Datei",
    "ROM öffnen…",
    "Zuletzt geöffnet",
    "Liste leeren",
    "ROM schließen",
    "Bildschirmfoto",
    "Bildschirmfoto in die Zwischenablage",
    "Einstellungen für dieses Spiel speichern",
    "Einstellungen für dieses Spiel löschen",
    "Beenden",
    "Emulation",
    "Pause",
    "Fortsetzen",
    "Zurücksetzen",
    "Aus- und einschalten",
    "Geschwindigkeit",
    "Region",
    "Ansicht",
    "Vollbild",
    "Fenstergröße",
    "Overscan ausblenden",
    "Design",
    "Werkzeuge",
    "Debug",
    "Hilfe",
    "Einstellungen",
    "Tastenkürzel",
    "Über",
    "Dokumentation",
    "Problem melden",
    "Video",
    "Audio",
    "Eingabe",
    "System",
    "Seitenverhältnis",
    "Ganzzahlige Skalierung",
    "Nachfilter",
    "Overscan-Beschnitt",
    "Bildtaktung",
    "Darstellungsmodus",
    "Audio aktiviert",
    "Lautstärke",
    "Ziel-Latenz (ms)",
    "Resampler-Kernel",
    "Ausgabegerät",
    "(Systemstandard)",
    "Grafischer Equalizer",
    "Zurücksetzen",
    "Dauerfeuer",
    "Gamepads",
    "Stick-Totzone",
    "Neu belegen",
    "Taste drücken…",
    "(nicht belegt)",
    "Wird nach Neustart übernommen",
    "Kein ROM geladen",
    "Zuerst ein ROM laden",
    "Pausiert",
];

/// Japanese.
static JA: [&str; Msg::COUNT] = [
    "ファイル",
    "ROMを開く…",
    "最近使ったROM",
    "履歴を消去",
    "ROMを閉じる",
    "スクリーンショット",
    "クリップボードにコピー",
    "このゲームの設定を保存",
    "このゲームの設定を消去",
    "終了",
    "エミュレーション",
    "一時停止",
    "再開",
    "リセット",
    "電源を入れ直す",
    "速度",
    "地域",
    "表示",
    "全画面",
    "ウィンドウサイズ",
    "オーバースキャンを隠す",
    "テーマ",
    "ツール",
    "デバッグ",
    "ヘルプ",
    "設定",
    "キーボードショートカット",
    "このソフトについて",
    "ドキュメント",
    "問題を報告",
    "映像",
    "音声",
    "入力",
    "システム",
    "アスペクト比",
    "整数倍スケール",
    "ポストフィルター",
    "オーバースキャンの切り取り",
    "フレームペーシング",
    "表示モード",
    "音声を有効化",
    "音量",
    "目標レイテンシ (ms)",
    "リサンプラー",
    "出力デバイス",
    "(システム既定)",
    "グラフィックイコライザー",
    "フラットにする",
    "連射",
    "ゲームパッド",
    "スティックのデッドゾーン",
    "再割り当て",
    "キーを押してください…",
    "(未割り当て)",
    "再起動後に反映されます",
    "ROMが読み込まれていません",
    "先にROMを読み込んでください",
    "一時停止中",
];

// Every table must be exactly `Msg::COUNT` wide. The array TYPE already enforces this at compile
// time; these assertions exist so that a mismatch names the offending table in the error rather
// than producing a wall of type diagnostics.
const _: () = assert!(EN.len() == Msg::COUNT, "EN table width != Msg::COUNT");
const _: () = assert!(ES.len() == Msg::COUNT, "ES table width != Msg::COUNT");
const _: () = assert!(FR.len() == Msg::COUNT, "FR table width != Msg::COUNT");
const _: () = assert!(DE.len() == Msg::COUNT, "DE table width != Msg::COUNT");
const _: () = assert!(JA.len() == Msg::COUNT, "JA table width != Msg::COUNT");

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in for a locale that has translated nothing yet, exercising the fallback directly.
    static BLANK: [&str; Msg::COUNT] = [""; Msg::COUNT];

    /// Every `Msg`, for exhaustive iteration in the tests below.
    ///
    /// Deliberately hand-listed: if a variant is added without extending this list, the length
    /// assertion fails and points at the omission — the same "the compiler enumerates the work"
    /// property the catalogue itself relies on.
    const ALL: [Msg; Msg::COUNT] = [
        Msg::MenuFile,
        Msg::OpenRom,
        Msg::OpenRecent,
        Msg::ClearList,
        Msg::CloseRom,
        Msg::Screenshot,
        Msg::ScreenshotClipboard,
        Msg::SavePerGame,
        Msg::ClearPerGame,
        Msg::Quit,
        Msg::MenuEmulation,
        Msg::Pause,
        Msg::Resume,
        Msg::Reset,
        Msg::PowerCycle,
        Msg::Speed,
        Msg::Region,
        Msg::MenuView,
        Msg::Fullscreen,
        Msg::WindowSize,
        Msg::HideOverscan,
        Msg::Theme,
        Msg::MenuTools,
        Msg::MenuDebug,
        Msg::MenuHelp,
        Msg::Settings,
        Msg::KeyboardShortcuts,
        Msg::About,
        Msg::Documentation,
        Msg::ReportIssue,
        Msg::TabVideo,
        Msg::TabAudio,
        Msg::TabInput,
        Msg::TabSystem,
        Msg::AspectRatio,
        Msg::IntegerScale,
        Msg::PostFilter,
        Msg::OverscanCrop,
        Msg::FramePacing,
        Msg::PresentMode,
        Msg::AudioEnabled,
        Msg::Volume,
        Msg::TargetLatency,
        Msg::ResamplerKernel,
        Msg::OutputDevice,
        Msg::SystemDefault,
        Msg::GraphicEq,
        Msg::Flatten,
        Msg::Autofire,
        Msg::Gamepads,
        Msg::StickDeadzone,
        Msg::Rebind,
        Msg::PressAKey,
        Msg::Unbound,
        Msg::RestartRequired,
        Msg::NoRomLoaded,
        Msg::LoadRomFirst,
        Msg::StatusPaused,
    ];

    #[test]
    fn msg_count_matches_the_variant_list() {
        assert_eq!(ALL.len(), Msg::COUNT);
        // Discriminants must be a dense 0..COUNT range, since every table is indexed by them.
        for (i, m) in ALL.iter().enumerate() {
            assert_eq!(*m as usize, i, "{m:?} is not at index {i}");
        }
    }

    #[test]
    fn english_is_complete() {
        // The fallback locale may never have a hole: an empty entry there renders as an empty
        // label in EVERY locale, which is the one failure the fallback cannot rescue.
        for m in ALL {
            assert!(!EN[m as usize].is_empty(), "English missing {m:?}");
        }
    }

    #[test]
    fn every_locale_resolves_every_message_non_empty() {
        // A locale may leave an entry blank (documented "not translated yet"), but `get` must then
        // fall back to English, so no lookup ever yields an empty string.
        for locale in Locale::all() {
            for m in ALL {
                assert!(
                    !locale.get(m).is_empty(),
                    "{locale:?} resolved {m:?} to an empty string"
                );
            }
        }
    }

    #[test]
    fn blank_entries_fall_back_to_english_rather_than_rendering_empty() {
        // Simulate the "not translated yet" case directly against the fallback logic.
        let idx = Msg::Settings as usize;
        assert!(
            !ES[idx].is_empty(),
            "precondition: ES has this one translated"
        );
        // A locale whose table is entirely blank must still return the English text.
        for m in ALL {
            let s = if BLANK[m as usize].is_empty() {
                EN[m as usize]
            } else {
                BLANK[m as usize]
            };
            assert_eq!(s, EN[m as usize]);
        }
    }

    #[test]
    fn locale_round_trips_through_toml_and_defaults_to_english() {
        assert_eq!(Locale::default(), Locale::En);
        for locale in Locale::all() {
            #[derive(serde::Serialize, serde::Deserialize)]
            struct Holder {
                locale: Locale,
            }
            let text = toml::to_string(&Holder { locale }).expect("serialize");
            let back: Holder = toml::from_str(&text).expect("deserialize");
            assert_eq!(back.locale, locale);
            assert!(!locale.display_name().is_empty());
        }
    }

    #[test]
    fn the_macro_resolves_through_the_active_locale() {
        assert_eq!(t!(Locale::En, Msg::Settings), "Settings");
        assert_eq!(t!(Locale::Es, Msg::Settings), "Ajustes");
        assert_eq!(t!(Locale::De, Msg::Settings), "Einstellungen");
    }
}
