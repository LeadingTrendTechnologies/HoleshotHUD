//! Groups tab. Split list and editor. Membership is the rider's display name.

use std::sync::Mutex;

use tiny_skia::Color;

use mxbo_hud::config::{
    clamp_group_name, group_badge, rider_key, GROUP_COLORS, GROUP_ICONS,
};

use super::*;

static SESSION_RIDERS: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[derive(Clone, Copy, PartialEq, Eq)]
enum GroupField {
    Name,
    Member,
}

struct GroupEdit {
    selected: usize,
    name: String,
    member: String,
    focus: Option<GroupField>,
    checks: Vec<String>,
}

thread_local! {
    static GROUP_EDIT: std::cell::RefCell<GroupEdit> = std::cell::RefCell::new(GroupEdit {
        selected: 0,
        name: String::new(),
        member: String::new(),
        focus: None,
        checks: Vec::new(),
    });
}

pub fn set_session_riders(snap: Option<&crate::shm::Snapshot>) {
    let names = snap.map(session_names).unwrap_or_default();
    if let Ok(mut slot) = SESSION_RIDERS.lock() {
        *slot = names;
    }
}

fn session_names(snap: &crate::shm::Snapshot) -> Vec<String> {
    let mut names = Vec::new();
    let riders = snap
        .rider_count
        .clamp(0, crate::shm::MAX_RIDERS as i32) as usize;
    for rider in snap.riders.iter().take(riders) {
        push_session_name(&mut names, &crate::shm::bytes_as_text(&rider.name));
    }
    let standings = snap
        .standing_count
        .clamp(0, crate::shm::MAX_STANDINGS as i32) as usize;
    for row in snap.standings.iter().take(standings) {
        push_session_name(&mut names, &crate::shm::bytes_as_text(&row.name));
    }
    names
}

fn push_session_name(names: &mut Vec<String>, raw: &str) {
    let name = clamp_group_name(raw);
    if name.is_empty() {
        return;
    }
    if names.iter().any(|have| rider_key(have) == rider_key(&name)) {
        return;
    }
    names.push(name);
}

pub(crate) fn session_riders() -> Vec<String> {
    SESSION_RIDERS.lock().map(|names| names.clone()).unwrap_or_default()
}

fn edit_selected() -> usize {
    GROUP_EDIT.with(|edit| edit.borrow().selected)
}

fn set_edit_selected(index: usize) {
    GROUP_EDIT.with(|edit| edit.borrow_mut().selected = index);
}

pub(crate) fn group_name_focused() -> bool {
    GROUP_EDIT.with(|edit| edit.borrow().focus == Some(GroupField::Name))
}

pub(crate) fn group_member_focused() -> bool {
    GROUP_EDIT.with(|edit| edit.borrow().focus == Some(GroupField::Member))
}

fn set_group_focus(focus: Option<GroupField>) {
    GROUP_EDIT.with(|edit| edit.borrow_mut().focus = focus);
}

pub(crate) fn focus_group_name() {
    let selected = edit_selected();
    let saved = with_config(|cfg| {
        cfg.groups
            .get(selected)
            .map(|group| group.name.clone())
            .unwrap_or_default()
    });
    GROUP_EDIT.with(|edit| {
        let mut edit = edit.borrow_mut();
        if edit.focus != Some(GroupField::Name) {
            edit.name = saved;
        }
        edit.focus = Some(GroupField::Name);
    });
    crate::feedback::set_focus(false);
}

fn focus_group_member() {
    GROUP_EDIT.with(|edit| edit.borrow_mut().focus = Some(GroupField::Member));
    crate::feedback::set_focus(false);
}

pub(crate) fn commit_group_name() {
    if !group_name_focused() {
        return;
    }
    let (selected, name) = GROUP_EDIT.with(|edit| {
        let edit = edit.borrow();
        (edit.selected, edit.name.clone())
    });
    update_config(|cfg| cfg.rename_group(selected, &name));
}

pub(crate) fn group_push(ch: char) {
    if ch.is_control() || ch == '\t' {
        return;
    }
    GROUP_EDIT.with(|edit| {
        let mut edit = edit.borrow_mut();
        let field = match edit.focus {
            Some(GroupField::Name) => &mut edit.name,
            Some(GroupField::Member) => &mut edit.member,
            None => return,
        };
        if field.chars().count() >= mxbo_hud::config::GROUP_NAME_MAX {
            return;
        }
        field.push(ch);
    });
}

pub(crate) fn group_backspace() {
    GROUP_EDIT.with(|edit| {
        let mut edit = edit.borrow_mut();
        match edit.focus {
            Some(GroupField::Name) => {
                edit.name.pop();
            }
            Some(GroupField::Member) => {
                edit.member.pop();
            }
            None => {}
        }
    });
}

pub(crate) fn group_revert_focus() {
    GROUP_EDIT.with(|edit| {
        let mut edit = edit.borrow_mut();
        match edit.focus {
            Some(GroupField::Name) => edit.name.clear(),
            Some(GroupField::Member) => edit.member.clear(),
            None => {}
        }
        edit.focus = None;
    });
}

fn add_typed_member() {
    let (selected, name) = GROUP_EDIT.with(|edit| {
        let mut edit = edit.borrow_mut();
        let name = edit.member.clone();
        edit.member.clear();
        (edit.selected, name)
    });
    update_config(|cfg| cfg.add_group_members(selected, &[name]));
}

fn toggle_check(index: u8) {
    let Some(name) = session_riders().get(index as usize).cloned() else {
        return;
    };
    let selected = edit_selected();
    let already = with_config(|cfg| {
        cfg.groups.get(selected).is_some_and(|group| {
            group
                .members
                .iter()
                .any(|member| rider_key(member) == rider_key(&name))
        })
    });
    if already {
        return;
    }
    GROUP_EDIT.with(|edit| {
        let mut edit = edit.borrow_mut();
        if let Some(pos) = edit
            .checks
            .iter()
            .position(|have| rider_key(have) == rider_key(&name))
        {
            edit.checks.remove(pos);
        } else {
            edit.checks.push(name);
        }
    });
}

fn add_checked() {
    let (selected, checks) = GROUP_EDIT.with(|edit| {
        let mut edit = edit.borrow_mut();
        let checks = std::mem::take(&mut edit.checks);
        (edit.selected, checks)
    });
    update_config(|cfg| cfg.add_group_members(selected, &checks));
}

/// Clicks that belong to the Groups page. `true` means dispatch should stop.
pub(crate) fn dispatch_group(id: Hit) -> bool {
    match id {
        Hit::GroupSelect(index) => {
            set_edit_selected(index as usize);
            set_group_focus(None);
            true
        }
        Hit::GroupUp(index) => {
            update_config(|cfg| {
                let next = cfg.move_group(index as usize, -1);
                set_edit_selected(next);
            });
            true
        }
        Hit::GroupDown(index) => {
            update_config(|cfg| {
                let next = cfg.move_group(index as usize, 1);
                set_edit_selected(next);
            });
            true
        }
        Hit::GroupNew => {
            let next = with_config(|cfg| cfg.groups.len());
            update_config(|cfg| {
                if let Some(index) = cfg.add_group("Group") {
                    set_edit_selected(index);
                }
            });
            if with_config(|cfg| cfg.groups.len()) > next {
                focus_group_name();
            }
            true
        }
        Hit::GroupDelete => {
            let selected = edit_selected();
            update_config(|cfg| cfg.remove_group(selected));
            let len = with_config(|cfg| cfg.groups.len());
            set_edit_selected(selected.min(len.saturating_sub(1)));
            set_group_focus(None);
            true
        }
        Hit::GroupIcon(index) => {
            let Some(icon) = GROUP_ICONS.get(index as usize).copied() else {
                return true;
            };
            let selected = edit_selected();
            update_config(|cfg| cfg.set_group_icon(selected, icon));
            true
        }
        Hit::GroupColor(index) => {
            let Some(color) = GROUP_COLORS.get(index as usize).copied() else {
                return true;
            };
            let selected = edit_selected();
            update_config(|cfg| cfg.set_group_color(selected, color));
            true
        }
        Hit::GroupMemberRemove(index) => {
            let selected = edit_selected();
            update_config(|cfg| cfg.remove_group_member(selected, index as usize));
            true
        }
        Hit::GroupCheck(index) => {
            toggle_check(index);
            true
        }
        Hit::GroupAddSelected => {
            add_checked();
            true
        }
        Hit::GroupName => {
            focus_group_name();
            true
        }
        Hit::GroupMember => {
            focus_group_member();
            true
        }
        Hit::GroupAddName => {
            add_typed_member();
            set_group_focus(None);
            true
        }
        Hit::GroupsOnMaps => {
            update_config(|cfg| cfg.groups_on_maps = !cfg.groups_on_maps);
            true
        }
        _ => false,
    }
}

pub(crate) fn pane_groups(
    px: &mut tiny_skia::Pixmap,
    fonts: &Fonts,
    cfg: &HudConfig,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    view_bottom: f32,
) -> f32 {
    let y = heading(
        px,
        fonts,
        x,
        y,
        w,
        "Groups",
        "A mark beside their name",
        None,
        hover,
        hits,
    );
    let y = toggle_row(
        px,
        fonts,
        x,
        y,
        w,
        "Show on maps",
        cfg.groups_on_maps,
        Hit::GroupsOnMaps,
        hover,
        hits,
    );
    let note = "Other riders in a group use that group's color. Your dot stays orange.";
    let mut y = y;
    for line in wrap_fb(fonts, note, w, 12.0) {
        text(px, fonts, &line, 12.0, x, y, dim(), false);
        y += 18.0;
    }
    let y = y + 8.0;
    let groups = cfg.groups.as_slice();
    if groups.is_empty() {
        return paint_groups_empty(px, fonts, hover, hits, x, y, w, view_bottom);
    }
    let selected = edit_selected().min(groups.len().saturating_sub(1));
    let left_w = (w * 0.36).clamp(200.0, 280.0);
    let gap = 12.0;
    let right_x = x + left_w + gap;
    let right_w = (w - left_w - gap).max(220.0);
    let left_bottom = paint_group_list(px, fonts, hover, hits, x, y, left_w, &groups, selected);
    let right_bottom = paint_group_editor(
        px, fonts, hover, hits, right_x, y, right_w, &groups, selected,
    );
    left_bottom.max(right_bottom)
}

fn paint_groups_empty(
    px: &mut tiny_skia::Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    view_bottom: f32,
) -> f32 {
    let title = "New group to mark riders";
    let note = "A rider stays in a group by the name shown in the race. If that name changes, the mark will not show.";
    let note_w = (w * 0.72).clamp(240.0, 420.0);
    let lines = wrap_fb(fonts, note, note_w, 12.0);
    let title_h = 22.0;
    let gap = 16.0;
    let btn_h = 32.0;
    let line_h = 18.0;
    let stack_h = title_h + gap + btn_h + gap + lines.len() as f32 * line_h;
    let room = (view_bottom - y).max(stack_h);
    let mut cy = y + ((room - stack_h) * 0.5).max(0.0);
    text(
        px,
        fonts,
        title,
        13.0,
        x + w * 0.5,
        cy,
        dim(),
        true,
    );
    cy += title_h + gap;
    let label = "New group";
    let btn_w = (measure(fonts, label, 13.0) + 36.0).max(128.0);
    action_btn(
        px,
        fonts,
        x + (w - btn_w) * 0.5,
        cy,
        btn_w,
        btn_h,
        label,
        Hit::GroupNew,
        hover,
        hits,
        false,
    );
    cy += btn_h + gap;
    for line in &lines {
        text(px, fonts, line, 12.0, x + w * 0.5, cy, dim(), true);
        cy += line_h;
    }
    cy
}

fn paint_group_list(
    px: &mut tiny_skia::Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    groups: &[mxbo_hud::config::RiderGroup],
    selected: usize,
) -> f32 {
    let row_h = 44.0;
    let card_h = 12.0 + groups.len() as f32 * row_h + 52.0;
    outlined(px, x, y, w, card_h, 10.0, panel());
    let mut row_y = y + 8.0;
    for (index, group) in groups.iter().enumerate() {
        let hit = Hit::GroupSelect(index as u8);
        hits.push(HitBox {
            id: hit,
            x: x + 6.0,
            y: row_y,
            w: w - 12.0,
            h: row_h - 4.0,
        });
        if index == selected {
            fill_round(px, x + 6.0, row_y, w - 12.0, row_h - 4.0, 8.0, tab_on());
        } else if hover == Some(hit) {
            fill_round(px, x + 6.0, row_y, w - 12.0, row_h - 4.0, 8.0, hover_wash());
        }
        let rgb = group.color;
        icon(
            px,
            fonts,
            group.icon,
            14.0,
            x + 16.0,
            row_y + 10.0,
            Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255),
            false,
        );
        let name = ellipsize_heading(fonts, &group.name, 13.0, w - 120.0);
        text(px, fonts, &name, 13.0, x + 38.0, row_y + 6.0, text_col(), false);
        let count = if group.members.len() == 1 {
            mxbo_hud::i18n::t("1 rider").to_string()
        } else {
            mxbo_hud::i18n::t_fmt("{n} riders", &[("n", &group.members.len().to_string())])
        };
        text(px, fonts, &count, 11.0, x + 38.0, row_y + 22.0, dim(), false);
        if groups.len() > 1 {
            btn_icon(
                px,
                fonts,
                x + w - 62.0,
                row_y + 6.0,
                22.0,
                22.0,
                '\u{f077}',
                Hit::GroupUp(index as u8),
                hover,
                hits,
            );
            btn_icon(
                px,
                fonts,
                x + w - 36.0,
                row_y + 6.0,
                22.0,
                22.0,
                '\u{f078}',
                Hit::GroupDown(index as u8),
                hover,
                hits,
            );
        }
        row_y += row_h;
    }
    action_btn(
        px,
        fonts,
        x + 10.0,
        y + card_h - 44.0,
        w - 20.0,
        32.0,
        "New group",
        Hit::GroupNew,
        hover,
        hits,
        false,
    );
    y + card_h
}

fn paint_group_editor(
    px: &mut tiny_skia::Pixmap,
    fonts: &Fonts,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
    x: f32,
    y: f32,
    w: f32,
    groups: &[mxbo_hud::config::RiderGroup],
    selected: usize,
) -> f32 {
    let Some(group) = groups.get(selected) else {
        text(
            px,
            fonts,
            "New group to mark riders",
            13.0,
            x,
            y + 8.0,
            dim(),
            false,
        );
        return y + 32.0;
    };
    let (name_draft, member_draft, name_on, member_on, checks) = GROUP_EDIT.with(|edit| {
        let edit = edit.borrow();
        (
            edit.name.clone(),
            edit.member.clone(),
            edit.focus == Some(GroupField::Name),
            edit.focus == Some(GroupField::Member),
            edit.checks.clone(),
        )
    });
    let shown_name = if name_on { name_draft } else { group.name.clone() };
    icon(
        px,
        fonts,
        group.icon,
        18.0,
        x,
        y + 2.0,
        Color::from_rgba8(group.color[0], group.color[1], group.color[2], 255),
        false,
    );
    text(px, fonts, &group.name, 18.0, x + 28.0, y, text_col(), false);
    btn_icon(
        px,
        fonts,
        x + w - 28.0,
        y,
        28.0,
        28.0,
        '\u{f2ed}',
        Hit::GroupDelete,
        hover,
        hits,
    );
    let mut cy = y + 40.0;
    text(px, fonts, mxbo_hud::i18n::t("NAME"), 10.0, x, cy, dim(), false);
    cy += 16.0;
    paint_field(
        px,
        fonts,
        x,
        cy,
        w,
        &shown_name,
        "Group name",
        name_on,
        Hit::GroupName,
        hover,
        hits,
    );
    cy += 40.0;
    text(px, fonts, mxbo_hud::i18n::t("ICON"), 10.0, x, cy, dim(), false);
    cy += 16.0;
    let icon_count = GROUP_ICONS.len() as f32;
    let icon_step = (w / icon_count).clamp(28.0, 36.0);
    let icon_btn = icon_step - 4.0;
    let mut ix = x;
    for (index, mark) in GROUP_ICONS.iter().enumerate() {
        let hit = Hit::GroupIcon(index as u8);
        let on = group.icon == *mark;
        hits.push(HitBox {
            id: hit,
            x: ix,
            y: cy,
            w: icon_btn,
            h: icon_btn,
        });
        let fill = if on {
            tab_on()
        } else if hover == Some(hit) {
            hover_wash()
        } else {
            panel()
        };
        outlined(px, ix, cy, icon_btn, icon_btn, 7.0, fill);
        if on {
            fill_round(px, ix, cy, icon_btn, 2.0, 1.0, accent());
        }
        icon(
            px,
            fonts,
            *mark,
            14.0,
            ix + icon_btn * 0.5,
            cy + (icon_btn - 16.0) * 0.5,
            text_col(),
            true,
        );
        ix += icon_step;
    }
    cy += 44.0;
    text(px, fonts, mxbo_hud::i18n::t("COLOR"), 10.0, x, cy, dim(), false);
    cy += 18.0;
    let mut sx = x + 12.0;
    for (index, rgb) in GROUP_COLORS.iter().enumerate() {
        let hit = Hit::GroupColor(index as u8);
        hits.push(HitBox {
            id: hit,
            x: sx - 12.0,
            y: cy - 4.0,
            w: 28.0,
            h: 28.0,
        });
        if group.color == *rgb {
            fill_circle(px, sx, cy + 8.0, 12.0, accent());
        }
        fill_circle(
            px,
            sx,
            cy + 8.0,
            9.0,
            Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255),
        );
        sx += 28.0;
    }
    cy += 32.0;
    text(px, fonts, mxbo_hud::i18n::t("RIDERS"), 10.0, x, cy, dim(), false);
    cy += 16.0;
    if group.members.is_empty() {
        text(px, fonts, mxbo_hud::i18n::t("No riders yet"), 12.0, x, cy, dim(), false);
        cy += 22.0;
    }
    for (index, member) in group.members.iter().enumerate().take(64) {
        outlined(px, x, cy, w, 32.0, 7.0, panel());
        icon(
            px,
            fonts,
            group.icon,
            12.0,
            x + 10.0,
            cy + 8.0,
            Color::from_rgba8(group.color[0], group.color[1], group.color[2], 255),
            false,
        );
        let label = ellipsize_heading(fonts, member, 12.0, w - 64.0);
        text(px, fonts, &label, 12.0, x + 30.0, cy + 8.0, text_col(), false);
        btn_icon(
            px,
            fonts,
            x + w - 30.0,
            cy + 4.0,
            24.0,
            24.0,
            '\u{f00d}',
            Hit::GroupMemberRemove(index as u8),
            hover,
            hits,
        );
        cy += 36.0;
    }
    let session = session_riders();
    if !session.is_empty() {
        cy += 4.0;
        text(px, fonts, mxbo_hud::i18n::t("ON THIS SERVER"), 10.0, x, cy, dim(), false);
        cy += 16.0;
        for (index, name) in session.iter().enumerate().take(64) {
            let in_group = group
                .members
                .iter()
                .any(|member| rider_key(member) == rider_key(name));
            let staged = checks.iter().any(|have| rider_key(have) == rider_key(name));
            let on = in_group || staged;
            let hit = Hit::GroupCheck(index as u8);
            hits.push(HitBox {
                id: hit,
                x,
                y: cy,
                w,
                h: 28.0,
            });
            if hover == Some(hit) {
                fill_round(px, x, cy, w, 28.0, 6.0, hover_wash());
            }
            paint_check(px, fonts, x + 4.0, cy + 6.0, on);
            text(px, fonts, name, 12.0, x + 28.0, cy + 6.0, text_col(), false);
            if let Some((mark, rgb)) = group_badge(groups, name) {
                icon(
                    px,
                    fonts,
                    mark,
                    12.0,
                    x + w - 22.0,
                    cy + 6.0,
                    Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255),
                    false,
                );
            }
            cy += 30.0;
        }
        action_btn(
            px,
            fonts,
            x,
            cy,
            140.0,
            32.0,
            "Add selected",
            Hit::GroupAddSelected,
            hover,
            hits,
            true,
        );
        cy += 40.0;
    }
    paint_field(
        px,
        fonts,
        x,
        cy,
        (w - 88.0).max(120.0),
        &member_draft,
        "Type a name",
        member_on,
        Hit::GroupMember,
        hover,
        hits,
    );
    action_btn(
        px,
        fonts,
        x + w - 80.0,
        cy,
        80.0,
        32.0,
        "Add",
        Hit::GroupAddName,
        hover,
        hits,
        false,
    );
    cy += 44.0;
    let note = "A rider stays in a group by the name shown in the race. If that name changes, the mark will not show.";
    for line in wrap_fb(fonts, note, w, 12.0) {
        text(px, fonts, &line, 12.0, x, cy, dim(), false);
        cy += 18.0;
    }
    cy
}

fn paint_field(
    px: &mut tiny_skia::Pixmap,
    fonts: &Fonts,
    x: f32,
    y: f32,
    w: f32,
    value: &str,
    placeholder: &str,
    focused: bool,
    hit: Hit,
    hover: Option<Hit>,
    hits: &mut Vec<HitBox>,
) {
    hits.push(HitBox {
        id: hit,
        x,
        y,
        w,
        h: 32.0,
    });
    let hot = focused || hover == Some(hit);
    outlined(
        px,
        x,
        y,
        w,
        32.0,
        7.0,
        if hot { chip_hover() } else { bg() },
    );
    let placeholder = mxbo_hud::i18n::t(placeholder);
    let shown = if value.is_empty() { placeholder } else { value };
    let ink = if value.is_empty() { muted() } else { text_col() };
    let label = ellipsize_heading(fonts, shown, 12.0, w - 20.0);
    text(px, fonts, &label, 12.0, x + 10.0, y + 8.0, ink, false);
    if focused {
        let caret_x = x + 10.0 + measure(fonts, &label, 12.0) + 2.0;
        fill_round(
            px,
            caret_x.min(x + w - 8.0),
            y + 8.0,
            1.5,
            14.0,
            0.0,
            accent(),
        );
    }
}

fn paint_check(px: &mut tiny_skia::Pixmap, fonts: &Fonts, x: f32, y: f32, on: bool) {
    if on {
        fill_round(px, x, y, 14.0, 14.0, 3.0, accent());
        icon(px, fonts, '\u{f00c}', 9.0, x + 7.0, y + 2.0, ink(), true);
    } else {
        outlined(px, x, y, 14.0, 14.0, 3.0, bg());
    }
}
