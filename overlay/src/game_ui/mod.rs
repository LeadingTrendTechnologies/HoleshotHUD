//! MX Bikes menu pack — Floating F8 / glass UI installed into the game `ui/` folder.

mod io;
mod mnu;
mod pack;
mod shell;
mod sprites;

#[allow(unused_imports)] // public API surface for settings / uninstall / callers
pub use pack::{
    apply, needs_restart, needs_retry, remove, remove_pack, retry_if_needed, sync_from_config,
    sync_quiet,
};

pub const MANIFEST: &str = "holeshot-ui.manifest";
pub const BAK_DIR: &str = "ui.holeshot-bak";
pub const RESTART_MENUS: &str = "Fully quit MX Bikes and start it again so the menus reload.";

#[cfg(test)]
mod tests {
    use super::io::*;
    use super::mnu::*;
    use super::pack::*;
    use super::shell::*;
    use super::{BAK_DIR, MANIFEST};
    use crate::config::{ink_on_rgb, DEFAULT_PRIMARY};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;

    fn temp_game() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "mxbo-game-ui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join("ui")).unwrap();
        fs::write(p.join("mxbikes.exe"), b"x").unwrap();
        fs::create_dir_all(p.join("plugins")).unwrap();
        p
    }

    #[test]
    fn apply_writes_manifest_tga_and_main() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(ui.join("keep-me.txt"), b"stay").unwrap();
        fs::write(
            ui.join("english.str"),
            "Testing\r\nSingle Player\r\nSingle_race\r\nMulti Player\r\nExit\r\nExit\r\n",
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("accent=#FF9430"));
        assert!(man.contains("main.mnu"));
        assert!(man.contains("english.str"));
        assert!(ui.join("main.mnu").is_file());
        assert!(ui.join("main2.tga").is_file());
        assert!(ui.join("mainbox.tga").is_file());
        assert!(ui.join("main1.tga").is_file());
        let mainbox = fs::read(ui.join("mainbox.tga")).unwrap();
        // Rounded glass: pure black A=100 (stock pointer_shadow / darkbox recipe).
        let w = u16::from_le_bytes([mainbox[12], mainbox[13]]) as usize;
        let h = u16::from_le_bytes([mainbox[14], mainbox[15]]) as usize;
        let px = &mainbox[18..];
        let cx = w / 2;
        let cy = h / 2;
        let i = (cy * w + cx) * 4;
        assert_eq!([px[i], px[i + 1], px[i + 2], px[i + 3]], [0, 0, 0, 100]);
        assert_eq!(px[3], 0, "mainbox.tga outside round must stay clear");
        let main1 = fs::read(ui.join("main1.tga")).unwrap();
        assert!(
            main1.iter().skip(18).all(|&b| b == 0),
            "main1.tga idle dest must be clear (no hollow hairline frame)"
        );
        let mnu = fs::read_to_string(ui.join("main.mnu")).unwrap();
        assert!(mnu.contains("name id_testing"));
        assert!(mnu.contains("name ID_MULTIPLAYER"));
        assert!(mnu.contains("textid Testing"));
        assert!(mnu.contains("textid Single_race"));
        assert!(mnu.contains("textid bike_selection"));
        assert!(mnu.contains("sprite1 main1.tga"));
        assert!(mnu.contains("sprite mainbox.tga"));
        assert!(mnu.contains("sprite logo_ui.tga"));
        assert!(mnu.contains("name ID_LOGO"));
        assert!(
            mnu.contains("color 0 0 0 0"),
            "main darkbox must stay clear; glass is rounded mainbox.tga"
        );
        assert!(
            mnu.contains("rect 0.022000 0.180000 0.202000"),
            "dest card rect must stay full-size"
        );
        assert!(mnu.contains("align center"));
        let yes = mnu.split("name id_yes").nth(1).unwrap().split("name ID_TEXT").next().unwrap();
        let no = mnu.split("name id_no").nth(1).unwrap().split("name id_yes").next().unwrap();
        assert!(yes.contains("align center"), "Exit YES label must be centered");
        assert!(no.contains("align center"), "Exit NO label must be centered");
        assert!(mnu.contains("48 148 255"));
        let eng = fs::read_to_string(ui.join("english.str")).unwrap();
        assert!(eng.contains("Testing\r\nPractice\r\n"));
        assert!(eng.contains("Single_race\r\nOnline\r\n"));
        assert!(!eng.contains("Single Player"));
        assert!(!eng.contains("Multi Player"));
        let card_x0 = 0.022_f32;
        let pad = 0.010_f32;
        assert!(mnu.contains(&format!("{:.6}", card_x0)));
        assert!(mnu.contains(&format!("{:.6}", card_x0 + pad)));
        assert!(ui.join("keep-me.txt").is_file());
        let tga = fs::read(ui.join("done1.tga")).unwrap();
        assert!(tga.windows(3).any(|w| w == [48, 148, 255]));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn apply_restores_logo_ui_from_bak() {
        let game = temp_game();
        let ui = game.join("ui");
        let bak = game.join(BAK_DIR);
        fs::create_dir_all(&bak).unwrap();
        fs::write(bak.join("english.str"), b"Testing\r\nPractice\r\n").unwrap();
        fs::write(bak.join("main.fnt"), b"fnt").unwrap();
        fs::write(bak.join("logo_ui.tga"), b"fake-logo").unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        assert_eq!(fs::read(ui.join("logo_ui.tga")).unwrap(), b"fake-logo");
        let mnu = fs::read_to_string(ui.join("main.mnu")).unwrap();
        assert!(mnu.contains("sprite logo_ui.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn patch_str_pair_rewrites_value_line() {
        let raw = "Testing\r\nSingle Player\r\nSingle_race\r\nMulti Player\r\n";
        let out = patch_str_pair(raw, "Testing", "Practice");
        let out = patch_str_pair(&out, "Single_race", "Online");
        assert_eq!(
            out,
            "Testing\r\nPractice\r\nSingle_race\r\nOnline\r\n"
        );
    }

    #[test]
    fn remove_restores_backup_and_keeps_unrelated() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(ui.join("keep-me.txt"), b"stay").unwrap();
        fs::write(ui.join("main.mnu"), b"stock").unwrap();
        apply(&game, [59, 130, 246]).unwrap();
        assert!(ui.join(MANIFEST).is_file());
        assert!(game.join(BAK_DIR).join("keep-me.txt").is_file());
        assert!(game.join(BAK_DIR).join("main.mnu").is_file());
        remove(&game).unwrap();
        assert!(!ui.join(MANIFEST).is_file());
        assert_eq!(fs::read_to_string(ui.join("keep-me.txt")).unwrap(), "stay");
        assert_eq!(fs::read_to_string(ui.join("main.mnu")).unwrap(), "stock");
        let _ = fs::remove_dir_all(&game);
    }

    /// Uncompressed 32-bit BGRA TGA (bottom-up), one opaque pixel.
    fn tiny_tga(bgr: [u8; 3]) -> Vec<u8> {
        let mut out = vec![
            0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 32, 8,
        ];
        out.extend_from_slice(&[bgr[0], bgr[1], bgr[2], 255]);
        out
    }

    #[test]
    fn remove_restores_recolored_pointer_and_arrows() {
        let game = temp_game();
        let ui = game.join("ui");
        let bak = game.join(BAK_DIR);
        fs::create_dir_all(&bak).unwrap();
        fs::write(bak.join("english.str"), b"Testing\r\nPractice\r\n").unwrap();
        fs::write(bak.join("main.fnt"), b"fnt").unwrap();
        // Stock pointer is blue chrome; arrows are mid-gray.
        let stock_pointer = tiny_tga([200, 80, 40]); // BGR blue-ish
        let stock_arrow = tiny_tga([60, 60, 60]);
        fs::write(bak.join("pointer.tga"), &stock_pointer).unwrap();
        for name in [
            "arrow_lf1.tga",
            "arrow_lf2.tga",
            "arrow_lf3.tga",
            "arrow_rg1.tga",
            "arrow_rg2.tga",
            "arrow_rg3.tga",
        ] {
            fs::write(bak.join(name), &stock_arrow).unwrap();
        }
        apply(&game, [59, 130, 246]).unwrap();
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("pointer.tga"));
        assert!(man.contains("arrow_lf1.tga"));
        assert!(man.contains("arrow_rg3.tga"));
        let tinted_pointer = fs::read(ui.join("pointer.tga")).unwrap();
        let tinted_arrow = fs::read(ui.join("arrow_lf3.tga")).unwrap();
        assert_ne!(tinted_pointer, stock_pointer, "pointer must be recolored");
        assert_ne!(tinted_arrow, stock_arrow, "accent arrow must be recolored");
        remove(&game).unwrap();
        assert_eq!(
            fs::read(ui.join("pointer.tga")).unwrap(),
            stock_pointer,
            "disable must restore stock pointer"
        );
        assert_eq!(
            fs::read(ui.join("arrow_lf3.tga")).unwrap(),
            stock_arrow,
            "disable must restore stock spin arrow"
        );
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn remove_without_manifest_leaves_stock() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(ui.join("main.mnu"), b"stock").unwrap();
        fs::write(ui.join("keep-me.txt"), b"stay").unwrap();
        remove(&game).unwrap();
        assert_eq!(fs::read_to_string(ui.join("main.mnu")).unwrap(), "stock");
        assert_eq!(fs::read_to_string(ui.join("keep-me.txt")).unwrap(), "stay");
        assert!(!ui.join(MANIFEST).is_file());
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn accent_change_rewrites_without_new_backup() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(ui.join("orig.txt"), b"o").unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        fs::write(game.join(BAK_DIR).join("orig.txt"), b"bak").unwrap();
        apply(&game, [59, 130, 246]).unwrap();
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("accent=#3B82F6"));
        let mnu = fs::read_to_string(ui.join("main.mnu")).unwrap();
        assert!(mnu.contains("246 130 59"));
        assert_eq!(
            fs::read_to_string(game.join(BAK_DIR).join("orig.txt")).unwrap(),
            "bak"
        );
        // Back to default orange: hot colors must follow Look primary.
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("accent=#FF9430"));
        let mnu = fs::read_to_string(ui.join("main.mnu")).unwrap();
        assert!(mnu.contains("48 148 255"));
        assert!(!mnu.contains("246 130 59"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn apply_skips_when_manifest_accent_matches() {
        let game = temp_game();
        let ui = game.join("ui");
        apply(&game, DEFAULT_PRIMARY).unwrap();
        // Complete stock seed so skip considers the pack current.
        fs::write(ui.join("main.fnt"), b"fnt").unwrap();
        let before = fs::metadata(ui.join("main.mnu")).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        apply_pack_inner(&ui, DEFAULT_PRIMARY, Some(&game.join(BAK_DIR)), false).unwrap();
        let after = fs::metadata(ui.join("main.mnu")).unwrap().modified().unwrap();
        assert_eq!(before, after, "matching accent must not rewrite");
        apply_pack_inner(&ui, [59, 130, 246], Some(&game.join(BAK_DIR)), false).unwrap();
        let mnu = fs::read_to_string(ui.join("main.mnu")).unwrap();
        assert!(mnu.contains("246 130 59"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn failed_apply_sets_retry_and_success_clears() {
        NEED_RETRY.store(false, Ordering::Relaxed);
        let game = temp_game();
        // ui as a file → apply cannot create_dir_all / write.
        let _ = fs::remove_dir_all(game.join("ui"));
        fs::write(game.join("ui"), b"blocked").unwrap();
        sync_at(&game, true, DEFAULT_PRIMARY, false);
        assert!(needs_retry(), "failed apply must request retry");

        let _ = fs::remove_file(game.join("ui"));
        fs::create_dir_all(game.join("ui")).unwrap();
        sync_at(&game, true, DEFAULT_PRIMARY, false);
        assert!(!needs_retry(), "successful apply must clear retry");
        let man = fs::read_to_string(game.join("ui").join(MANIFEST)).unwrap();
        assert!(man.contains("accent=#FF9430"));
        let mnu = fs::read_to_string(game.join("ui").join("main.mnu")).unwrap();
        assert!(mnu.contains("48 148 255"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_multijoin_keeps_extra_dialog() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("multijoin.mnu"),
            "dialog\r\n{\r\n\tname idd_multi_lan\r\n\titem_text\r\n\t{\r\n\t\tname ID_OLD\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_serverinfo\r\n\titem_text\r\n\t{\r\n\t\tname ID_KEEP_EXTRA\r\n\t\trect 0.000000 0.000000 1.000000 1.000000\r\n\t}\r\n}\r\n",
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let join = fs::read_to_string(ui.join("multijoin.mnu")).unwrap();
        assert!(join.contains("name idd_multi_lan"));
        assert!(join.contains("name idd_multi_world"));
        assert!(join.contains("name idd_server_browser"));
        assert!(join.contains("48 148 255"));
        assert!(join.contains("backcolor 0 0 0 0"));
        assert!(join.contains("sortbackcolor 0 0 0 0"));
        assert!(!join.contains("backcolor 255 20 20 22"));
        assert!(join.contains("rect 0.012500 0.188889 0.987500 0.855556"));
        assert!(join.contains("sortbarheight 0.055556"));
        assert!(join.contains("sort 1"), "PiBoSo list sort is on/off only");
        assert!(join.contains("columnsize0 0.300000"));
        assert!(join.contains("columnsize4 0.200000"));
        assert!(join.contains("columnsize4 0.210000"));
        assert!(join.contains("columnsize5 0.090000"));
        assert!(join.contains("columnsize9 0.080000"));
        assert!(join.contains("name ID_BITMAP1"));
        assert!(join.contains("rect 0.000000 0.000000 0.001000 0.001000"));
        assert!(join.contains("name ID_BOARD"));
        assert!(join.contains("sprite serverboard.tga"));
        assert!(join.contains(&format!(
            "rect {CARD_X0:.6} {CARD_Y0:.6} {CARD_X1:.6} {CARD_Y1:.6}"
        )));
        assert!(join.contains(&format!("rect 0.030000 {CHROME_Y0:.6} 0.140000 {CHROME_Y1:.6}")));
        assert!(join.contains(&format!("rect 0.160000 {CHROME_Y0:.6} 0.340000 {CHROME_Y1:.6}")));
        let back = join.split("name id_back").nth(1).unwrap().split("name ID_SPECTATE").next().unwrap();
        assert!(
            back.contains(&format!("rect 0.030000 {CHROME_Y0:.6} 0.140000 {CHROME_Y1:.6}")),
            "Back sits left of Local/World"
        );
        assert!(ui.join("serverboard.tga").is_file());
        let board = fs::read(ui.join("serverboard.tga")).unwrap();
        let bw = u16::from_le_bytes([board[12], board[13]]) as usize;
        let bh = u16::from_le_bytes([board[14], board[15]]) as usize;
        let bpx = &board[18..];
        let bi = ((bh / 2) * bw + bw / 2) * 4;
        assert_eq!(
            [bpx[bi], bpx[bi + 1], bpx[bi + 2], bpx[bi + 3]],
            [0, 0, 0, 160],
            "serverboard.tga glass should be darker than main (A=160) for list contrast"
        );
        assert_eq!(bpx[3], 0, "serverboard.tga corners must stay clear");
        assert!(!join.contains("rect 0.000000 0.966667 1.000000 1.000000"));
        assert!(join.contains("sprite segtrack.tga"));
        assert!(join.contains("sprite1 segl1.tga"));
        assert!(join.contains("sprite1 segr1.tga"));
        assert!(ui.join("segtrack.tga").is_file());
        assert!(ui.join("segl2.tga").is_file());
        let host = join.split("name id_host").nth(1).unwrap().split("name id_password").next().unwrap();
        assert!(
            host.contains("button1.tga"),
            "Host above the list uses a button plaque"
        );
        assert!(join.contains(&format!("color {}", argb(255, INK))));
        assert!(join.contains("sprite1 done1.tga"));
        assert!(join.contains("sprite1 button1.tga"));
        assert!(join.contains("sprite1 back1.tga"));
        let spectate = join
            .split("name ID_SPECTATE")
            .nth(1)
            .unwrap()
            .split("name ID_BIKECHOOSE")
            .next()
            .unwrap();
        assert!(
            spectate.contains("button1.tga") && !spectate.contains("done1.tga"),
            "Join is the only skew; Spectate uses dest rows"
        );
        assert!(join.contains("sprite fieldbox.tga"));
        assert!(ui.join("fieldbox.tga").is_file());
        assert!(join.contains("name ID_HIDEEMPTY_BG"));
        assert!(join.contains("name ID_HIDEMISSING_BG"));
        assert!(join.contains("sprite button1.tga"));
        let missing = join
            .split("name ID_FILTERMISSINGCONTENTS")
            .nth(1)
            .unwrap()
            .split("name ID_HIDEEMPTY_BG")
            .next()
            .unwrap();
        assert!(
            missing.contains(&format!("color {}", argb(255, TEXT))),
            "Hide Missing label matches Hide Empty ink"
        );
        assert!(join.contains("rect 0.590000 0.150000 0.722000 0.184000"));
        assert!(join.contains("rect 0.412000 0.152000 0.575000 0.182000"));
        assert!(join.contains("align center"));
        assert!(join.contains("name ID_KEEP_EXTRA"));
        assert!(!join.contains("0.066667"));
        assert!(join.contains("backcolor 0 0 0 0"));
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("multijoin.mnu"));
        assert!(man.contains("serverboard.tga"));
        assert!(man.contains("fieldbox.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_connection_centers_loading_cancel() {
        let game = temp_game();
        let ui = game.join("ui");
        let stock = "dialog\r\n{\r\n\tname connection_dialog\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\tsprite dialog600x200.tga\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname id_message\r\n\t\tcolor 255 0 0 0\r\n\t\tbackcolor 0 0 0 0\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_INFO\r\n\t\tcolor 255 40 40 40\r\n\t\tbackcolor 0 0 0 0\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_cancel\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.437500 0.566667 0.562500 0.600000\r\n\t\t\tsprite1 button1.tga\r\n\t\t}\r\n\t\ttextid cancel\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_password\r\n\titem_button\r\n\t{\r\n\t\tname ID_CANCEL\r\n\t\ttextid cancel\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t}\r\n}\r\n";
        fs::write(ui.join("connection.mnu"), stock).unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let conn = fs::read_to_string(ui.join("connection.mnu")).unwrap();
        let loading = conn
            .split("name connection_dialog")
            .nth(1)
            .unwrap()
            .split("name idd_password")
            .next()
            .unwrap();
        assert!(loading.contains("sprite profiledeleteboard.tga"));
        assert!(!loading.contains("dialog600x200.tga"));
        assert!(!loading.contains("\tcolor 255 0 0 0\n"));
        assert!(!loading.contains("\tcolor 255 40 40 40\n"));
        assert!(loading.contains(&format!("\tcolor 255 {}\n", bgr(TEXT))));
        assert!(loading.contains("align center"));
        let cancel = loading.split("name id_cancel").nth(1).unwrap();
        assert!(cancel.contains(&format!("color1 {}", argb(255, TEXT))));
        let password = conn.split("name idd_password").nth(1).unwrap();
        assert!(
            password.contains("align right"),
            "password Cancel stays stock-aligned"
        );
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("connection.mnu"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_options_rewrites_chrome_and_keeps_panes() {
        let game = temp_game();
        let ui = game.join("ui");
        let stock = "dialog\r\n{\r\n\tname idd_options\r\n\titem_tab\r\n\t{\r\n\t\tname id_input_tab\r\n\t\ttextid input_tab\r\n\t\tcolor1 255 208 208 208\r\n\t\tcolor2 255 0 0 0\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_done\r\n\t\ttextid Done\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\tsprite logo_ui.tga\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_graphic_options\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.006250 0.133333 0.993750 0.911111\r\n\t\tsprite dialog1580x700t.tga\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname id_resolutiontext\r\n\t\ttextid resolution\r\n\t\tcolor 255 163 163 163\r\n\t\tbackcolor 0 0 0 0\r\n\t}\r\n\titem_pull\r\n\t{\r\n\t\tname id_fullscreen\r\n\t\ttextcolor 255 163 163 163\r\n\t\tbackcolor 255 37 37 37\r\n\t\tpullbackcolor 200 0 0 0\r\n\t\tpulltextcolor1 255 163 163 163\r\n\t\tpulltextcolor2 255 208 208 208\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname id_screentext\r\n\t\ttextid screen\r\n\t\tcolor 255 0 0 0\r\n\t\tbackcolor 255 0 0 0\r\n\t}\r\n\titem_checkbox\r\n\t{\r\n\t\tname id_vsync\r\n\t\ttextid vsync\r\n\t\tcolor 255 60 60 60\r\n\t\ttextcolor 255 60 60 60\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_wait_control\r\n\titem_text\r\n\t{\r\n\t\tname ID_KEEP_EXTRA\r\n\t\trect 0.000000 0.000000 1.000000 1.000000\r\n\t}\r\n}\r\n";
        fs::write(ui.join("options.mnu"), stock).unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let opt = fs::read_to_string(ui.join("options.mnu")).unwrap();
        assert!(opt.contains("name idd_options"));
        assert!(opt.contains("name id_input_tab"));
        assert!(opt.contains("name ID_INPUT2_TAB"));
        assert!(opt.contains("name ID_INPUT3_TAB"));
        assert!(opt.contains("name id_gfx_tab"));
        assert!(opt.contains("name id_misc_tab"));
        assert!(opt.contains("name id_others_tab"));
        assert!(opt.contains("textid simulation"));
        assert!(opt.contains("sprite1 segl1.tga"));
        assert!(opt.contains("sprite1 segm1.tga"));
        assert!(opt.contains("sprite1 segr1.tga"));
        assert!(opt.contains("sprite optionstabtrack.tga"));
        assert!(opt.contains("align center"));
        assert!(opt.contains(&format!(
            "rect {OPT_TAB_GROUP_X0:.6} {OPT_TAB_Y0:.6}"
        )));
        assert!(opt.contains(&format!(
            "rect 0.030000 {OPT_CHROME_Y0:.6} 0.140000 {OPT_CHROME_Y1:.6}"
        )));
        assert!(opt.contains("sprite1 done1.tga"));
        assert!(opt.contains("sprite1 back1.tga"));
        assert!(opt.contains("48 148 255"));
        assert!(!opt.contains("40 60 240"), "stock PiBoSo blue must become Look accent");
        assert!(opt.contains("name idd_graphic_options"));
        assert!(opt.contains("sprite optionsboard.tga"));
        assert!(!opt.contains("dialog1580x700t.tga"));
        assert!(!opt.contains("optionsrail.tga"));
        assert!(opt.contains(&format!(
            "rect {OPT_FORM_X0:.6} {OPT_FORM_Y0:.6} {OPT_FORM_X1:.6} {OPT_FORM_Y1:.6}"
        )));
        assert!(!opt.contains("rect 0.006250 0.133333 0.993750 0.911111"));
        assert!(opt.contains(&format!("color {}", argb(255, TEXT))));
        assert!(opt.contains(&format!("backcolor {}", argb(255, ROW))));
        assert!(opt.contains(&format!("pulltextcolor2 {}", argb(255, DEFAULT_PRIMARY))));
        assert!(!opt.contains(&format!("backcolor {}", argb(255, TEXT))));
        let pane = opt
            .split("name idd_graphic_options")
            .nth(1)
            .unwrap()
            .split("name idd_wait_control")
            .next()
            .unwrap();
        assert!(
            !pane.contains("\tcolor 255 0 0 0\n"),
            "black form labels must become TEXT on glass"
        );
        assert!(
            !pane.contains("\tcolor 255 60 60 60\n"),
            "dark grey checkbox labels must become TEXT"
        );
        assert!(!pane.contains("textcolor 255 60 60 60"));
        assert!(!pane.contains("backcolor 255 0 0 0"));
        assert!(pane.contains(&format!("\tcolor 255 {}\n", bgr(TEXT))));
        let tab_pos = (OPT_TAB_Y1 - OPT_TAB_Y0 - 0.018) * 0.5;
        let chrome_pos = (OPT_CHROME_Y1 - OPT_CHROME_Y0 - 0.022222) * 0.5;
        assert!(opt.contains(&format!("pos 0.000000 {tab_pos:.6}")));
        assert!(opt.contains(&format!("pos 0.000000 {chrome_pos:.6}")));
        // Remapped controls stay above the tab strip; only the board may span it.
        for line in pane.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("rect ") {
                let nums: Vec<f32> = rest
                    .split_whitespace()
                    .filter_map(|p| p.parse().ok())
                    .collect();
                if nums.len() == 4 && (nums[2] - nums[0]) > 0.002 {
                    let is_board = (nums[0] - OPT_FORM_X0).abs() < 0.001
                        && (nums[1] - OPT_FORM_Y0).abs() < 0.001
                        && (nums[2] - OPT_FORM_X1).abs() < 0.001;
                    assert!(
                        is_board || nums[3] <= OPT_CONTENT_Y1 + 0.001,
                        "control bleeds onto tabs: {t}"
                    );
                }
            }
        }
        assert!(opt.contains("name ID_KEEP_EXTRA"));
        assert!(opt.contains("name idd_wait_control"));
        let chrome = opt.split("name idd_graphic_options").next().unwrap();
        assert!(!chrome.contains("name ID_RAIL"));
        assert!(chrome.contains("name ID_TABGROUP"));
        assert!(!chrome.contains("0.066667"));
        assert!(ui.join("optionsboard.tga").is_file());
        assert!(ui.join("optionstabtrack.tga").is_file());
        assert!(ui.join("segm2.tga").is_file());
        assert!(ui.join("check2.tga").is_file());
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("options.mnu"));
        assert!(man.contains("optionsboard.tga"));
        assert!(man.contains("optionstabtrack.tga"));
        assert!(man.contains("segm1.tga"));
        assert!(!man.contains("optionsrail.tga"));
        assert!(man.contains("check1.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_testsetup_rewrites_chrome_and_recolors_forms() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("testsetup.mnu"),
            "dialog\r\n{\r\n\tname idd_testing_setup\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.006250 0.133333 0.993750 0.188889\r\n\t\tcolor 240 170 180 190\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_start\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n\t\ttextid Start\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_track_info\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.000000 0.000000 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.206250 0.111111 0.793750 0.888889\r\n\t\tsprite dialog940x700.tga\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname id_trackmap\r\n\t\trect 0.212500 0.155556 0.587500 0.822222\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\ttextid author\r\n\t\tcolor 255 0 0 0\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TRACK_AUTHOR\r\n\t\tcolor 255 40 40 40\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname ID_CLOSE\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.437500 0.844444 0.562500 0.877778\r\n\t\t\tsprite1 button1.tga\r\n\t\t}\r\n\t\ttextid close\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_testing_set\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.406250 0.200000 0.993750 0.944444\r\n\t\tsprite dialog940x670.tga\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname id_temperaturetext\r\n\t\ttextid temperature\r\n\t\tcolor 255 0 0 0\r\n\t\tbackcolor 0 0 0 0\r\n\t}\r\n\titem_checkbox\r\n\t{\r\n\t\tname ID_CRASHMODE\r\n\t\ttextid overjump_crash\r\n\t\tcolor 255 60 60 60\r\n\t}\r\n\titem_pull\r\n\t{\r\n\t\tname id_dummy\r\n\t\tbackcolor 255 220 220 220\r\n\t\tpullbackcolor 200 0 0 0\r\n\t\tpulltextcolor2 255 40 60 240\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_track_image\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.006250 0.200000 0.393750 0.944444\r\n\t\tsprite dialog620x670.tga\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname ID_TRACK_INFO\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.012500 0.900000 0.137500 0.933333\r\n\t\t\tsprite1 button1.tga\r\n\t\t}\r\n\t\ttextid track_info\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\t			color3 255 240 240 240\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_bikechoose\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX1\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_back\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.000000 0.966667 0.100000 1.000000\r\n\t\t\tsprite2 back1.tga\r\n\t\t\tsprite3 back2.tga\r\n\t\t}\r\n\t\ttextid Back\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_start\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n\t\ttextid Start\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname ID_INFO\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.562500 0.966667 0.687500 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n\t\ttextid Info\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP2\r\n\t\trect 0.006250 0.133333 0.381250 0.744444\r\n\t\tsprite dialog600x460.tga\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP1\r\n\t\trect 0.006250 0.755556 0.206250 0.833333\r\n\t\tsprite dialog320x70.tga\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT4\r\n\t\ttextid bikeselection_title\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n\titem_pull\r\n\t{\r\n\t\tname id_curbike\r\n\t\trect 0.137500 0.177778 0.375000 0.200000\r\n\t\teditbox\r\n\t\t{\r\n\t\t\tbackcolor 255 220 220 220\r\n\t\t\tcolor1 255 60 60 60\r\n\t\t\tcolor2 255 0 0 0\r\n\t\t}\r\n\t\tpullbackcolor 200 0 0 0\r\n\t\tpulltextcolor2 255 40 60 240\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_bikeinfo\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX1\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_back\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.000000 0.966667 0.100000 1.000000\r\n\t\t\tsprite2 back1.tga\r\n\t\t\tsprite3 back2.tga\r\n\t\t}\r\n\t\ttextid Back\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP1\r\n\t\tsprite dialog420x200.tga\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT4\r\n\t\ttextid bikeinfo_title\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_AUTHOR\r\n\t\tcolor 255 0 0 0\r\n\t}\r\n}\r\n",
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let setup = fs::read_to_string(ui.join("testsetup.mnu")).unwrap();
        assert!(setup.contains("name idd_testing_setup\n"));
        assert!(setup.contains("name idd_testing_set\n"));
        assert!(setup.contains("name idd_track_image\n"));
        assert!(setup.contains("name idd_track_info\n"));
        assert!(setup.contains("name idd_bikechoose\n"));
        assert!(setup.contains("name idd_bikeinfo\n"));
        assert!(
            !setup.contains("color 240 170 180 190"),
            "beige track strip must go"
        );
        assert!(
            setup.contains("sprite setupbar.tga"),
            "track strip must be rounded glass, not a square darkbox"
        );
        let chrome = setup
            .split("name idd_testing_setup\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            !chrome.contains("item_darkbox"),
            "track strip darkbox must be replaced by rounded bitmap"
        );
        assert!(SETUP_BOARD_Y1 < SETUP_CHROME_Y0);
        assert!(ui.join("setupboard_l.tga").is_file());
        assert!(ui.join("setupboard_r.tga").is_file());
        assert!(ui.join("setupbar.tga").is_file());
        let board = fs::read(ui.join("setupboard_r.tga")).unwrap();
        let bw = u16::from_le_bytes([board[12], board[13]]) as usize;
        let bh = u16::from_le_bytes([board[14], board[15]]) as usize;
        let bpx = &board[18..];
        let bi = ((bh / 2) * bw + bw / 2) * 4;
        assert_eq!(
            [bpx[bi], bpx[bi + 1], bpx[bi + 2], bpx[bi + 3]],
            [0, 0, 0, 160],
            "setup boards use the same glass alpha as serverboard"
        );
        let bar = fs::read(ui.join("setupbar.tga")).unwrap();
        let bar_w = u16::from_le_bytes([bar[12], bar[13]]) as usize;
        let bar_h = u16::from_le_bytes([bar[14], bar[15]]) as usize;
        let bar_px = &bar[18..];
        assert_eq!(bar_px[3], 0, "setupbar.tga corners must stay clear");
        let bari = ((bar_h / 2) * bar_w + bar_w / 2) * 4;
        assert_eq!(
            [bar_px[bari], bar_px[bari + 1], bar_px[bari + 2], bar_px[bari + 3]],
            [0, 0, 0, 160],
            "setupbar uses the same glass alpha as boards"
        );
        assert!(setup.contains("sprite1 done1.tga"));
        assert!(setup.contains(&format!(
            "rect 0.030000 {SETUP_CHROME_Y0:.6} 0.140000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(setup.contains(&format!(
            "rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        let form = setup
            .split("name idd_testing_set\n")
            .nth(1)
            .unwrap()
            .split("name idd_track_image\n")
            .next()
            .unwrap();
        assert!(form.contains("sprite setupboard_r.tga"));
        assert!(form.contains(&format!("0.993750 {SETUP_BOARD_Y1:.6}")));
        assert!(!form.contains("0.944444"));
        assert!(form.contains(&format!("color {}", argb(255, TEXT))));
        assert!(form.contains(&format!("backcolor {}", argb(255, ROW))));
        assert!(!form.contains("backcolor 255 220 220 220"));
        assert!(!form.contains("pulltextcolor2 255 40 60 240"));
        assert!(form.contains(&format!("pulltextcolor2 {}", argb(255, DEFAULT_PRIMARY))));
        let left = setup.split("name idd_track_image\n").nth(1).unwrap();
        assert!(left.contains("sprite setupboard_l.tga"));
        assert!(left.contains(&format!("0.393750 {SETUP_BOARD_Y1:.6}")));
        assert!(left.contains(&format!(
            "rect 0.160000 {SETUP_CHROME_Y0:.6} 0.300000 {SETUP_CHROME_Y1:.6}"
        )));
        let track_info = left
            .split("name ID_TRACK_INFO\n")
            .nth(1)
            .unwrap_or(left);
        assert!(
            track_info.contains("align center"),
            "Track Info label must be centered"
        );
        assert!(!track_info.contains("align right"));
        let tip_y = (SETUP_CHROME_Y1 - SETUP_CHROME_Y0 - 0.022222) * 0.5;
        assert!(
            track_info.contains(&format!("pos 0.000000 {tip_y:.6}")),
            "Track Info label must be vertically centered on chrome"
        );
        let start = setup
            .split("name id_start\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(start.contains("done1.tga"));
        assert!(start.contains(&format!("color1 {}", argb(255, ink_on_rgb(DEFAULT_PRIMARY)))));
        let info = setup
            .split("name idd_track_info\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            info.contains("sprite trackinfoboard.tga"),
            "Track Info modal must use glass board"
        );
        assert!(!info.contains("dialog940x700.tga"));
        assert!(info.contains("color 160 0 0 0"), "scrim should match glass alpha");
        assert!(info.contains("name id_trackmap"));
        assert!(info.contains("name ID_TRACKMAP_MASK\n"));
        assert!(info.contains("sprite trackmapmask.tga"));
        assert!(info.contains(&format!("rect {TRACKMAP_SETUP_RECT}")));
        assert!(info.contains("name ID_TRACK_AUTHOR"));
        assert!(info.contains("name ID_CLOSE"));
        assert!(
            info.contains("0.787500"),
            "meta column must widen to board inner edge"
        );
        assert!(
            info.contains(&format!("color {}", argb(255, DIM))),
            "labels use DIM ink"
        );
        assert!(
            info.contains(&format!("color {}", argb(255, TEXT))),
            "values use TEXT ink"
        );
        let close = info.split("name ID_CLOSE\n").nth(1).unwrap_or(info);
        assert!(
            close.contains("align center"),
            "Close label must be centered"
        );
        assert!(!close.contains("align right"));
        assert!(ui.join("trackinfoboard.tga").is_file());
        let tib = fs::read(ui.join("trackinfoboard.tga")).unwrap();
        let tw = u16::from_le_bytes([tib[12], tib[13]]) as usize;
        let th = u16::from_le_bytes([tib[14], tib[15]]) as usize;
        let tpx = &tib[18..];
        assert_eq!(tpx[3], 0, "trackinfoboard corners must stay clear");
        let ti = ((th / 2) * tw + tw / 2) * 4;
        assert_eq!(
            [tpx[ti], tpx[ti + 1], tpx[ti + 2], tpx[ti + 3]],
            [0, 0, 0, 160],
            "trackinfoboard uses the same glass alpha as boards"
        );
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("testsetup.mnu"));
        assert!(man.contains("setupboard_l.tga"));
        assert!(man.contains("setupboard_r.tga"));
        assert!(man.contains("setupbar.tga"));
        assert!(man.contains("trackinfoboard.tga"));
        let bike = setup
            .split("name idd_bikechoose\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            bike.contains("sprite bikeboard.tga"),
            "bike list board must be glass"
        );
        assert!(bike.contains("sprite biketrackboard.tga"));
        assert!(!bike.contains("dialog600x460.tga"));
        assert!(!bike.contains("dialog320x70.tga"));
        assert!(
            bike.contains("sprite fieldbox.tga"),
            "pulls must sit on fieldbox pills"
        );
        assert!(bike.contains("name ID_BG_id_curbike"));
        assert!(bike.contains("backcolor 0 0 0 0"));
        assert!(
            !bike.contains("backcolor 127 0 0 0"),
            "Bike Selection title band must be cleared"
        );
        assert!(bike.contains(&format!(
            "rect 0.000000 {SETUP_CHROME_Y0:.6} 1.000000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(
            bike.contains("color 0 0 0 0"),
            "footer darkbox must be clear so plaques float"
        );
        assert!(bike.contains(&format!(
            "rect 0.030000 {SETUP_CHROME_Y0:.6} 0.140000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(bike.contains(&format!(
            "rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(bike.contains("sprite1 back1.tga"));
        assert!(bike.contains("sprite1 done1.tga"));
        assert!(bike.contains("sprite1 button1.tga"));
        let bike_start = bike.split("name id_start\n").nth(1).unwrap();
        assert!(
            bike_start.contains("align center"),
            "Start label must be centered"
        );
        assert!(!bike_start.contains("align right"));
        assert!(bike_start.contains(&format!(
            "color1 {}",
            argb(255, ink_on_rgb(DEFAULT_PRIMARY))
        )));
        assert!(bike_start.contains("sprite1 done1.tga"));
        let bike_back = bike.split("name id_back\n").nth(1).unwrap();
        assert!(bike_back.contains("sprite1 back1.tga"));
        assert!(bike_back.contains(&format!("color1 {}", argb(255, TEXT))));
        assert!(ui.join("bikeboard.tga").is_file());
        assert!(ui.join("biketrackboard.tga").is_file());
        assert!(ui.join("bikeinfoboard.tga").is_file());
        let binfo = setup.split("name idd_bikeinfo\n").nth(1).unwrap();
        assert!(binfo.contains("sprite bikeinfoboard.tga"));
        assert!(!binfo.contains("dialog420x200.tga"));
        assert!(
            !binfo.contains("backcolor 127 0 0 0"),
            "title top band must be cleared"
        );
        assert!(binfo.contains("sprite1 back1.tga"));
        assert!(binfo.contains(&format!(
            "rect 0.030000 {SETUP_CHROME_Y0:.6} 0.140000 {SETUP_CHROME_Y1:.6}"
        )));
        let binfo_back = binfo.split("name id_back\n").nth(1).unwrap();
        assert!(binfo_back.contains("align center"));
        assert!(man.contains("bikeboard.tga"));
        assert!(man.contains("biketrackboard.tga"));
        assert!(man.contains("bikeinfoboard.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_options_full_stock_when_fixture_set() {
        let Ok(root) = std::env::var("MXBO_GAME_UI_FIXTURE") else {
            return;
        };
        let game = PathBuf::from(root);
        let ui = game.join("ui");
        assert!(
            ui.join("options.mnu").is_file(),
            "fixture needs ui/options.mnu"
        );
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let opt = fs::read_to_string(ui.join("options.mnu")).unwrap();
        for name in [
            "idd_options",
            "idd_graphic_options",
            "idd_input_options",
            "idd_input2_options",
            "idd_input3_options",
            "idd_misc_options",
            "idd_others_options",
            "idd_wait_control",
            "idd_calibration",
        ] {
            assert!(opt.contains(&format!("name {name}")), "missing {name}");
        }
        assert!(opt.contains("sprite optionsboard.tga"));
        assert!(!opt.contains("optionsrail.tga"));
        assert!(opt.contains("sprite1 done1.tga"));
        assert!(opt.contains("sprite1 segl2.tga"));
        assert!(opt.contains("sprite optionstabtrack.tga"));
        assert!(opt.contains("name id_resolution"));
        assert!(opt.contains("name id_mastervolume"));
        assert!(opt.contains("backcolor 255 20 20 22"));
        assert!(!opt.contains("backcolor 255 228 228 230"));
        assert!(opt.contains(&format!(
            "rect {OPT_FORM_X0:.6} {OPT_FORM_Y0:.6} {OPT_FORM_X1:.6} {OPT_FORM_Y1:.6}"
        )));
        assert!(ui.join("optionsboard.tga").is_file());
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("options.mnu"));
    }

    #[test]
    fn splice_profiles_restyles_main_and_modals() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("profiles.mnu"),
            "dialog\r\n{\r\n\tname idd_profiles\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP1\r\n\t\trect 0.006250 0.133333 0.993750 0.944444\r\n\t\tsprite dialog1580x730.tga\r\n\t}\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_done\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n\t\ttextid Done\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_new\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.012500 0.622222 0.137500 0.655556\r\n\t\t\tsprite1 button1.tga\r\n\t\t}\r\n\t\ttextid new\r\n\t\ttext\r\n\t\t{\r\n\t\t\talign right\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\ttextid profiles\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT2\r\n\t\ttextid language\r\n\t\tcolor 255 255 255 255\r\n\t\tbackcolor 255 0 0 0\r\n\t}\r\n\titem_pull\r\n\t{\r\n\t\tname ID_LANGUAGE\r\n\t\trect 0.012500 0.177778 0.200000 0.200000\r\n\t\tbackcolor 255 220 220 220\r\n\t\tpullbackcolor 200 0 0 0\r\n\t}\r\n\titem_editbox\r\n\t{\r\n\t\tname ID_NICKNAME\r\n\t\trect 0.012500 0.288889 0.200000 0.311111\r\n\t\teditbox\r\n\t\t{\r\n\t\t\tbackcolor 255 220 220 220\r\n\t\t}\r\n\t}\r\n\titem_pull\r\n\t{\r\n\t\tname ID_CURPROFILE\r\n\t\trect 0.012500 0.466667 0.262500 0.488889\r\n\t\tbackcolor 255 220 220 220\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_profilemodify\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\tsprite dialog600x300.tga\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_ok\r\n\t\tbutton\r\n\t\t{\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n\t\ttext\r\n\t\t{\r\n\t\t\talign left\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t}\r\n\titem_editbox\r\n\t{\r\n\t\tname id_name\r\n\t\trect 0.443750 0.344444 0.681250 0.366667\r\n\t\teditbox\r\n\t\t{\r\n\t\t\tbackcolor 255 220 220 220\r\n\t\t}\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_profiledeleteconfirm\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\tsprite dialog600x200.tga\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_yes\r\n\t\tbutton\r\n\t\t{\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n\t\ttext\r\n\t\t{\r\n\t\t\talign left\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_bestlaps\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP1\r\n\t\trect 0.006250 0.133333 0.993750 0.944444\r\n\t\tsprite dialog1580x730.tga\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_back\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.000000 0.966667 0.100000 1.000000\r\n\t\t\tsprite2 back1.tga\r\n\t\t\tsprite3 back2.tga\r\n\t\t}\r\n\t\ttextid Back\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_export\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.012500 0.900000 0.137500 0.933333\r\n\t\t\tsprite1 button1.tga\r\n\t\t\tsprite2 button2.tga\r\n\t\t\tsprite3 button3.tga\r\n\t\t}\r\n\t\ttextid Export\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\ttextid bestlaps_title\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_bestoverall\r\n\titem_list\r\n\t{\r\n\t\tname id_table\r\n\t\tlist\r\n\t\t{\r\n\t\t\tcolor1 255 80 80 80\r\n\t\t\tcolor2 255 0 0 255\r\n\t\t\tcolor3 255 0 255 255\r\n\t\t\tbackcolor 255 220 220 220\r\n\t\t\tsortbartextcolor 255 0 0 0\r\n\t\t\tsortbackcolor 255 180 180 180\r\n\t\t}\r\n\t}\r\n}\r\n",
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let profiles = fs::read_to_string(ui.join("profiles.mnu")).unwrap();
        assert!(profiles.contains("name idd_profiles\n"));
        assert!(profiles.contains("name idd_profilemodify\n"));
        assert!(profiles.contains("name idd_profiledeleteconfirm\n"));
        assert!(profiles.contains("name idd_bestlaps\n"));
        assert!(profiles.contains("name idd_bestoverall\n"));
        let main = profiles
            .split("name idd_profiles\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            !main.contains("backcolor 127 0 0 0"),
            "title band must be cleared"
        );
        assert!(
            !main.contains("backcolor 255 0 0 0"),
            "section bars must be cleared"
        );
        assert!(main.contains("sprite fieldbox.tga"));
        assert!(main.contains("name ID_BG_ID_LANGUAGE"));
        assert!(main.contains("name ID_BG_ID_NICKNAME"));
        assert!(main.contains("name ID_BG_ID_CURPROFILE"));
        assert!(
            main.contains("sprite profilesboard.tga"),
            "middle board must be frosted glass"
        );
        assert!(!main.contains("dialog1580x730.tga"));
        assert!(
            main.contains(&format!("0.993750 {SETUP_BOARD_Y1:.6}")),
            "board must stop above Done chrome"
        );
        assert!(!main.contains("0.944444"));
        assert!(main.contains("sprite1 done1.tga"));
        assert!(main.contains(&format!(
            "rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        let done = main.split("name id_done\n").nth(1).unwrap();
        assert!(done.contains("align center"));
        assert!(done.contains(&format!(
            "color1 {}",
            argb(255, ink_on_rgb(DEFAULT_PRIMARY))
        )));
        let modify = profiles
            .split("name idd_profilemodify\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(modify.contains("sprite profilemodifyboard.tga"));
        assert!(!modify.contains("dialog600x300.tga"));
        assert!(modify.contains("sprite1 button1.tga"));
        assert!(modify.contains("name ID_BG_id_name"));
        let del = profiles
            .split("name idd_profiledeleteconfirm\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(del.contains("sprite profiledeleteboard.tga"));
        assert!(!del.contains("dialog600x200.tga"));
        assert!(del.contains("sprite1 button1.tga"));
        let best = profiles
            .split("name idd_bestlaps\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            !best.contains("backcolor 127 0 0 0"),
            "Best Laps title band must be cleared"
        );
        assert!(best.contains("sprite profilesboard.tga"));
        assert!(!best.contains("dialog1580x730.tga"));
        assert!(best.contains(&format!("0.993750 {SETUP_BOARD_Y1:.6}")));
        assert!(best.contains("sprite1 back1.tga"));
        assert!(best.contains(&format!(
            "rect 0.030000 {SETUP_CHROME_Y0:.6} 0.140000 {SETUP_CHROME_Y1:.6}"
        )));
        let export = best.split("name id_export\n").nth(1).unwrap();
        assert!(export.contains(&format!(
            "rect 0.160000 {SETUP_CHROME_Y0:.6} 0.300000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(export.contains("align center"));
        assert!(export.contains(&format!("color1 {}", argb(255, TEXT))));
        let overall = profiles
            .split("name idd_bestoverall\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(overall.contains("backcolor 160 0 0 0"));
        assert!(overall.contains("sortbackcolor 160 0 0 0"));
        assert!(!overall.contains("sortbackcolor 255 180 180 180"));
        assert!(!overall.contains(&format!("sortbackcolor {}", argb(160, ROW))));
        assert!(!overall.contains("color2 255 0 0 255"));
        assert!(ui.join("profilesboard.tga").is_file());
        assert!(ui.join("profilemodifyboard.tga").is_file());
        assert!(ui.join("profiledeleteboard.tga").is_file());
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("profiles.mnu"));
        assert!(man.contains("profilesboard.tga"));
        assert!(man.contains("profilemodifyboard.tga"));
        assert!(man.contains("profiledeleteboard.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_viewreplays_restyles_main_and_delete() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("viewreplays.mnu"),
            "dialog\r\n{\r\n\tname idd_viewreplays\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX1\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP1\r\n\t\trect 0.006250 0.133333 0.993750 0.944444\r\n\t\tsprite dialog1580x730.tga\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_back\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.000000 0.966667 0.100000 1.000000\r\n\t\t\tsprite2 back1.tga\r\n\t\t\tsprite3 back2.tga\r\n\t\t}\r\n\t\ttextid Back\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n\titem_list\r\n\t{\r\n\t\tname id_replaylist\r\n\t\tlist\r\n\t\t{\r\n\t\t\tcolor1 255 80 80 80\r\n\t\t\tcolor2 255 40 60 240\r\n\t\t\tcolor3 255 0 0 0\r\n\t\t\tbackcolor 255 220 220 220\r\n\t\t\tsortbartextcolor 255 0 0 0\r\n\t\t\tsortbartextcolor2 255 80 80 80\r\n\t\t\tsortbartextcolor3 255 40 60 240\r\n\t\t\tsortbackcolor 255 200 200 200\r\n\t\t}\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_view\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n\t\ttextid view\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_delete\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.012500 0.900000 0.137500 0.933333\r\n\t\t\tsprite1 button1.tga\r\n\t\t\tsprite2 button2.tga\r\n\t\t\tsprite3 button3.tga\r\n\t\t}\r\n\t\ttextid delete\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\ttextid Viewreplays\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_confirm_replay_delete\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\tsprite dialog600x200.tga\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_yes\r\n\t\tbutton\r\n\t\t{\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n\t\ttext\r\n\t\t{\r\n\t\t\talign left\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t}\r\n}\r\n",
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("viewreplays.mnu")).unwrap();
        assert!(mnu.contains("name idd_viewreplays\n"));
        assert!(mnu.contains("name idd_confirm_replay_delete\n"));
        let main = mnu
            .split("name idd_viewreplays\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            !main.contains("backcolor 127 0 0 0"),
            "title band must be cleared"
        );
        assert!(main.contains("sprite profilesboard.tga"));
        assert!(!main.contains("dialog1580x730.tga"));
        assert!(main.contains(&format!("0.993750 {SETUP_BOARD_Y1:.6}")));
        assert!(main.contains("backcolor 160 0 0 0"));
        assert!(main.contains("sortbackcolor 160 0 0 0"));
        assert!(!main.contains("sortbackcolor 255 200 200 200"));
        assert!(main.contains("sprite1 back1.tga"));
        assert!(main.contains("sprite1 done1.tga"));
        assert!(main.contains(&format!(
            "rect 0.030000 {SETUP_CHROME_Y0:.6} 0.140000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(main.contains(&format!(
            "rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        let del = main.split("name id_delete\n").nth(1).unwrap();
        assert!(del.contains(&format!(
            "rect 0.160000 {SETUP_CHROME_Y0:.6} 0.300000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(del.contains("align center"));
        let view = main.split("name id_view\n").nth(1).unwrap();
        assert!(view.contains("align center"));
        assert!(view.contains(&format!(
            "color1 {}",
            argb(255, ink_on_rgb(DEFAULT_PRIMARY))
        )));
        let confirm = mnu
            .split("name idd_confirm_replay_delete\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(confirm.contains("sprite profiledeleteboard.tga"));
        assert!(!confirm.contains("dialog600x200.tga"));
        assert!(confirm.contains("sprite1 button1.tga"));
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("viewreplays.mnu"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_hostsetup_clears_title_glass_board_and_chrome() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("hostsetup.mnu"),
            "dialog\r\n{\r\n\tname idd_host_setup\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX1\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP1\r\n\t\trect 0.006250 0.133333 0.993750 0.944444\r\n\t\tsprite dialog1580x730.tga\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_continue\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done2.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n\t\ttextid continue\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname id_back\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.000000 0.966667 0.100000 1.000000\r\n\t\t\tsprite2 back2.tga\r\n\t\t\tsprite3 back2.tga\r\n\t\t}\r\n\t\ttextid Back\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n\titem_editbox\r\n\t{\r\n\t\tname id_sessionname\r\n\t\trect 0.137500 0.144444 0.262500 0.166667\r\n\t\teditbox\r\n\t\t{\r\n\t\t\tcolor1 255 80 80 80\r\n\t\t\tbackcolor 255 220 220 220\r\n\t\t}\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\ttextid hostsetup_title\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_race_setup\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\tcolor 255 0 0 0\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n}\r\n",
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("hostsetup.mnu")).unwrap();
        assert!(mnu.contains("name idd_host_setup\n"));
        assert!(mnu.contains("name idd_race_setup\n"));
        let main = mnu
            .split("name idd_host_setup\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            !main.contains("backcolor 127 0 0 0"),
            "title band must be cleared"
        );
        assert!(main.contains("sprite profilesboard.tga"));
        assert!(!main.contains("dialog1580x730.tga"));
        assert!(main.contains(&format!("0.993750 {SETUP_BOARD_Y1:.6}")));
        assert!(main.contains("sprite1 back1.tga"));
        assert!(main.contains("sprite1 done1.tga"));
        assert!(!main.contains("sprite2 back2.tga\n\t\t\tsprite3 back2.tga"));
        assert!(!main.contains("sprite2 done2.tga\n\t\t\tsprite3 done2.tga"));
        assert!(main.contains(&format!(
            "rect 0.030000 {SETUP_CHROME_Y0:.6} 0.140000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(main.contains(&format!(
            "rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(main.contains("sprite fieldbox.tga"));
        let cont = main.split("name id_continue\n").nth(1).unwrap();
        assert!(cont.contains("align center"));
        assert!(cont.contains(&format!(
            "color1 {}",
            argb(255, ink_on_rgb(DEFAULT_PRIMARY))
        )));
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("hostsetup.mnu"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_hostsetup_restyles_race_setup_shell_and_form() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("hostsetup.mnu"),
            concat!(
                "dialog\r\n{\r\n\tname idd_race_setup\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX1\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.006250 0.133333 0.993750 0.188889\r\n\t\tcolor 240 170 180 190\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_back\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.000000 0.966667 0.100000 1.000000\r\n",
                "\t\t\tsprite2 back1.tga\r\n\t\t\tsprite3 back2.tga\r\n\t\t}\r\n",
                "\t\ttextid Back\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign left\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_start\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n",
                "\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n",
                "\t\ttextid Start\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_BIKECHOOSE\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.718750 0.966667 0.843750 1.000000\r\n",
                "\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n",
                "\t\ttextid bike_selection\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\ttextid racesetup_title\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_pull\r\n\t{\r\n\t\tname id_changetrack\r\n\t\trect 0.137500 0.144444 0.512500 0.177778\r\n",
                "\t\teditbox\r\n\t\t{\r\n\t\t\tbackcolor 255 220 220 220\r\n\t\t}\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_host_racesetup\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.406250 0.200000 0.993750 0.944444\r\n\t\tsprite dialog940x670.tga\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_RACETEXT\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 255 0 0 0\r\n\t}\r\n",
                "\titem_pull\r\n\t{\r\n\t\tname ID_CLASS\r\n\t\trect 0.412500 0.477778 0.562500 0.500000\r\n",
                "\t\teditbox\r\n\t\t{\r\n\t\t\tbackcolor 255 220 220 220\r\n\t\t}\r\n\t}\r\n}\r\n",
            ),
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("hostsetup.mnu")).unwrap();
        let race = mnu
            .split("name idd_race_setup\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            !race.contains("backcolor 127 0 0 0"),
            "title band must be cleared"
        );
        assert!(race.contains("sprite setupbar.tga"));
        assert!(!race.contains("color 240 170 180 190"));
        assert!(race.contains("sprite1 back1.tga"));
        assert!(race.contains("sprite1 done1.tga"));
        assert!(race.contains("sprite1 button1.tga"));
        assert!(race.contains(&format!(
            "rect 0.030000 {SETUP_CHROME_Y0:.6} 0.140000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(race.contains(&format!(
            "rect 0.440000 {SETUP_CHROME_Y0:.6} 0.560000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(race.contains(&format!(
            "rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        let start = race.split("name id_start\n").nth(1).unwrap();
        assert!(start.contains("align center"));
        assert!(start.contains(&format!(
            "color1 {}",
            argb(255, ink_on_rgb(DEFAULT_PRIMARY))
        )));
        let form = mnu
            .split("name idd_host_racesetup\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(form.contains("sprite setupboard_r.tga"));
        assert!(!form.contains("dialog940x670.tga"));
        assert!(form.contains(&format!("0.993750 {SETUP_BOARD_Y1:.6}")));
        assert!(
            !form.contains("backcolor 255 0 0 0"),
            "section headers must be clear on glass"
        );
        assert!(form.contains("sprite fieldbox.tga"));
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("hostsetup.mnu"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_replay_glasses_chrome_classification_and_save() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("replay.mnu"),
            concat!(
                "dialog\r\n{\r\n\tname idd_replay\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname idc_darkbox1\r\n\t\trect 0.675000 0.000000 1.000000 0.144444\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname idc_darkbox2\r\n\t\trect 0.000000 0.866667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.012500 0.877778 0.987500 0.897778\r\n\t\tcolor 255 230 230 230\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_save\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.656250 0.922222 0.781250 0.955556\r\n",
                "\t\t\tsprite1 r_button1.tga\r\n\t\t\tsprite2 r_button2.tga\r\n\t\t\tsprite3 r_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid save\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 0 0 0\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_SETTINGS\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.406250 0.966667 0.531250 1.000000\r\n",
                "\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid settings\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_done\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n",
                "\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n",
                "\t\ttextid Done\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_replay_classification\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_SESSION\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_list\r\n\t{\r\n\t\tname ID_LIST\r\n\t\tlist\r\n\t\t{\r\n",
                "\t\t\tbackcolor 127 0 0 0\r\n\t\t}\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_replay_telemetry\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.900000 0.700000 0.993750 0.822222\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_replay_save\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\tsprite dialog600x600.tga\r\n\t}\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.318750 0.788889 0.681250 0.822222\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_ok\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.318750 0.788889 0.418750 0.822222\r\n",
                "\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid ok\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign left\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_cancel\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.581250 0.788889 0.681250 0.822222\r\n",
                "\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid cancel\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n}\r\n"
            ),
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("replay.mnu")).unwrap();
        let main = mnu
            .split("name idd_replay\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(main.contains("sprite replaypanel.tga"));
        assert!(!main.contains("name idc_darkbox1"));
        assert!(!main.contains("name idc_darkbox2"));
        assert!(!main.contains("255 230 230 230"));
        assert!(!main.contains("r_button"));
        assert!(main.contains("sprite1 button1.tga"));
        assert!(main.contains("sprite1 done1.tga"));
        assert!(main.contains("rect 0.800000 0.966667 0.970000 1.000000"));
        let save = main.split("name id_save\n").nth(1).unwrap();
        assert!(save.contains("align center"));
        let class = mnu
            .split("name idd_replay_classification\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(class.contains("backcolor 160 0 0 0"));
        assert!(!class.contains("backcolor 127 0 0 0"));
        let telem = mnu
            .split("name idd_replay_telemetry\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(telem.contains("sprite replaypanel.tga"));
        assert!(!telem.contains("item_darkbox"));
        let save_dlg = mnu
            .split("name idd_replay_save\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(save_dlg.contains("sprite replayboard.tga"));
        assert!(!save_dlg.contains("dialog600x600.tga"));
        assert!(save_dlg.contains("sprite1 button1.tga"));
        assert!(ui.join("replaypanel.tga").is_file());
        assert!(ui.join("replayboard.tga").is_file());
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("replay.mnu"));
        assert!(man.contains("replaypanel.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_multiclient_restyles_race_exit() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("multiclient.mnu"),
            "dialog\r\n{\r\n\tname idd_multi_pit\r\n\titem_text\r\n\t{\r\n\t\tname ID_KEEP\r\n\t\tcolor 255 0 0 0\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_raceexit\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.000000 0.000000 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.312500 0.388889 0.687500 0.611111\r\n\t\tsprite dialog600x200.tga\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\trect 0.318750 0.400000 0.681250 0.422222\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.000000 0.000000\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign center\r\n\t\t}\r\n\t\ttextid exit_race\r\n\t\tcolor 255 0 0 0\r\n\t\tbackcolor 0 0 0 0\r\n\t}\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX2\r\n\t\trect 0.318750 0.566667 0.681250 0.600000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname ID_YES\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.318750 0.566667 0.418750 0.600000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n\t\ttextid yes\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 0\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname ID_NO\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.581250 0.566667 0.681250 0.600000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n\t\ttextid no\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 0\r\n\t}\r\n}\r\n",
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("multiclient.mnu")).unwrap();
        assert!(mnu.contains("name idd_multi_pit\n"));
        assert!(mnu.contains("name idd_raceexit\n"));
        let pit = mnu
            .split("name idd_multi_pit\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            pit.contains(&format!("\tcolor {}\n", argb(255, TEXT))),
            "multi_pit ink remaps through glass restyle"
        );
        let exit = mnu
            .split("name idd_raceexit\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(exit.contains("sprite dialog600x200.tga"));
        assert!(!exit.contains("profiledeleteboard.tga"));
        assert!(exit.contains("color 160 0 0 0"));
        assert!(
            exit.contains("rect 0.318750 0.566667 0.681250 0.600000\n\t\tcolor 0 0 0 0"),
            "footer strip must clear so plaques float"
        );
        assert!(!exit.contains("\tcolor 255 0 0 0\n"));
        assert!(exit.contains(&format!("\tcolor 255 {}\n", bgr(TEXT))));
        assert!(exit.contains("sprite1 button1.tga"));
        let yes = exit.split("name ID_YES\n").nth(1).unwrap();
        assert!(yes.contains("align center"));
        assert!(!yes.contains("align left"));
        assert!(yes.contains(&format!("color1 {}", argb(255, TEXT))));
        let no = exit.split("name ID_NO\n").nth(1).unwrap();
        assert!(no.contains("align center"));
        assert!(!no.contains("align right"));
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("multiclient.mnu"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_test_restyles_testing_exit() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("test.mnu"),
            "dialog\r\n{\r\n\tname idd_testing\r\n\titem_text\r\n\t{\r\n\t\tname ID_KEEP\r\n\t\tcolor 255 0 0 0\r\n\t}\r\n}\r\n\r\ndialog\r\n{\r\n\tname idd_testingexit\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.000000 0.000000 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.312500 0.388889 0.687500 0.611111\r\n\t\tsprite dialog600x200.tga\r\n\t}\r\n\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\trect 0.318750 0.400000 0.681250 0.422222\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.000000 0.000000\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign center\r\n\t\t}\r\n\t\ttextid testing_exit\r\n\t\tcolor 255 0 0 0\r\n\t\tbackcolor 0 0 0 0\r\n\t}\r\n\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX2\r\n\t\trect 0.318750 0.566667 0.681250 0.600000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname ID_YES\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.318750 0.566667 0.418750 0.600000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n\t\ttextid yes\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 0\r\n\t}\r\n\titem_button\r\n\t{\r\n\t\tname ID_NO\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.581250 0.566667 0.681250 0.600000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n\t\ttextid no\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 0\r\n\t}\r\n}\r\n",
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("test.mnu")).unwrap();
        let keep = mnu
            .split("name idd_testing\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(
            keep.contains(&format!("\tcolor 255 {}\n", bgr(TEXT))),
            "unhandled test.mnu dialogs still get shared ink recolor"
        );
        let exit = mnu
            .split("name idd_testingexit\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(exit.contains("sprite dialog600x200.tga"));
        assert!(!exit.contains("profiledeleteboard.tga"));
        assert!(exit.contains("sprite1 button1.tga"));
        assert!(exit.split("name ID_YES\n").nth(1).unwrap().contains("align center"));
        assert!(exit.split("name ID_NO\n").nth(1).unwrap().contains("align center"));
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("test.mnu"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_test_glasses_practice_pit_and_panes() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("test.mnu"),
            concat!(
                "dialog\r\n{\r\n\tname testing_pit\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_start\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n",
                "\t\ttextid totrack\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_back\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.000000 0.966667 0.100000 1.000000\r\n\t\t\tsprite2 back1.tga\r\n\t\t\tsprite3 back2.tga\r\n\t\t}\r\n",
                "\t\ttextid Back\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_SETTINGS\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.406250 0.966667 0.531250 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid settings\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_replay\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.562500 0.966667 0.687500 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid replay\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_garage\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.718750 0.966667 0.843750 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid garage\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_PHOTO\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.243750 0.966667 0.262500 1.000000\r\n\t\t}\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname id_weather\r\n\t\trect 0.006250 0.166667 0.993750 0.188889\r\n",
                "\t\tcolor 255 0 0 0\r\n\t\tbackcolor 240 170 180 190\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\ttextid Testing\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_eventinfo\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.131250 0.200000 0.993750 0.944444\r\n\t\tsprite dialog1380x670t.tga\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_RESETTRACK\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.137500 0.900000 0.262500 0.933333\r\n\t\t\tsprite1 button1.tga\r\n\t\t}\r\n",
                "\t\ttextid reset_track\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_laps\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.131250 0.200000 0.993750 0.944444\r\n\t\tsprite dialog1380x670t.tga\r\n\t}\r\n",
                "\titem_list\r\n\t{\r\n\t\tname id_lapstable\r\n\t\tlist\r\n\t\t{\r\n\t\t\tbackcolor 255 220 220 220\r\n",
                "\t\t\tsortbartextcolor 255 0 0 0\r\n\t\t\tsortbackcolor 255 200 200 200\r\n\t\t}\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\trect 0.137500 0.855556 0.262500 0.877778\r\n",
                "\t\ttextid ideal_lap_time\r\n\t\tcolor 255 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_export\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.137500 0.900000 0.262500 0.933333\r\n\t\t\tsprite1 button1.tga\r\n\t\t}\r\n",
                "\t\ttextid Export\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_track_info_pit\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.131250 0.200000 0.993750 0.944444\r\n\t\tsprite dialog1380x670t.tga\r\n\t}\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname id_trackmap\r\n\t\trect 0.137500 0.211111 0.543750 0.933333\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_trainer\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.131250 0.200000 0.993750 0.944444\r\n\t\tsprite dialog1380x670t.tga\r\n\t}\r\n",
                "\titem_list\r\n\t{\r\n\t\tname ID_LIST\r\n\t\tlist\r\n\t\t{\r\n\t\t\tbackcolor 255 220 220 220\r\n\t\t}\r\n\t}\r\n}\r\n"
            ),
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("test.mnu")).unwrap();
        let pit = mnu
            .split("name testing_pit\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(!pit.contains("backcolor 240 170 180 190"));
        assert!(!pit.contains("backcolor 127 0 0 0"));
        assert!(
            !pit.contains("sprite setupbar.tga"),
            "Practice clears weather — no frost bar"
        );
        assert!(pit.contains("name ID_PITGROUP\n"));
        assert!(pit.contains("sprite optionstabtrack.tga"));
        assert!(pit.contains("sprite1 segl1.tga"));
        assert!(pit.contains("sprite1 segm1.tga"));
        assert!(pit.contains("sprite1 segr1.tga"));
        assert!(pit.contains(&format!(
            "rect {PIT_GROUP_X0:.6} {SETUP_CHROME_Y0:.6} {PIT_GROUP_X1:.6} {SETUP_CHROME_Y1:.6}"
        )));
        assert!(pit.contains(&format!(
            "rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(pit.contains(&format!(
            "rect 0.155000 {SETUP_CHROME_Y0:.6} 0.185000 {SETUP_CHROME_Y1:.6}"
        )));
        let info = mnu
            .split("name idd_eventinfo\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(info.contains("sprite raceinfoboard.tga"));
        assert!(info.contains(&format!("0.993750 {SETUP_BOARD_Y1:.6}")));
        assert!(info.contains("rect 0.137500 0.850000 0.262500 0.885000"));
        assert!(!info.contains("0.137500 0.900000 0.262500 0.933333"));
        for pane in ["idd_laps", "idd_track_info_pit", "idd_trainer"] {
            let body = mnu
                .split(&format!("name {pane}\n"))
                .nth(1)
                .unwrap()
                .split("\ndialog\n")
                .next()
                .unwrap();
            assert!(body.contains("sprite raceinfoboard.tga"), "{pane}");
            assert!(
                body.contains(&format!("0.993750 {SETUP_BOARD_Y1:.6}")),
                "{pane} board height"
            );
        }
        let laps = mnu
            .split("name idd_laps\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(laps.contains("backcolor 160 0 0 0"));
        assert!(laps.contains("sortbackcolor 160 0 0 0"));
        assert!(!laps.contains("sortbackcolor 255 200 200 200"));
        assert!(laps.contains(&format!("sortbartextcolor {}", argb(255, TEXT))));
        assert!(laps.contains("rect 0.800000 0.850000 0.968750 0.885000"));
        assert!(!laps.contains("rect 0.137500 0.850000 0.262500 0.885000"));
        assert!(!laps.contains("0.137500 0.900000 0.262500 0.933333"));
        let track = mnu
            .split("name idd_track_info_pit\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(track.contains("rect 0.137500 0.211111 0.543750 0.880000"));
        assert!(!track.contains("0.543750 0.933333"));
        assert!(track.contains("name ID_TRACKMAP_MASK\n"));
        assert!(track.contains("sprite trackmapmask.tga"));
        assert!(ui.join("raceinfoboard.tga").is_file());
        assert!(ui.join("trackmapmask.tga").is_file());
        let mask = fs::read(ui.join("trackmapmask.tga")).unwrap();
        let mpx = &mask[18..];
        assert_eq!(mpx[3], 160, "mask corner must be frosted glass");
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("test.mnu"));
        assert!(man.contains("raceinfoboard.tga"));
        assert!(man.contains("trackmapmask.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_test_glasses_on_track_panel() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("test.mnu"),
            concat!(
                "dialog\r\n{\r\n\tname testing_pit\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_KEEP\r\n\t\tcolor 255 0 0 0\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname test_panel\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.375000 0.222222 0.625000 0.444444\r\n",
                "\t\tsprite dialog400x200.tga\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_CONTINUE\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.425000 0.233333 0.575000 0.266667\r\n",
                "\t\t\tsprite1 w_button1.tga\r\n\t\t\tsprite2 w_button2.tga\r\n\t\t\tsprite3 w_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid continue_ontrack\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_TOPIT\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.425000 0.288889 0.575000 0.322222\r\n",
                "\t\t\tsprite1 w_button1.tga\r\n\t\t\tsprite2 w_button2.tga\r\n\t\t\tsprite3 w_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid returntopit\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_REPLAY\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.425000 0.344444 0.575000 0.377778\r\n",
                "\t\t\tsprite1 w_button1.tga\r\n\t\t\tsprite2 w_button2.tga\r\n\t\t\tsprite3 w_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid replay\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_SETTINGS\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.425000 0.400000 0.575000 0.433333\r\n",
                "\t\t\tsprite1 w_button1.tga\r\n\t\t\tsprite2 w_button2.tga\r\n\t\t\tsprite3 w_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid settings\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t\tcolor3 255 0 0 0\r\n\t}\r\n}\r\n"
            ),
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("test.mnu")).unwrap();
        let panel = mnu
            .split("name test_panel\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(panel.contains("sprite testpanelboard.tga"));
        assert!(!panel.contains("dialog400x200.tga"));
        assert!(!panel.contains("w_button"));
        assert!(panel.contains("sprite1 button1.tga"));
        for name in ["ID_CONTINUE", "ID_TOPIT", "ID_REPLAY", "ID_SETTINGS"] {
            let btn = panel.split(&format!("name {name}\n")).nth(1).unwrap();
            assert!(btn.contains("align center"), "{name}");
            assert!(!btn.contains("align right"), "{name}");
        }
        assert!(ui.join("testpanelboard.tga").is_file());
        let board = fs::read(ui.join("testpanelboard.tga")).unwrap();
        let bpx = &board[18..];
        assert_eq!(bpx[3], 0, "testpanelboard corners must stay clear");
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("testpanelboard.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn main_mnu_exit_confirm_uses_solid_board() {
        let mnu = main_mnu(DEFAULT_PRIMARY);
        let exit = mnu
            .split("name exit_confirm\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(exit.contains("sprite dialog600x200.tga"));
        assert!(!exit.contains("profiledeleteboard.tga"));
        assert!(exit.contains("sprite1 button1.tga"));
        assert!(exit.contains("sprite2 button2.tga"));
        assert!(exit.contains("sprite3 button3.tga"));
        assert!(!exit.contains("b_button"));
        assert!(exit.contains("color 160 0 0 0"));
    }

    #[test]
    fn splice_multiclient_restyles_pit_entries_and_info() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("multiclient.mnu"),
            concat!(
                "dialog\r\n{\r\n\tname idd_multi_pit\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_totrack\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n",
                "\t\ttextid totrack\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_back\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.000000 0.966667 0.100000 1.000000\r\n\t\t\tsprite2 back1.tga\r\n\t\t\tsprite3 back2.tga\r\n\t\t}\r\n",
                "\t\ttextid Back\r\n\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_garage\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.718750 0.966667 0.843750 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid garage\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_replay\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.562500 0.966667 0.687500 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid replay\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_SETTINGS\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.406250 0.966667 0.531250 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid settings\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname id_weather\r\n\t\trect 0.006250 0.166667 0.993750 0.188889\r\n\t\tcolor 255 0 0 0\r\n\t\tbackcolor 240 170 180 190\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname id_raceinfo\r\n\t\trect 0.006250 0.133333 0.993750 0.155556\r\n\t\tcolor 255 0 0 0\r\n\t\tbackcolor 240 170 180 190\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\ttextid raceweekend\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_join\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n",
                "\t\ttextid join\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_race_entries\r\n\titem_list\r\n\t{\r\n\t\tname id_entries_table\r\n\t\tlist\r\n\t\t{\r\n",
                "\t\t\tcolor1 255 80 80 80\r\n\t\t\tcolor2 255 127 127 127\r\n\t\t\tcolor3 255 0 255 255\r\n",
                "\t\t\tbackcolor 255 220 220 220\r\n\t\t\tsortbartextcolor 255 0 0 0\r\n\t\t\tsortbackcolor 255 180 180 180\r\n",
                "\t\t}\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_race_info\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n",
                "\t\trect 0.131250 0.200000 0.993750 0.944444\r\n\t\tsprite dialog1380x670t.tga\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_WEATHER_TEXT\r\n\t\tcolor 255 40 40 40\r\n\t\tbackcolor 0 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_ADMIN\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.137500 0.900000 0.262500 0.933333\r\n\t\t\tsprite1 button1.tga\r\n\t\t}\r\n",
                "\t\ttextid admin\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_RESETTRACK\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.862500 0.900000 0.987500 0.933333\r\n\t\t\tsprite1 button1.tga\r\n\t\t\tsprite2 button2.tga\r\n\t\t\tsprite3 button3.tga\r\n\t\t}\r\n",
                "\t\ttextid reset_track\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_race_results\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n",
                "\t\trect 0.131250 0.200000 0.993750 0.944444\r\n\t\tsprite dialog1380x670t.tga\r\n\t}\r\n",
                "\titem_pull\r\n\t{\r\n\t\tname ID_PULL\r\n\t\trect 0.137500 0.211111 0.262500 0.233333\r\n",
                "\t\ttextcolor 255 60 60 60\r\n\t\tbackcolor 255 240 240 240\r\n",
                "\t\tpullbackcolor 200 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_EXPORT\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.137500 0.900000 0.262500 0.933333\r\n\t\t\tsprite1 button1.tga\r\n\t\t}\r\n",
                "\t\ttextid Export\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_race_practice\r\n\titem_list\r\n\t{\r\n\t\tname ID_STANDINGS_TABLE\r\n\t\tlist\r\n\t\t{\r\n",
                "\t\t\tcolor1 255 80 80 80\r\n\t\t\tcolor2 255 127 127 127\r\n\t\t\tcolor3 255 0 255 255\r\n",
                "\t\t\tbackcolor 255 220 220 220\r\n\t\t\tsortbartextcolor 255 0 0 0\r\n\t\t\tsortbackcolor 255 180 180 180\r\n",
                "\t\t}\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_gatechoose\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_TEXT\r\n\t\trect 0.000000 0.000000 1.000000 0.088889\r\n",
                "\t\ttextid gateselection_tittle\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_DONE\r\n\t\tbutton\r\n\t\t{\r\n",
                "\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n",
                "\t\ttextid Done\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP1\r\n\t\trect 0.006250 0.133333 0.143750 0.177778\r\n",
                "\t\tsprite dialog220x40.tga\r\n\t}\r\n",
                "\titem_pull\r\n\t{\r\n\t\tname ID_CURGATE\r\n\t\trect 0.012500 0.144444 0.137500 0.166667\r\n",
                "\t\ttextcolor 255 60 60 60\r\n\t\tbackcolor 255 220 220 220\r\n",
                "\t\tpullbackcolor 255 0 0 0\r\n\t}\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n\t\trect 0.006250 0.000000 0.206250 0.088889\r\n",
                "\t\tsprite logo_ui.tga\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_chatswitch\r\n\titem_button\r\n\t{\r\n\t\tname ID_CHAT\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.190625 0.966667 0.315625 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid chat_button\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n}\r\n"
            ),
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("multiclient.mnu")).unwrap();
        let pit = mnu
            .split("name idd_multi_pit\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(!pit.contains("backcolor 127 0 0 0"));
        assert!(!pit.contains("backcolor 240 170 180 190"));
        assert!(pit.contains("sprite setupbar.tga"));
        assert!(pit.contains(&format!(
            "rect 0.030000 {SETUP_CHROME_Y0:.6} 0.140000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(pit.contains("sprite1 back1.tga"));
        assert!(pit.contains(&format!(
            "rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(pit.contains("sprite optionstabtrack.tga"));
        assert!(pit.contains(&format!(
            "rect {PIT_GROUP_X0:.6} {SETUP_CHROME_Y0:.6} {PIT_GROUP_X1:.6} {SETUP_CHROME_Y1:.6}"
        )));
        let (sx0, sx1) = pit_group_slot(0);
        let (gx0, gx1) = pit_group_slot(2);
        assert!(pit.contains(&format!(
            "rect {sx0:.6} {SETUP_CHROME_Y0:.6} {sx1:.6} {SETUP_CHROME_Y1:.6}"
        )));
        assert!(pit.contains(&format!(
            "rect {gx0:.6} {SETUP_CHROME_Y0:.6} {gx1:.6} {SETUP_CHROME_Y1:.6}"
        )));
        assert!(pit.contains("sprite1 segl1.tga"));
        assert!(pit.contains("sprite1 segm1.tga"));
        let chat = mnu
            .split("name idd_chatswitch\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        let (cx0, cx1) = (CHAT_SLOT.0, CHAT_SLOT.1);
        assert!(chat.contains(&format!(
            "rect {cx0:.6} {SETUP_CHROME_Y0:.6} {cx1:.6} {SETUP_CHROME_Y1:.6}"
        )));
        assert!(chat.contains("sprite1 button1.tga"));
        assert!(!chat.contains("segm1.tga"));
        assert!(!chat.contains("0.190625 0.966667"));
        let entries = mnu
            .split("name idd_race_entries\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(entries.contains("backcolor 160 0 0 0"));
        assert!(entries.contains("sortbackcolor 160 0 0 0"));
        let info = mnu
            .split("name idd_race_info\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(info.contains("sprite raceinfoboard.tga"));
        assert!(!info.contains("dialog1380x670t.tga"));
        assert!(info.contains(&format!("0.993750 {SETUP_BOARD_Y1:.6}")));
        assert!(info.contains("rect 0.800000 0.850000 0.968750 0.885000"));
        assert!(!info.contains("0.862500 0.900000 0.987500 0.933333"));
        let admin = info.split("name ID_ADMIN\n").nth(1).unwrap();
        assert!(admin.contains("rect 0.650000 0.850000 0.785000 0.885000"));
        assert!(admin.contains("align center"));
        assert!(!admin.contains("0.137500 0.900000 0.262500 0.933333"));
        let results = mnu
            .split("name idd_race_results\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(results.contains("sprite raceinfoboard.tga"));
        assert!(results.contains("sprite fieldbox.tga"));
        assert!(!results.contains("backcolor 255 240 240 240"));
        assert!(results.contains("rect 0.800000 0.850000 0.968750 0.885000"));
        assert!(!results.contains("0.137500 0.900000 0.262500 0.933333"));
        let practice = mnu
            .split("name idd_race_practice\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(practice.contains("backcolor 160 0 0 0"));
        assert!(practice.contains("sortbackcolor 160 0 0 0"));
        assert!(practice.contains(&format!("sortbartextcolor {}", argb(255, TEXT))));
        assert!(!practice.contains("sortbackcolor 255 180 180 180"));
        let gate = mnu
            .split("name idd_gatechoose\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(!gate.contains("backcolor 127 0 0 0"));
        assert!(gate.contains("sprite fieldbox.tga"));
        assert!(!gate.contains("dialog220x40.tga"));
        assert!(!gate.contains(&format!("backcolor 255 {}", bgr(ROW))));
        assert!(!gate.contains("backcolor 255 220 220 220"));
        assert!(gate.contains(&format!(
            "rect 0.800000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(gate.contains("sprite1 done1.tga"));
        let done = gate.split("name ID_DONE\n").nth(1).unwrap();
        assert!(done.contains("align center"));
        assert!(ui.join("raceinfoboard.tga").is_file());
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("multiclient.mnu"));
        assert!(man.contains("raceinfoboard.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn splice_garage_clears_strips_glasses_boards_and_chrome() {
        let game = temp_game();
        let ui = game.join("ui");
        fs::write(
            ui.join("garage.mnu"),
            concat!(
                "dialog\r\n{\r\n\tname idd_garage\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX2\r\n\t\trect 0.000000 0.966667 1.000000 1.000000\r\n\t\tcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX\r\n\t\trect 0.006250 0.133333 0.993750 0.211111\r\n\t\tcolor 240 170 180 190\r\n\t}\r\n",
                "\titem_darkbox\r\n\t{\r\n\t\tname ID_DARKBOX3\r\n\t\trect 0.006250 0.222222 0.712500 0.266667\r\n\t\tcolor 240 170 180 190\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname id_done\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.875000 0.966667 1.000000 1.000000\r\n\t\t\tsprite2 done1.tga\r\n\t\t\tsprite3 done2.tga\r\n\t\t}\r\n",
                "\t\ttextid Done\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\talign right\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP1\r\n\t\trect 0.718750 0.222222 0.993750 0.411111\r\n\t\tsprite dialog440x170.tga\r\n\t}\r\n",
                "\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP2\r\n\t\trect 0.718750 0.422222 0.993750 0.944444\r\n\t\tsprite dialog440x470.tga\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_TEXT5\r\n\t\ttextid garage_title\r\n\t\tcolor 255 240 240 240\r\n\t\tbackcolor 127 0 0 0\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_TEST\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.718750 0.966667 0.843750 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid test_garage\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_button\r\n\t{\r\n\t\tname ID_INFO\r\n\t\tbutton\r\n\t\t{\r\n\t\t\trect 0.562500 0.966667 0.687500 1.000000\r\n\t\t\tsprite2 b_button2.tga\r\n\t\t\tsprite3 b_button3.tga\r\n\t\t}\r\n",
                "\t\ttextid info\r\n\t\ttext\r\n\t\t{\r\n\t\t\tpos 0.000000 0.005556\r\n\t\t\talign center\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 240 240 240\r\n\t\tcolor3 255 240 240 240\r\n\t}\r\n",
                "\titem_pull\r\n\t{\r\n\t\tname id_dryqual\r\n\t\trect 0.137500 0.144444 0.325000 0.166667\r\n",
                "\t\ttextcolor 255 60 60 60\r\n\t\tbackcolor 255 240 240 240\r\n",
                "\t\tpullbackcolor 255 0 0 0\r\n\t\tpulltextcolor1 255 255 255 255\r\n",
                "\t\tpulltextcolor2 255 40 60 240\r\n\t}\r\n",
                "\titem_tab\r\n\t{\r\n\t\tname id_general_tab\r\n\t\tgroup 0\r\n\t\trect 0.006250 0.911111 0.131250 0.944444\r\n",
                "\t\tsprite1 tabh1.tga\r\n\t\tsprite2 tabh2.tga\r\n\t\ttextid general\r\n",
                "\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t}\r\n",
                "\titem_tab\r\n\t{\r\n\t\tname id_suspensions_tab\r\n\t\tgroup 0\r\n\t\trect 0.131250 0.911111 0.256250 0.944444\r\n",
                "\t\tsprite1 tabh1.tga\r\n\t\tsprite2 tabh2.tga\r\n\t\ttextid suspensions\r\n",
                "\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t}\r\n",
                "\titem_tab\r\n\t{\r\n\t\tname id_drivetrain_tab\r\n\t\tgroup 0\r\n\t\trect 0.256250 0.911111 0.381250 0.944444\r\n",
                "\t\tsprite1 tabh1.tga\r\n\t\tsprite2 tabh2.tga\r\n\t\ttextid drivetrain\r\n",
                "\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t}\r\n",
                "\titem_tab\r\n\t{\r\n\t\tname ID_OTHERS_TAB\r\n\t\tgroup 0\r\n\t\trect 0.381250 0.911111 0.506250 0.944444\r\n",
                "\t\tsprite1 tabh1.tga\r\n\t\tsprite2 tabh2.tga\r\n\t\ttextid others_garage\r\n",
                "\t\ttext\r\n\t\t{\r\n\t\t\tfont main.fnt\r\n\t\t\tpos 0.006250 0.005556\r\n\t\t\tfontsize 0.022222\r\n\t\t\talign left\r\n\t\t}\r\n",
                "\t\tcolor1 255 240 240 240\r\n\t\tcolor2 255 0 0 0\r\n\t}\r\n}\r\n\r\n",
                "dialog\r\n{\r\n\tname idd_garage_general\r\n\titem_bitmap\r\n\t{\r\n\t\tname ID_BITMAP\r\n",
                "\t\trect 0.006250 0.277778 0.518750 0.911111\r\n\t\tsprite dialog820x570t.tga\r\n\t}\r\n",
                "\titem_text\r\n\t{\r\n\t\tname ID_FUEL\r\n\t\trect 0.012500 0.288889 0.512500 0.311111\r\n",
                "\t\ttextid fuel_garage\r\n\t\tcolor 255 40 40 40\r\n\t\tbackcolor 0 0 0 0\r\n\t}\r\n",
                "\titem_pull\r\n\t{\r\n\t\tname ID_FRONTCOMPOUND\r\n\t\trect 0.137500 0.400000 0.262500 0.422222\r\n",
                "\t\ttextcolor 255 60 60 60\r\n\t\tbackcolor 255 240 240 240\r\n",
                "\t\tpullbackcolor 255 0 0 0\r\n\t}\r\n}\r\n"
            ),
        )
        .unwrap();
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let mnu = fs::read_to_string(ui.join("garage.mnu")).unwrap();
        let garage = mnu
            .split("name idd_garage\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(!garage.contains("color 240 170 180 190"));
        assert!(!garage.contains("backcolor 127 0 0 0"));
        assert!(
            !garage.contains("backcolor 255 240 240 240"),
            "setup-select pulls must not keep cream fills"
        );
        assert!(
            garage.contains(&format!("pullbackcolor 255 {}", bgr(NIGHT))),
            "open pull lists need an opaque night panel"
        );
        assert!(!garage.contains(&format!("pullbackcolor 160 {}", bgr(NIGHT))));
        assert!(!garage.contains("pullbackcolor 0 0 0 0"));
        assert!(garage.contains("sprite setupbar.tga"));
        assert!(garage.contains("name ID_SETUP_BAR\n"));
        assert!(garage.contains("name ID_COMPARE_BAR\n"));
        assert!(garage.contains("sprite fieldbox.tga"));
        assert!(garage.contains("sprite garagepanel_t.tga"));
        assert!(garage.contains("sprite garagepanel_r.tga"));
        assert!(!garage.contains("dialog440x170.tga"));
        assert!(!garage.contains("dialog440x470.tga"));
        assert!(garage.contains("sprite optionstabtrack.tga"));
        assert!(garage.contains("sprite1 segl1.tga"));
        assert!(garage.contains("sprite1 segm1.tga"));
        assert!(garage.contains("sprite1 segr1.tga"));
        assert!(garage.contains("name ID_SETUPTABS\n"));
        assert!(garage.contains("name ID_GARAGE_ACT\n"));
        assert!(!garage.contains("sprite1 tabh1.tga"));
        assert!(garage.contains(&format!(
            "rect {:.6} {SETUP_CHROME_Y0:.6} {:.6} {SETUP_CHROME_Y1:.6}",
            GARAGE_ACT_X0, CHAT_SLOT.0
        )));
        assert!(garage.contains(&format!(
            "rect {:.6} {SETUP_CHROME_Y0:.6} {:.6} {SETUP_CHROME_Y1:.6}",
            CHAT_SLOT.1, GARAGE_ACT_X1
        )));
        assert!(garage.contains(&format!(
            "rect 0.820000 {SETUP_CHROME_Y0:.6} 0.970000 {SETUP_CHROME_Y1:.6}"
        )));
        assert!(garage.contains(&format!(
            "rect 0.718750 0.422222 0.993750 {SETUP_BOARD_Y1:.6}"
        )));
        assert!(!garage.contains("0.993750 0.944444"));
        assert!(garage.contains("sprite1 done1.tga"));
        let general = mnu
            .split("name idd_garage_general\n")
            .nth(1)
            .unwrap()
            .split("\ndialog\n")
            .next()
            .unwrap();
        assert!(general.contains("sprite garageboard.tga"));
        assert!(!general.contains("dialog820x570t.tga"));
        assert!(general.contains(&format!(
            "rect 0.006250 0.311111 0.518750 {GARAGE_PANE_Y1:.6}"
        )));
        assert!(!general.contains("rect 0.006250 0.311111 0.518750 0.911111"));
        assert!(!general.contains("rect 0.006250 0.277778 0.518750 0.911111"));
        assert!(
            !general.contains("backcolor 255 240 240 240"),
            "tyre/brake pulls must not stay cream/white"
        );
        assert!(
            general.contains("sprite fieldbox.tga"),
            "pulls need grey fieldbox pills"
        );
        assert!(general.contains("name ID_BG_ID_FRONTCOMPOUND\n"));
        // Category tabs above chrome, left of Info|Chat|Test.
        assert!(GARAGE_TAB_Y1 < SETUP_CHROME_Y0);
        assert!(GARAGE_TAB_X1 <= GARAGE_ACT_X0);
        assert!(GARAGE_PANE_Y1 <= GARAGE_TAB_Y0);
        assert!(garage.contains(&format!(
            "rect {GARAGE_TAB_X0:.6} {GARAGE_TAB_Y0:.6} {GARAGE_TAB_X1:.6} {GARAGE_TAB_Y1:.6}"
        )));
        assert!(!garage.contains("rect 0.006250 0.911111 0.506250 0.944444"));
        assert!(ui.join("garageboard.tga").is_file());
        assert!(ui.join("garagepanel_t.tga").is_file());
        assert!(ui.join("garagepanel_r.tga").is_file());
        assert!(ui.join("fieldbox.tga").is_file());
        let man = fs::read_to_string(ui.join(MANIFEST)).unwrap();
        assert!(man.contains("garage.mnu"));
        assert!(man.contains("garageboard.tga"));
        let _ = fs::remove_dir_all(&game);
    }

    /// Live install apply: `MXBO_GAME_UI_APPLY` = MX Bikes game root (has ui.holeshot-bak or ui/).
    #[test]
    fn apply_live_game_when_env_set() {
        let Ok(root) = std::env::var("MXBO_GAME_UI_APPLY") else {
            return;
        };
        let game = PathBuf::from(root);
        assert!(game.join("mxbikes.exe").is_file(), "not an MX Bikes folder");
        apply(&game, DEFAULT_PRIMARY).unwrap();
        let ui = game.join("ui");
        assert!(ui.join("options.mnu").is_file());
        assert!(ui.join("optionsboard.tga").is_file());
        let opt = fs::read_to_string(ui.join("options.mnu")).unwrap();
        assert!(opt.contains("sprite optionsboard.tga"));
        assert!(opt.contains("name id_gfx_tab"));
        assert!(opt.contains("sprite1 done1.tga"));
    }
}

