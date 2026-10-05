//! UI strings. The English source is the key. A missing translation stays English.
//! Active language is a thread-local set once per settings paint and HUD frame.
//! Unset (tests, the web preview) stays English.

mod catalog;

use std::cell::Cell;

use crate::config::Language;

thread_local! {
    static ACTIVE: Cell<Language> = const { Cell::new(Language::En) };
}

pub fn set_language(lang: Language) {
    let lang = match lang {
        Language::System => Language::En,
        other => other,
    };
    ACTIVE.with(|slot| slot.set(lang));
}

pub fn language() -> Language {
    ACTIVE.with(|slot| slot.get())
}

/// Translate `key`. The result lives as long as `key` when the catalog misses,
/// and for `'static` when the key is a string literal.
pub fn t<'a>(key: &'a str) -> &'a str {
    let (table, extra) = match language() {
        Language::En | Language::System => return key,
        Language::Fr => (catalog::FR, EXTRA_FR),
        Language::It => (catalog::IT, EXTRA_IT),
        Language::Es => (catalog::ES, EXTRA_ES),
        Language::De => (catalog::DE, EXTRA_DE),
        Language::PtBr => (catalog::PT_BR, EXTRA_PT_BR),
        Language::Nl => (catalog::NL, EXTRA_NL),
    };
    lookup(table, key).unwrap_or_else(|| lookup(extra, key).unwrap_or(key))
}

fn lookup<'a>(table: &'a [(&'a str, &'a str)], key: &str) -> Option<&'a str> {
    table
        .binary_search_by(|row| row.0.cmp(key))
        .ok()
        .map(|index| table[index].1)
}

/// Short on-track words and templates added after the main catalogs. Keys stay sorted.
const EXTRA_FR: &[(&str, &str)] = &[
    ("1 race moto", "1 manche"),
    ("1 ranked race moto", "1 manche ranked"),
    ("1 rider", "1 pilote"),
    ("BLUE FLAG", "BLEU"),
    ("Best {best}   Ideal {ideal}   {gap}   {laps} laps merged", "Meilleur {best}   Idéal {ideal}   {gap}   {laps} tours fusionnés"),
    ("COLOR", "COULEUR"),
    ("Close, Open when Windows starts, and Open when MX Bikes opens hide to the tray. {key} or the tray icon brings settings back. Quit overlay exits.", "Fermer la fenêtre, Ouvrir au démarrage de Windows et Ouvrir quand MX Bikes s'ouvre envoient l'overlay dans la zone de notification. {key} ou l'icône de la zone rouvre les réglages. Quitter l'overlay ferme l'appli."),
    ("EXTRA", "SUPPL"),
    ("FASTEST LAP", "MEIL. TOUR"),
    ("Header  ·  3 slots", "En-tête  ·  3 emplacements"),
    ("ICON", "ICÔNE"),
    ("IDEAL", "IDÉAL"),
    ("Images (PNG, JPG)", "Images (PNG, JPG)"),
    ("LAP RACE", "TOURS"),
    ("LAPS", "TOURS"),
    ("NEW BEST", "NOUV. MEIL"),
    ("ON THIS SERVER", "SERVEUR"),
    ("PNG", "PNG"),
    ("Pit board (board.json)", "Panneau (board.json)"),
    ("RED FLAG", "ROUGE"),
    ("SIT", "ASSIS"),
    ("STAND", "DEBOUT"),
    ("TIME", "TEMPS"),
    ("TIMED", "TEMPS"),
    ("Up to {max} on the overlay. Matched by .exe name, not install folder. Hide a row with the switch; extras can be removed.", "Jusqu'à {max} sur l'overlay. On les reconnaît au nom du .exe, pas au dossier d'installation. Cache une ligne avec l'interrupteur. Tu peux enlever celles en trop."),
    ("WARMUP", "ÉCHAUFF"),
    ("WHITE FLAG", "BLANC"),
    ("YELLOW FLAG", "JAUNE"),
    ("gap {time}", "écart {time}"),
    ("gap —", "écart —"),
    ("{done} of {need} races", "{done} sur {need} manches"),
    ("{key}  settings", "{key}  réglages"),
    ("{n} race motos", "{n} manches"),
    ("{n} ranked race motos", "{n} manches ranked"),
    ("{n} riders", "{n} pilotes"),
];
const EXTRA_IT: &[(&str, &str)] = &[
    ("1 race moto", "1 manche"),
    ("1 ranked race moto", "1 manche ranked"),
    ("1 rider", "1 pilota"),
    ("BLUE FLAG", "BLU"),
    ("Best {best}   Ideal {ideal}   {gap}   {laps} laps merged", "Migliore {best}   Ideale {ideal}   {gap}   {laps} giri uniti"),
    ("COLOR", "COLORE"),
    ("Close, Open when Windows starts, and Open when MX Bikes opens hide to the tray. {key} or the tray icon brings settings back. Quit overlay exits.", "Chiudere la finestra, Apri all'avvio di Windows e Apri quando MX Bikes si apre mandano l'overlay nell'area di notifica. {key} o l'icona dell'area riapre le impostazioni. Esci dall'overlay chiude l'app."),
    ("EXTRA", "EXTRA"),
    ("FASTEST LAP", "GIRO MIGL."),
    ("Header  ·  3 slots", "Testata  ·  3 slot"),
    ("ICON", "ICONA"),
    ("IDEAL", "IDEALE"),
    ("Images (PNG, JPG)", "Immagini (PNG, JPG)"),
    ("LAP RACE", "GIRI"),
    ("LAPS", "GIRI"),
    ("NEW BEST", "NUOVO RECORD"),
    ("ON THIS SERVER", "SERVER"),
    ("PNG", "PNG"),
    ("Pit board (board.json)", "Tabellone (board.json)"),
    ("RED FLAG", "ROSSA"),
    ("SIT", "SEDUTO"),
    ("STAND", "IN PIEDI"),
    ("TIME", "TEMPO"),
    ("TIMED", "TEMPO"),
    ("Up to {max} on the overlay. Matched by .exe name, not install folder. Hide a row with the switch; extras can be removed.", "Fino a {max} sull'overlay. Li riconosce dal nome del .exe, non dalla cartella di installazione. Nascondi una riga con l'interruttore. Puoi togliere quelli in più."),
    ("WARMUP", "RISCALD"),
    ("WHITE FLAG", "BIANCA"),
    ("YELLOW FLAG", "GIALLA"),
    ("gap {time}", "scarto {time}"),
    ("gap —", "scarto —"),
    ("{done} of {need} races", "{done} di {need} manche"),
    ("{key}  settings", "{key}  impostazioni"),
    ("{n} race motos", "{n} manche"),
    ("{n} ranked race motos", "{n} manche ranked"),
    ("{n} riders", "{n} piloti"),
];
const EXTRA_ES: &[(&str, &str)] = &[
    ("1 race moto", "1 manga"),
    ("1 ranked race moto", "1 manga ranked"),
    ("1 rider", "1 piloto"),
    ("BLUE FLAG", "AZUL"),
    ("Best {best}   Ideal {ideal}   {gap}   {laps} laps merged", "Mejor {best}   Ideal {ideal}   {gap}   {laps} vueltas unidas"),
    ("COLOR", "COLOR"),
    ("Close, Open when Windows starts, and Open when MX Bikes opens hide to the tray. {key} or the tray icon brings settings back. Quit overlay exits.", "Cerrar la ventana, Abrir al iniciar Windows y Abrir cuando MX Bikes se abre mandan el overlay a la bandeja. {key} o el icono de la bandeja vuelve a abrir los ajustes. Salir del overlay cierra la app."),
    ("EXTRA", "EXTRA"),
    ("FASTEST LAP", "MEJOR VUELTA"),
    ("Header  ·  3 slots", "Cabecera  ·  3 espacios"),
    ("ICON", "ICONO"),
    ("IDEAL", "IDEAL"),
    ("Images (PNG, JPG)", "Imágenes (PNG, JPG)"),
    ("LAP RACE", "VUELTAS"),
    ("LAPS", "VUELTAS"),
    ("NEW BEST", "NUEVO MEJOR"),
    ("ON THIS SERVER", "SERVIDOR"),
    ("PNG", "PNG"),
    ("Pit board (board.json)", "Pit board (board.json)"),
    ("RED FLAG", "ROJA"),
    ("SIT", "SENTADO"),
    ("STAND", "DE PIE"),
    ("TIME", "TIEMPO"),
    ("TIMED", "TIEMPO"),
    ("Up to {max} on the overlay. Matched by .exe name, not install folder. Hide a row with the switch; extras can be removed.", "Hasta {max} en el overlay. Se reconocen por el nombre del .exe, no por la carpeta de instalación. Oculta una fila con el interruptor. Puedes quitar los que sobran."),
    ("WARMUP", "CALENT"),
    ("WHITE FLAG", "BLANCA"),
    ("YELLOW FLAG", "AMARILLA"),
    ("gap {time}", "hueco {time}"),
    ("gap —", "hueco —"),
    ("{done} of {need} races", "{done} de {need} mangas"),
    ("{key}  settings", "{key}  ajustes"),
    ("{n} race motos", "{n} mangas"),
    ("{n} ranked race motos", "{n} mangas ranked"),
    ("{n} riders", "{n} pilotos"),
];
const EXTRA_DE: &[(&str, &str)] = &[
    ("1 race moto", "1 Lauf"),
    ("1 ranked race moto", "1 Ranked-Lauf"),
    ("1 rider", "1 Fahrer"),
    ("BLUE FLAG", "BLAU"),
    ("Best {best}   Ideal {ideal}   {gap}   {laps} laps merged", "Beste {best}   Ideal {ideal}   {gap}   {laps} Runden vereint"),
    ("COLOR", "FARBE"),
    ("Close, Open when Windows starts, and Open when MX Bikes opens hide to the tray. {key} or the tray icon brings settings back. Quit overlay exits.", "Fenster schließen, Mit Windows starten und Öffnen, wenn MX Bikes startet legen das Overlay in die Taskleiste. {key} oder das Symbol in der Leiste holt die Einstellungen zurück. Overlay beenden schließt die App."),
    ("EXTRA", "EXTRA"),
    ("FASTEST LAP", "SCHN. RUNDE"),
    ("Header  ·  3 slots", "Kopf  ·  3 Felder"),
    ("ICON", "ICON"),
    ("IDEAL", "IDEAL"),
    ("Images (PNG, JPG)", "Bilder (PNG, JPG)"),
    ("LAP RACE", "RUNDEN"),
    ("LAPS", "RUNDEN"),
    ("NEW BEST", "BESTZEIT"),
    ("ON THIS SERVER", "SERVER"),
    ("PNG", "PNG"),
    ("Pit board (board.json)", "Pitboard (board.json)"),
    ("RED FLAG", "ROT"),
    ("SIT", "SITZ"),
    ("STAND", "STEH"),
    ("TIME", "ZEIT"),
    ("TIMED", "ZEIT"),
    ("Up to {max} on the overlay. Matched by .exe name, not install folder. Hide a row with the switch; extras can be removed.", "Bis zu {max} auf dem Overlay. Erkannt am .exe-Namen, nicht am Installationsordner. Blende eine Zeile mit dem Schalter aus. Zusätzliche kannst du entfernen."),
    ("WARMUP", "WARMUP"),
    ("WHITE FLAG", "WEISS"),
    ("YELLOW FLAG", "GELB"),
    ("gap {time}", "Abst. {time}"),
    ("gap —", "Abst. —"),
    ("{done} of {need} races", "{done} von {need} Läufen"),
    ("{key}  settings", "{key}  Einstellungen"),
    ("{n} race motos", "{n} Läufe"),
    ("{n} ranked race motos", "{n} Ranked-Läufe"),
    ("{n} riders", "{n} Fahrer"),
];
const EXTRA_PT_BR: &[(&str, &str)] = &[
    ("1 race moto", "1 bateria"),
    ("1 ranked race moto", "1 bateria ranked"),
    ("1 rider", "1 piloto"),
    ("BLUE FLAG", "AZUL"),
    ("Best {best}   Ideal {ideal}   {gap}   {laps} laps merged", "Melhor {best}   Ideal {ideal}   {gap}   {laps} voltas unidas"),
    ("COLOR", "COR"),
    ("Close, Open when Windows starts, and Open when MX Bikes opens hide to the tray. {key} or the tray icon brings settings back. Quit overlay exits.", "Fechar a janela, Abrir quando o Windows iniciar e Abrir quando o MX Bikes abrir mandam o overlay para a bandeja. {key} ou o ícone da bandeja reabre os ajustes. Sair do overlay fecha o app."),
    ("EXTRA", "EXTRA"),
    ("FASTEST LAP", "MELHOR VOLTA"),
    ("Header  ·  3 slots", "Cabeçalho  ·  3 espaços"),
    ("ICON", "ÍCONE"),
    ("IDEAL", "IDEAL"),
    ("Images (PNG, JPG)", "Imagens (PNG, JPG)"),
    ("LAP RACE", "VOLTAS"),
    ("LAPS", "VOLTAS"),
    ("NEW BEST", "NOVO MELHOR"),
    ("ON THIS SERVER", "SERVIDOR"),
    ("PNG", "PNG"),
    ("Pit board (board.json)", "Pit board (board.json)"),
    ("RED FLAG", "VERMELHA"),
    ("SIT", "SENTADO"),
    ("STAND", "EM PÉ"),
    ("TIME", "TEMPO"),
    ("TIMED", "TEMPO"),
    ("Up to {max} on the overlay. Matched by .exe name, not install folder. Hide a row with the switch; extras can be removed.", "Até {max} no overlay. O reconhecimento usa o nome do .exe, não a pasta de instalação. Esconda uma linha com o interruptor. Dá para remover os extras."),
    ("WARMUP", "AQUEC"),
    ("WHITE FLAG", "BRANCA"),
    ("YELLOW FLAG", "AMARELA"),
    ("gap {time}", "dif. {time}"),
    ("gap —", "dif. —"),
    ("{done} of {need} races", "{done} de {need} baterias"),
    ("{key}  settings", "{key}  ajustes"),
    ("{n} race motos", "{n} baterias"),
    ("{n} ranked race motos", "{n} baterias ranked"),
    ("{n} riders", "{n} pilotos"),
];
const EXTRA_NL: &[(&str, &str)] = &[
    ("1 race moto", "1 manche"),
    ("1 ranked race moto", "1 ranked manche"),
    ("1 rider", "1 rijder"),
    ("BLUE FLAG", "BLAUW"),
    ("Best {best}   Ideal {ideal}   {gap}   {laps} laps merged", "Beste {best}   Ideaal {ideal}   {gap}   {laps} ronden samengevoegd"),
    ("COLOR", "KLEUR"),
    ("Close, Open when Windows starts, and Open when MX Bikes opens hide to the tray. {key} or the tray icon brings settings back. Quit overlay exits.", "Venster sluiten, Openen als Windows start en Openen als MX Bikes start zetten de overlay in het systeemvak. {key} of het pictogram in het vak haalt de instellingen terug. Overlay sluiten stopt de app."),
    ("EXTRA", "EXTRA"),
    ("FASTEST LAP", "SNELSTE RONDE"),
    ("Header  ·  3 slots", "Kop  ·  3 vakken"),
    ("ICON", "ICOON"),
    ("IDEAL", "IDEAAL"),
    ("Images (PNG, JPG)", "Afbeeldingen (PNG, JPG)"),
    ("LAP RACE", "RONDES"),
    ("LAPS", "RONDES"),
    ("NEW BEST", "NIEUW RECORD"),
    ("ON THIS SERVER", "SERVER"),
    ("PNG", "PNG"),
    ("Pit board (board.json)", "Pitboard (board.json)"),
    ("RED FLAG", "ROOD"),
    ("SIT", "ZIT"),
    ("STAND", "STA"),
    ("TIME", "TIJD"),
    ("TIMED", "TIJD"),
    ("Up to {max} on the overlay. Matched by .exe name, not install folder. Hide a row with the switch; extras can be removed.", "Tot {max} op de overlay. Herkend aan de .exe-naam, niet aan de installatiemap. Verberg een rij met de schakelaar. Extra's kun je verwijderen."),
    ("WARMUP", "WARMUP"),
    ("WHITE FLAG", "WIT"),
    ("YELLOW FLAG", "GEEL"),
    ("gap {time}", "afst. {time}"),
    ("gap —", "afst. —"),
    ("{done} of {need} races", "{done} van {need} manches"),
    ("{key}  settings", "{key}  instellingen"),
    ("{n} race motos", "{n} manches"),
    ("{n} ranked race motos", "{n} ranked manches"),
    ("{n} riders", "{n} rijders"),
];

/// `template` uses `{name}` placeholders. The template is translated, then values are substituted.
pub fn t_fmt(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = t(template).to_string();
    for (name, value) in pairs {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            set_language(Language::En);
        }
    }

    #[test]
    fn english_is_the_key_when_unset() {
        let _reset = Reset;
        set_language(Language::En);
        assert_eq!(t("Look"), "Look");
        assert_eq!(t("Not a catalog key"), "Not a catalog key");
    }

    #[test]
    fn french_known_key_and_english_fallback() {
        let _reset = Reset;
        set_language(Language::Fr);
        assert_eq!(t("Look"), "Apparence");
        assert_eq!(t("Not a catalog key"), "Not a catalog key");
        assert_ne!(t("Widgets"), "NOT-WIDGETS");
    }

    #[test]
    fn catalogs_are_sorted_and_unique() {
        for table in [
            catalog::FR,
            catalog::IT,
            catalog::ES,
            catalog::DE,
            catalog::PT_BR,
            catalog::NL,
            EXTRA_FR,
            EXTRA_IT,
            EXTRA_ES,
            EXTRA_DE,
            EXTRA_PT_BR,
            EXTRA_NL,
        ] {
            let mut previous = "";
            for (key, value) in table {
                assert!(
                    key > &previous,
                    "catalog key {key:?} is not strictly after {previous:?}"
                );
                assert!(!value.is_empty(), "empty translation for {key:?}");
                previous = key;
            }
        }
    }

    #[test]
    fn every_language_has_the_same_keys() {
        let keys: Vec<&str> = catalog::FR.iter().map(|row| row.0).collect();
        for table in [
            catalog::IT,
            catalog::ES,
            catalog::DE,
            catalog::PT_BR,
            catalog::NL,
        ] {
            let other: Vec<&str> = table.iter().map(|row| row.0).collect();
            assert_eq!(keys, other);
        }
    }
}
