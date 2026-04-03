// app.rs — Main application state and UI
// Enterprise CPM Scheduler — egui 0.24

use egui::{Context, Ui, Color32, Stroke, RichText, Vec2, Pos2, Rect, FontId, Align};
use crate::activity::{Activity, Predecessor, RelType};
use crate::project::Project;
use crate::scheduler::run_cpm;

// ─── Palette ─────────────────────────────────────────────────────────────────
const C_BG: Color32        = Color32::from_rgb(13, 17, 23);
const C_SURFACE: Color32   = Color32::from_rgb(22, 27, 34);
const C_SURFACE2: Color32  = Color32::from_rgb(30, 38, 48);
const C_BORDER: Color32    = Color32::from_rgb(48, 58, 72);
const C_TEXT: Color32      = Color32::from_rgb(230, 237, 243);
const C_TEXT_DIM: Color32  = Color32::from_rgb(125, 140, 158);
const C_ACCENT: Color32    = Color32::from_rgb(88, 166, 255);
const C_CRITICAL: Color32  = Color32::from_rgb(248, 81, 73);
const C_FLOAT: Color32     = Color32::from_rgb(63, 185, 80);
const C_WARN: Color32      = Color32::from_rgb(210, 153, 34);
const C_ROW_ALT: Color32   = Color32::from_rgb(18, 23, 30);
const C_ROW_CRIT: Color32  = Color32::from_rgb(40, 20, 20);
const C_GANTT_GRID: Color32= Color32::from_rgb(33, 43, 55);

#[derive(PartialEq, Clone, Copy)]
enum Tab { Schedule, Gantt, Network }

#[derive(Default)]
struct EditState {
    open: bool,
    is_new: bool,
    id: String,
    name: String,
    duration: String,
    resource: String,
    predecessors: String,
    error: Option<String>,
}

impl EditState {
    fn from_activity(act: &Activity) -> Self {
        EditState {
            open: true,
            is_new: false,
            id: act.id.clone(),
            name: act.name.clone(),
            duration: act.duration.to_string(),
            resource: act.resource.clone(),
            predecessors: act.predecessors.iter().map(|p| p.activity_id.clone()).collect::<Vec<_>>().join(", "),
            error: None,
        }
    }
    fn new_empty() -> Self {
        EditState {
            open: true,
            is_new: true,
            id: Activity::new_uid(),
            name: String::new(),
            duration: "1".to_string(),
            ..Default::default()
        }
    }
}

pub struct CpmApp {
    project: Project,
    tab: Tab,
    edit: EditState,
    settings_open: bool,
    settings_name: String,
    settings_date: String,
    sched_error: Option<String>,
    status: String,
    confirm_delete: Option<String>,
}

impl CpmApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        CpmApp {
            project: Project::default(),
            tab: Tab::Schedule,
            edit: EditState::default(),
            settings_open: false,
            settings_name: String::new(),
            settings_date: String::new(),
            sched_error: None,
            status: "Welcome. Load a sample project or add activities to begin.".into(),
            confirm_delete: None,
        }
    }

    fn run_schedule(&mut self) {
        self.sched_error = None;
        match run_cpm(&self.project.activities) {
            Ok(result) => {
                let dur = result.project_duration;
                let n_cp = result.critical_path.len();
                self.project.activities = result.activities;
                self.project.is_scheduled = true;
                self.status = format!(
                    "✓  Scheduled {} activities  |  Project duration: {:.0} days  |  Critical path: {} activities",
                    self.project.activities.len(), dur, n_cp
                );
            }
            Err(e) => {
                self.project.is_scheduled = false;
                self.sched_error = Some(e.to_string());
                self.status = format!("Schedule error: {}", e);
            }
        }
    }

    fn apply_edit(&mut self) {
        let duration = match self.edit.duration.trim().parse::<f64>() {
            Ok(d) if d > 0.0 => d,
            _ => { self.edit.error = Some("Duration must be a positive number.".into()); return; }
        };
        if self.edit.name.trim().is_empty() {
            self.edit.error = Some("Name cannot be empty.".into()); return;
        }
        if self.edit.is_new && self.project.activities.iter().any(|a| a.id == self.edit.id) {
            self.edit.error = Some("ID already exists.".into()); return;
        }
        let preds: Vec<Predecessor> = self.edit.predecessors
            .split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
            .map(|id| Predecessor { activity_id: id, rel_type: RelType::FS, lag: 0 })
            .collect();
        for p in &preds {
            if !self.project.activities.iter().any(|a| a.id == p.activity_id) {
                self.edit.error = Some(format!("Unknown predecessor: '{}'", p.activity_id)); return;
            }
        }
        let act = Activity {
            id: self.edit.id.clone(),
            name: self.edit.name.trim().to_string(),
            duration,
            predecessors: preds,
            resource: self.edit.resource.trim().to_string(),
            wbs: String::new(),
            cpm: Default::default(),
        };
        if self.edit.is_new {
            self.status = format!("Added activity '{}'.", act.name);
            self.project.add_activity(act);
        } else {
            self.status = format!("Updated activity '{}'.", act.name);
            self.project.update_activity(act);
        }
        self.edit.open = false;
        self.edit.error = None;
    }

    // ── panels ────────────────────────────────────────────────────────────────

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            ui.label(RichText::new("⬡ CPM Scheduler").size(16.0).color(C_ACCENT).strong());
            ui.separator();

            if ui.button("📋  Load Sample").clicked() {
                self.project = Project::load_sample();
                self.sched_error = None;
                self.status = "Sample loaded. Press ▶ Schedule or F5.".into();
            }
            if ui.button("🗒  New").clicked() {
                self.project = Project::default();
                self.sched_error = None;
                self.status = "New project.".into();
            }
            ui.separator();

            if ui.button(RichText::new("＋  Add Activity").color(C_FLOAT)).clicked() {
                self.edit = EditState::new_empty();
            }

            let sched_btn = egui::Button::new(
                RichText::new("▶  Schedule  (F5)").color(Color32::WHITE).strong()
            ).fill(Color32::from_rgb(30, 90, 45));
            if ui.add(sched_btn).clicked() { self.run_schedule(); }

            ui.separator();
            if ui.button("⚙  Settings").clicked() {
                self.settings_open = true;
                self.settings_name = self.project.settings.name.clone();
                self.settings_date = self.project.settings.start_date
                    .map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
            }

            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if self.project.is_scheduled {
                    ui.label(RichText::new("● Scheduled").color(C_FLOAT).size(11.0));
                } else if !self.project.activities.is_empty() {
                    ui.label(RichText::new("○ Not scheduled").color(C_WARN).size(11.0));
                }
                ui.label(RichText::new(&self.project.settings.name).color(C_TEXT_DIM).size(12.0));
            });
        });
    }

    fn tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (t, lbl) in [(Tab::Schedule,"📊  Schedule"),(Tab::Gantt,"📅  Gantt"),(Tab::Network,"🔗  Network")] {
                let active = self.tab == t;
                let btn = egui::Button::new(
                    RichText::new(lbl).size(13.0).color(if active { C_ACCENT } else { C_TEXT_DIM })
                )
                .fill(if active { C_SURFACE2 } else { C_SURFACE })
                .min_size(Vec2::new(140.0, 32.0));
                if ui.add(btn).clicked() { self.tab = t; }
            }
        });
    }

    fn summary_bar(&self, ui: &mut Ui) {
        let acts = &self.project.activities;
        let dur = acts.iter().map(|a| a.cpm.ef as i64).max().unwrap_or(0);
        let n_cp = acts.iter().filter(|a| a.cpm.critical).count();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 28.0;
            kpi(ui, "Duration", &format!("{} days", dur), C_ACCENT);
            kpi(ui, "Activities", &format!("{}", acts.len()), C_TEXT);
            kpi(ui, "Critical", &format!("{}", n_cp), C_CRITICAL);
            kpi(ui, "Has Float", &format!("{}", acts.len() - n_cp), C_FLOAT);
        });
    }

    fn status_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if let Some(e) = &self.sched_error {
                ui.label(RichText::new(format!("⚠  {}", e)).color(C_CRITICAL).size(11.0));
            } else {
                ui.label(RichText::new(&self.status).color(C_TEXT_DIM).size(11.0));
            }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{} activities", self.project.activities.len())).color(C_TEXT_DIM).size(11.0));
            });
        });
    }

    // ── Schedule tab ──────────────────────────────────────────────────────────
    fn draw_schedule(&mut self, ui: &mut Ui) {
        let col_w: &[f32] = &[52., 160., 56., 180., 120., 48., 48., 48., 48., 52., 52., 72.];
        let headers   = ["ID","Name","Dur","Predecessors","Resource","ES","EF","LS","LF","TF","FF","Actions"];

        // Header row
        egui::Frame::none().fill(C_SURFACE2).inner_margin(egui::Margin::symmetric(8.,5.)).show(ui, |ui| {
            ui.horizontal(|ui| {
                for (h, w) in headers.iter().zip(col_w) {
                    ui.add_sized(Vec2::new(*w, 18.), egui::Label::new(
                        RichText::new(*h).size(11.).color(C_TEXT_DIM).strong()
                    ));
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical().auto_shrink([false;2]).show(ui, |ui| {
            let acts = self.project.activities.clone();

            if acts.is_empty() {
                ui.add_space(80.);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No activities yet.\nClick  ＋ Add Activity  or  📋 Load Sample  to get started.")
                        .color(C_TEXT_DIM).size(14.));
                });
                return;
            }

            for (i, act) in acts.iter().enumerate() {
                let sched = self.project.is_scheduled;
                let row_bg = if act.cpm.critical && sched { C_ROW_CRIT }
                             else if i % 2 == 0          { C_BG }
                             else                         { C_ROW_ALT };

                egui::Frame::none().fill(row_bg).inner_margin(egui::Margin::symmetric(8.,3.)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let id_col = if act.cpm.critical && sched { C_CRITICAL } else { C_ACCENT };
                        ui.add_sized(Vec2::new(52.,22.), egui::Label::new(RichText::new(&act.id).color(id_col).size(12.).monospace()));
                        ui.add_sized(Vec2::new(160.,22.), egui::Label::new(RichText::new(&act.name).color(C_TEXT).size(12.)));
                        ui.add_sized(Vec2::new(56.,22.), egui::Label::new(RichText::new(format!("{:.0}",act.duration)).color(C_TEXT).size(12.)));
                        let preds = act.predecessors.iter().map(|p|p.activity_id.as_str()).collect::<Vec<_>>().join(", ");
                        ui.add_sized(Vec2::new(180.,22.), egui::Label::new(RichText::new(&preds).color(C_TEXT_DIM).size(12.)));
                        ui.add_sized(Vec2::new(120.,22.), egui::Label::new(RichText::new(&act.resource).color(C_TEXT_DIM).size(12.)));

                        let f = |v:f64| if sched { format!("{:.0}",v) } else { "—".into() };
                        for v in [act.cpm.es, act.cpm.ef, act.cpm.ls, act.cpm.lf] {
                            ui.add_sized(Vec2::new(48.,22.), egui::Label::new(RichText::new(f(v)).color(C_TEXT).size(12.).monospace()));
                        }
                        let tf_col = if !sched { C_TEXT_DIM } else if act.cpm.critical { C_CRITICAL } else if act.cpm.tf < 5. { C_WARN } else { C_FLOAT };
                        ui.add_sized(Vec2::new(52.,22.), egui::Label::new(RichText::new(f(act.cpm.tf)).color(tf_col).size(12.).monospace()));
                        ui.add_sized(Vec2::new(52.,22.), egui::Label::new(RichText::new(f(act.cpm.ff)).color(C_TEXT_DIM).size(12.).monospace()));

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.;
                            if ui.small_button(RichText::new("✏").color(C_ACCENT)).clicked() {
                                self.edit = EditState::from_activity(act);
                            }
                            if ui.small_button(RichText::new("🗑").color(C_CRITICAL)).clicked() {
                                self.confirm_delete = Some(act.id.clone());
                            }
                        });
                    });
                });
            }
        });
    }

    // ── Gantt tab ─────────────────────────────────────────────────────────────
    fn draw_gantt(&self, ui: &mut Ui) {
        if !self.project.is_scheduled {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("Run ▶ Schedule first.").color(C_TEXT_DIM).size(14.));
            });
            return;
        }
        let acts = &self.project.activities;
        let max_day = acts.iter().map(|a| a.cpm.ef).fold(0f64, f64::max).max(1.0) as f32;
        let row_h = 28.0f32;
        let label_w = 170.0f32;
        let avail_w = ui.available_width() - label_w - 24.;
        let px = avail_w / max_day;
        let tick = ((max_day / 12.).ceil() as i32).max(1);

        // Header
        egui::Frame::none().fill(C_SURFACE2).inner_margin(egui::Margin::symmetric(8.,4.)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized(Vec2::new(label_w, 18.), egui::Label::new(RichText::new("Activity").color(C_TEXT_DIM).size(11.).strong()));
                let (resp, painter) = ui.allocate_painter(Vec2::new(avail_w, 18.), egui::Sense::hover());
                let ox = resp.rect.left();
                let oy = resp.rect.top();
                let mut d = 0i32;
                while d <= max_day as i32 {
                    let x = ox + d as f32 * px;
                    painter.text(Pos2::new(x+2., oy+2.), Align::LEFT,
                        format!("D{}", d), FontId::monospace(9.), C_TEXT_DIM);
                    d += tick;
                }
            });
        });

        egui::ScrollArea::vertical().auto_shrink([false;2]).show(ui, |ui| {
            for (i, act) in acts.iter().enumerate() {
                let row_bg = if i % 2 == 0 { C_BG } else { C_ROW_ALT };
                egui::Frame::none().fill(row_bg).inner_margin(egui::Margin { left:8., right:8., top:2., bottom:2. }).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let nc = if act.cpm.critical { C_CRITICAL } else { C_TEXT };
                        ui.add_sized(Vec2::new(label_w, row_h),
                            egui::Label::new(RichText::new(format!("{} {}", act.id, act.name)).color(nc).size(12.)));

                        let (resp, painter) = ui.allocate_painter(Vec2::new(avail_w, row_h), egui::Sense::hover());
                        let ox = resp.rect.left();
                        let oy = resp.rect.top();
                        let ymid = oy + row_h / 2.;
                        let bh = 14.0f32;

                        // Grid lines
                        let mut d = 0i32;
                        while d <= max_day as i32 {
                            painter.vline(ox + d as f32 * px, oy..=(oy+row_h), Stroke::new(0.5, C_GANTT_GRID));
                            d += tick;
                        }

                        // Float ghost
                        if act.cpm.tf > 0.01 {
                            let x0 = ox + act.cpm.ef as f32 * px;
                            let x1 = ox + act.cpm.lf as f32 * px;
                            painter.rect_filled(
                                Rect::from_x_y_ranges(x0..=x1, (ymid-bh/2.+3.)..=(ymid+bh/2.-3.)),
                                2., Color32::from_rgba_unmultiplied(63,185,80,35));
                        }

                        // Main bar
                        let x0 = ox + act.cpm.es as f32 * px;
                        let x1 = ox + act.cpm.ef as f32 * px;
                        let bar_col = if act.cpm.critical { Color32::from_rgb(248,81,73) } else { Color32::from_rgb(88,166,255) };
                        painter.rect_filled(Rect::from_x_y_ranges(x0..=x1, (ymid-bh/2.)..=(ymid+bh/2.)), 3., bar_col);
                        if x1-x0 > 22. {
                            painter.text(Pos2::new((x0+x1)/2., ymid), Align::Center,
                                format!("{:.0}", act.duration), FontId::monospace(10.), Color32::WHITE);
                        }

                        // Tooltip
                        if resp.hovered() {
                            egui::show_tooltip_at_pointer(ui.ctx(), egui::Id::new("gantt_tip"), |ui| {
                                ui.label(RichText::new(format!(
                                    "{}  {}\nES:{:.0}  EF:{:.0}  LS:{:.0}  LF:{:.0}\nTF:{:.0}  FF:{:.0}",
                                    act.id, act.name, act.cpm.es, act.cpm.ef, act.cpm.ls, act.cpm.lf, act.cpm.tf, act.cpm.ff
                                )).color(C_TEXT).size(12.));
                            });
                        }
                    });
                });
            }
        });

        ui.add_space(4.);
        egui::Frame::none().fill(C_SURFACE).inner_margin(egui::Margin::symmetric(12.,6.)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 20.;
                ui.label(RichText::new("■").color(Color32::from_rgb(248,81,73)));
                ui.label(RichText::new("Critical path").color(C_TEXT_DIM).size(11.));
                ui.label(RichText::new("■").color(Color32::from_rgb(88,166,255)));
                ui.label(RichText::new("Normal activity").color(C_TEXT_DIM).size(11.));
                ui.label(RichText::new("■").color(Color32::from_rgba_unmultiplied(63,185,80,80)));
                ui.label(RichText::new("Float window").color(C_TEXT_DIM).size(11.));
            });
        });
    }

    // ── Network tab ───────────────────────────────────────────────────────────
    fn draw_network(&self, ui: &mut Ui) {
        if !self.project.is_scheduled {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("Run ▶ Schedule first.").color(C_TEXT_DIM).size(14.));
            });
            return;
        }
        let acts = &self.project.activities;
        if acts.is_empty() { return; }

        let node_w = 115.0f32;
        let node_h = 76.0f32;
        let h_gap  = 64.0f32;
        let v_gap  = 22.0f32;

        // Layered layout by ES
        let mut layers: std::collections::BTreeMap<i64, Vec<usize>> = Default::default();
        for (i, act) in acts.iter().enumerate() {
            layers.entry(act.cpm.es as i64).or_default().push(i);
        }

        let mut positions: std::collections::HashMap<String, Pos2> = Default::default();
        let mut x = 24.0f32;
        for layer in layers.values() {
            let total_h = layer.len() as f32 * (node_h + v_gap) - v_gap;
            let mut y = 24.0 + (560.0 - total_h) / 2.0;
            for &idx in layer {
                positions.insert(acts[idx].id.clone(), Pos2::new(x, y));
                y += node_h + v_gap;
            }
            x += node_w + h_gap;
        }

        let canvas_w = x + 24.;
        egui::ScrollArea::both().auto_shrink([false;2]).show(ui, |ui| {
            let (_, painter) = ui.allocate_painter(Vec2::new(canvas_w.max(ui.available_width()), 640.), egui::Sense::hover());

            // Edges
            for act in acts {
                if let Some(&tp) = positions.get(&act.id) {
                    let tc = Pos2::new(tp.x, tp.y + node_h/2.);
                    for pred in &act.predecessors {
                        if let Some(&fp) = positions.get(&pred.activity_id) {
                            let fc = Pos2::new(fp.x + node_w, fp.y + node_h/2.);
                            let is_crit = act.cpm.critical
                                && acts.iter().find(|a| a.id == pred.activity_id).map(|a| a.cpm.critical).unwrap_or(false);
                            let col = if is_crit { C_CRITICAL } else { C_BORDER };
                            let sw  = if is_crit { 2.0 } else { 1.0 };
                            let mx = (fc.x + tc.x) / 2.;
                            painter.line_segment([fc, Pos2::new(mx, fc.y)], Stroke::new(sw, col));
                            painter.line_segment([Pos2::new(mx, fc.y), Pos2::new(mx, tc.y)], Stroke::new(sw, col));
                            painter.line_segment([Pos2::new(mx, tc.y), tc], Stroke::new(sw, col));
                            painter.arrow(Pos2::new(tc.x - 8., tc.y), Vec2::new(8., 0.), Stroke::new(sw, col));
                        }
                    }
                }
            }

            // Nodes
            for act in acts {
                if let Some(&pos) = positions.get(&act.id) {
                    let rect = Rect::from_min_size(pos, Vec2::new(node_w, node_h));
                    let crit = act.cpm.critical;
                    let bg   = if crit { Color32::from_rgb(40,18,18) } else { C_SURFACE2 };
                    let bcol = if crit { C_CRITICAL } else { C_BORDER };
                    painter.rect_filled(rect, 6., bg);
                    painter.rect_stroke(rect, 6., Stroke::new(if crit {2.} else {1.}, bcol));

                    // Top strip: ES | EF
                    let top = Rect::from_min_size(pos, Vec2::new(node_w, 20.));
                    painter.rect_filled(top, egui::Rounding { nw:6., ne:6., sw:0., se:0. },
                        if crit { Color32::from_rgb(55,22,22) } else { C_SURFACE });
                    painter.text(Pos2::new(pos.x+7., pos.y+4.), Align::LEFT,
                        format!("{:.0}", act.cpm.es), FontId::monospace(11.), C_ACCENT);
                    painter.text(Pos2::new(pos.x+node_w-7., pos.y+4.), Align::RIGHT,
                        format!("{:.0}", act.cpm.ef), FontId::monospace(11.), C_ACCENT);

                    // ID
                    painter.text(Pos2::new(pos.x+node_w/2., pos.y+28.), Align::Center,
                        &act.id, FontId::proportional(13.), if crit { C_CRITICAL } else { C_ACCENT });
                    // Name (truncated)
                    let name = if act.name.len() > 14 { &act.name[..14] } else { &act.name };
                    painter.text(Pos2::new(pos.x+node_w/2., pos.y+43.), Align::Center,
                        name, FontId::proportional(10.), C_TEXT);

                    // Bottom: LS | TF | LF
                    let tf_col = if crit { C_CRITICAL } else { C_FLOAT };
                    painter.text(Pos2::new(pos.x+7., pos.y+node_h-12.), Align::LEFT,
                        format!("{:.0}", act.cpm.ls), FontId::monospace(11.), C_WARN);
                    painter.text(Pos2::new(pos.x+node_w/2., pos.y+node_h-12.), Align::Center,
                        format!("TF:{:.0}", act.cpm.tf), FontId::monospace(10.), tf_col);
                    painter.text(Pos2::new(pos.x+node_w-7., pos.y+node_h-12.), Align::RIGHT,
                        format!("{:.0}", act.cpm.lf), FontId::monospace(11.), C_WARN);
                }
            }
        });
    }

    // ── Dialogs ───────────────────────────────────────────────────────────────
    fn dialog_edit(&mut self, ctx: &Context) {
        if !self.edit.open { return; }
        let title = if self.edit.is_new { "Add Activity" } else { "Edit Activity" };
        egui::Window::new(title)
            .resizable(false).min_width(360.).collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Grid::new("eg").num_columns(2).spacing([12.,8.]).show(ui, |ui| {
                    ui.label(RichText::new("ID").color(C_TEXT_DIM));
                    if self.edit.is_new { ui.text_edit_singleline(&mut self.edit.id); }
                    else { ui.label(RichText::new(&self.edit.id).color(C_ACCENT).monospace()); }
                    ui.end_row();
                    ui.label(RichText::new("Name *").color(C_TEXT_DIM));
                    ui.text_edit_singleline(&mut self.edit.name);
                    ui.end_row();
                    ui.label(RichText::new("Duration (days) *").color(C_TEXT_DIM));
                    ui.text_edit_singleline(&mut self.edit.duration);
                    ui.end_row();
                    ui.label(RichText::new("Predecessors").color(C_TEXT_DIM));
                    ui.text_edit_singleline(&mut self.edit.predecessors);
                    ui.end_row();
                    ui.label(RichText::new("Resource").color(C_TEXT_DIM));
                    ui.text_edit_singleline(&mut self.edit.resource);
                    ui.end_row();
                });
                ui.label(RichText::new("Predecessors: comma-separated IDs  e.g.  A, B").color(C_TEXT_DIM).size(10.).italics());
                if let Some(e) = &self.edit.error {
                    ui.label(RichText::new(format!("⚠  {}", e)).color(C_CRITICAL).size(12.));
                }
                ui.add_space(6.);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new(if self.edit.is_new {"Add"} else {"Save"}).color(Color32::WHITE))
                        .fill(Color32::from_rgb(30,90,45))).clicked() { self.apply_edit(); }
                    if ui.button("Cancel").clicked() { self.edit.open = false; self.edit.error = None; }
                });
            });
    }

    fn dialog_settings(&mut self, ctx: &Context) {
        if !self.settings_open { return; }
        egui::Window::new("Project Settings")
            .resizable(false).min_width(320.).collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Grid::new("sg").num_columns(2).spacing([12.,8.]).show(ui, |ui| {
                    ui.label(RichText::new("Project Name").color(C_TEXT_DIM));
                    ui.text_edit_singleline(&mut self.settings_name);
                    ui.end_row();
                    ui.label(RichText::new("Start Date").color(C_TEXT_DIM));
                    ui.text_edit_singleline(&mut self.settings_date);
                    ui.end_row();
                });
                ui.label(RichText::new("Date format: YYYY-MM-DD").color(C_TEXT_DIM).size(10.).italics());
                ui.add_space(6.);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Apply").color(Color32::WHITE))
                        .fill(Color32::from_rgb(30,90,45))).clicked() {
                        self.project.settings.name = self.settings_name.clone();
                        if let Ok(d) = chrono::NaiveDate::parse_from_str(&self.settings_date, "%Y-%m-%d") {
                            self.project.settings.start_date = Some(d);
                        }
                        self.project.is_dirty = true;
                        self.settings_open = false;
                    }
                    if ui.button("Cancel").clicked() { self.settings_open = false; }
                });
            });
    }

    fn dialog_delete(&mut self, ctx: &Context) {
        let id = match self.confirm_delete.clone() { Some(id) => id, None => return };
        egui::Window::new("Confirm Delete")
            .resizable(false).collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(RichText::new(format!("Delete activity  '{}'?", id)).color(C_TEXT).size(13.));
                ui.label(RichText::new("This also removes it from all predecessor lists.").color(C_TEXT_DIM).size(11.));
                ui.add_space(6.);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Delete").color(Color32::WHITE)).fill(C_CRITICAL)).clicked() {
                        self.project.remove_activity(&id);
                        self.status = format!("Deleted activity '{}'.", id);
                        self.confirm_delete = None;
                    }
                    if ui.button("Cancel").clicked() { self.confirm_delete = None; }
                });
            });
    }
}

fn kpi(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(value).size(20.).color(color).strong());
        ui.label(RichText::new(label).size(10.).color(C_TEXT_DIM));
    });
}

impl eframe::App for CpmApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // F5 shortcut
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) { self.run_schedule(); }

        // Visuals
        let mut v = egui::Visuals::dark();
        v.panel_fill = C_BG;
        v.window_fill = C_SURFACE;
        v.override_text_color = Some(C_TEXT);
        v.widgets.noninteractive.bg_fill = C_SURFACE;
        v.widgets.inactive.bg_fill = C_SURFACE2;
        v.widgets.hovered.bg_fill = C_SURFACE2;
        v.widgets.active.bg_fill = C_BORDER;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1., C_BORDER);
        ctx.set_visuals(v);

        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::none().fill(C_SURFACE).inner_margin(egui::Margin::symmetric(10.,8.)))
            .show(ctx, |ui| self.toolbar(ui));

        egui::TopBottomPanel::top("tabs")
            .frame(egui::Frame::none().fill(C_SURFACE).inner_margin(egui::Margin { left:10., right:10., top:0., bottom:0. }))
            .show(ctx, |ui| { self.tab_bar(ui); ui.separator(); });

        if self.project.is_scheduled {
            egui::TopBottomPanel::top("kpi")
                .frame(egui::Frame::none().fill(C_SURFACE).inner_margin(egui::Margin::symmetric(16.,8.)))
                .show(ctx, |ui| { self.summary_bar(ui); ui.separator(); });
        }

        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame::none().fill(C_SURFACE).inner_margin(egui::Margin::symmetric(10.,5.)))
            .show(ctx, |ui| self.status_bar(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(C_BG))
            .show(ctx, |ui| {
                match self.tab {
                    Tab::Schedule => self.draw_schedule(ui),
                    Tab::Gantt    => self.draw_gantt(ui),
                    Tab::Network  => self.draw_network(ui),
                }
            });

        self.dialog_edit(ctx);
        self.dialog_settings(ctx);
        self.dialog_delete(ctx);
    }
}
