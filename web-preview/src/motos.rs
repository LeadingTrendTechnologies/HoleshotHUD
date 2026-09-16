use mxbo_hud::render::Fonts;
use mxbo_motos_ui::{
    draw_pending_drop_menus, paint_analyze_open, AnalyzePaintState, Drop, Hit, HitBox,
};
use mxbo_review::{demo_session, SessionDetail};
use tiny_skia::{Color, Pixmap};

pub struct MotosDemo {
    pub detail: SessionDetail,
    pub compare: i32,
    pub you_lap: i32,
    pub warmup: bool,
    pub scrub: f32,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_z: f32,
    pub follow: bool,
    pub open_drop: Option<Drop>,
    pub hover: Option<Hit>,
    hits: Vec<HitBox>,
    drag_scrub: bool,
    drag_map: bool,
    last_x: f32,
    last_y: f32,
}

impl MotosDemo {
    pub fn new() -> Self {
        let detail = demo_session();
        Self {
            compare: 0,
            you_lap: -1,
            warmup: false,
            scrub: 0.38,
            zoom: 1.0,
            pan_x: 0.0,
            pan_z: 0.0,
            follow: false,
            open_drop: None,
            hover: None,
            hits: Vec::new(),
            drag_scrub: false,
            drag_map: false,
            last_x: 0.0,
            last_y: 0.0,
            detail,
        }
    }

    pub fn paint(&mut self, px: &mut Pixmap, fonts: &Fonts, w: f32, h: f32) {
        px.fill(Color::from_rgba8(18, 18, 20, 255));
        self.hits.clear();
        let state = AnalyzePaintState {
            compare: self.compare,
            you_lap: self.you_lap,
            warmup: self.warmup,
            scrub: self.scrub,
            zoom: self.zoom,
            pan_x: self.pan_x,
            pan_z: self.pan_z,
            follow: self.follow,
            open_drop: self.open_drop,
            live: false,
        };
        let mut ah = Vec::new();
        paint_analyze_open(px, fonts, w, h, &self.detail, &state, self.hover, &mut ah);
        self.hits.extend(ah);
        draw_pending_drop_menus(px, fonts);
        if self.follow && self.zoom > 1.0 {
            if let Some((px, pz)) = mxbo_motos_ui::follow_pan_for(
                self.detail.row.id,
                self.you_lap,
                self.warmup,
                self.scrub,
            ) {
                self.pan_x = px;
                self.pan_z = pz;
            }
        }
    }

    pub fn pointer_move(&mut self, x: f32, y: f32, w: f32, _h: f32) {
        self.hover = self.hit_at(x, y).map(|b| b.id);
        if self.drag_scrub {
            if let Some(b) = self.hits.iter().find(|b| matches!(b.id, Hit::AnalyzeScrub)) {
                self.scrub = ((x - b.x) / b.w.max(1.0)).clamp(0.0, 1.0);
            }
        }
        if self.drag_map {
            let dx = x - self.last_x;
            let dy = y - self.last_y;
            self.pan_x -= dx * 0.8 / self.zoom.max(1.0);
            self.pan_z += dy * 0.8 / self.zoom.max(1.0);
        }
        self.last_x = x;
        self.last_y = y;
    }

    pub fn pointer_down(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.last_x = x;
        self.last_y = y;
        let Some(hit) = self.hit_at(x, y).map(|b| b.id) else {
            return;
        };
        match hit {
            Hit::AnalyzeScrub => self.drag_scrub = true,
            Hit::AnalyzeMap => self.drag_map = true,
            Hit::AnalyzeFollow => self.follow = !self.follow,
            Hit::AnalyzeCompareOpen => {
                self.open_drop = if self.open_drop == Some(Drop::AnalyzeCompare) {
                    None
                } else {
                    Some(Drop::AnalyzeCompare)
                };
            }
            Hit::AnalyzeLapOpen => {
                self.open_drop = if self.open_drop == Some(Drop::AnalyzeLap) {
                    None
                } else {
                    Some(Drop::AnalyzeLap)
                };
            }
            Hit::AnalyzeCompare(n) => {
                self.compare = n;
                self.open_drop = None;
            }
            Hit::AnalyzeYouLap(n) => {
                self.you_lap = n;
                self.warmup =
                    self.detail.laps.iter().any(|l| {
                        l.lap_num == n && l.race_num == self.detail.your_race_num && l.warmup
                    });
                self.open_drop = None;
            }
            _ => {}
        }
    }

    pub fn pointer_up(&mut self) {
        self.drag_scrub = false;
        self.drag_map = false;
    }

    pub fn wheel(&mut self, x: f32, y: f32, delta: f32) {
        if self
            .hit_at(x, y)
            .is_some_and(|b| matches!(b.id, Hit::AnalyzeMap))
        {
            let z = self.zoom * if delta < 0.0 { 1.12 } else { 0.89 };
            self.zoom = z.clamp(1.0, 12.0);
        }
    }

    fn hit_at<'a>(&'a self, x: f32, y: f32) -> Option<&'a HitBox> {
        self.hits
            .iter()
            .rev()
            .find(|b| x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h)
    }
}
