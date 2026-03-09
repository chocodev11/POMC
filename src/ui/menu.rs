use std::path::Path;
use std::time::Instant;

use egui::{Color32, Pos2, Rect, Stroke, Vec2};

use super::font::{mc_text, mc_text_centered, mc_text_width, RotatedText};
use super::hud::{gui_scale, HudTextures, UV_FULL};
use super::options::{GameSettings, GuiScale, SettingsFile};
use super::server_list::{is_valid_address, ping_all_servers, PingResults, PingState, ServerEntry, ServerList};
use crate::assets::AssetIndex;

pub enum MenuAction {
    None,
    Connect { server: String, username: String },
    Quit,
}

const BG: Color32 = Color32::from_rgb(12, 12, 20);
const CARD: Color32 = Color32::from_rgb(20, 20, 30);
const CARD_HOVER: Color32 = Color32::from_rgb(26, 26, 38);
const CARD_SELECTED: Color32 = Color32::from_rgb(22, 36, 52);
const ACCENT: Color32 = Color32::from_rgb(67, 160, 71);
const ACCENT_HOVER: Color32 = Color32::from_rgb(87, 182, 91);
const DANGER: Color32 = Color32::from_rgb(198, 44, 44);
const DANGER_HOVER: Color32 = Color32::from_rgb(220, 60, 60);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 230, 235);
const TEXT_DIM: Color32 = Color32::from_rgb(100, 100, 115);
const TEXT_DISABLED: Color32 = Color32::from_rgb(45, 45, 55);
const GHOST_HOVER: Color32 = Color32::from_rgb(28, 28, 40);
const INPUT_BG: Color32 = Color32::from_rgb(16, 16, 24);
const INPUT_BORDER: Color32 = Color32::from_rgb(32, 32, 44);

const ROUND: f32 = 6.0;
const LOGO_WIDTH: f32 = 256.0;
const EDITION_WIDTH: f32 = 128.0;
const EDITION_OVERLAP: f32 = 7.0;
const LOGO_Y_OFFSET: f32 = 30.0;
const ENTRY_H: f32 = 44.0;
const FORM_W: f32 = 240.0;
const BTN_H: f32 = 26.0;
const BTN_FULL_W: f32 = 200.0;
const ACCENT_BAR: f32 = 3.0;
const TITLE_SIZE: f32 = 14.0;
const BODY_SIZE: f32 = 8.0;

enum Screen {
    Main,
    Options,
    ServerList,
    ConfirmDelete(usize),
    DirectConnect,
    AddServer,
    EditServer(usize),
}

#[derive(Clone, Copy)]
enum BtnKind { Accent, Danger, Ghost }

pub struct MainMenu {
    username: String,
    screen: Screen,
    splash: Option<String>,
    start_time: Instant,
    server_list: ServerList,
    selected_server: Option<usize>,
    edit_name: String,
    edit_address: String,
    last_mp_ip: String,
    ping_results: PingResults,
    rt: std::sync::Arc<tokio::runtime::Runtime>,
    settings_file: SettingsFile,
}

impl MainMenu {
    pub fn new(game_dir: &Path, rt: std::sync::Arc<tokio::runtime::Runtime>) -> Self {
        let server_list = ServerList::load(game_dir);
        let ping_results: PingResults = Default::default();
        ping_all_servers(&rt, &server_list.servers, &ping_results);
        let settings_file = SettingsFile::load(game_dir);
        Self {
            username: "Steve".into(),
            screen: Screen::Main,
            splash: None,
            start_time: Instant::now(),
            server_list,
            selected_server: None,
            edit_name: String::new(),
            edit_address: String::new(),
            last_mp_ip: String::new(),
            ping_results,
            rt,
            settings_file,
        }
    }

    pub fn settings(&self) -> &GameSettings {
        &self.settings_file.settings
    }

    pub fn load_splash(&mut self, assets_dir: &Path, asset_index: &Option<AssetIndex>) {
        let path = asset_index
            .as_ref()
            .and_then(|idx| idx.resolve("minecraft/texts/splashes.txt"))
            .unwrap_or_else(|| assets_dir.join("assets/minecraft/texts/splashes.txt"));

        let Ok(contents) = std::fs::read_to_string(&path) else {
            log::warn!("Failed to load splashes.txt");
            return;
        };

        let lines: Vec<&str> = contents.lines().filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            return;
        }

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.start_time.hash(&mut hasher);
        let index = hasher.finish() as usize % lines.len();
        self.splash = Some(lines[index].to_string());
    }

    pub fn draw(&mut self, ctx: &egui::Context, textures: &HudTextures) -> MenuAction {
        let mut action = MenuAction::None;

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let screen = ui.max_rect();
                draw_background(ui.painter(), screen);

                let gs = gui_scale(ctx);

                match self.screen {
                    Screen::Main => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(LOGO_Y_OFFSET * gs);
                            draw_title_logo(ui, textures, &self.splash, self.start_time, gs);
                            ui.add_space(44.0 * gs);
                            self.draw_main_buttons(ui, textures, &mut action, gs);
                        });
                    }
                    Screen::Options => {
                        self.draw_options(ui, gs);
                    }
                    Screen::ServerList => {
                        self.draw_server_list(ui, &mut action, gs);
                    }
                    Screen::ConfirmDelete(_) => {
                        self.draw_confirm_delete(ui, gs);
                    }
                    Screen::DirectConnect => {
                        self.draw_direct_connect(ui, &mut action, gs);
                    }
                    Screen::AddServer | Screen::EditServer(_) => {
                        self.draw_edit_server(ui, gs);
                    }
                }

                draw_bottom_text(ui, screen, gs);
            });

        action
    }

    fn draw_main_buttons(
        &mut self,
        ui: &mut egui::Ui,
        textures: &HudTextures,
        action: &mut MenuAction,
        gs: f32,
    ) {
        let full_w = BTN_FULL_W * gs;
        let gap = 8.0 * gs;

        if btn(ui, "Singleplayer", full_w, BtnKind::Ghost, true) {
            // TODO: singleplayer world list
        }

        ui.add_space(gap);

        if btn(ui, "Multiplayer", full_w, BtnKind::Accent, true) {
            self.screen = Screen::ServerList;
        }

        ui.add_space(gap * 4.0);

        ui.horizontal(|ui| {
            let icon_size = BTN_H * gs;
            let half_w = (full_w - gap) / 2.0;
            let total_w = icon_size + gap + full_w + gap + icon_size;
            let offset = (ui.available_width() - total_w) / 2.0;
            ui.add_space(offset.max(0.0));
            ui.spacing_mut().item_spacing.x = gap;

            icon_btn(ui, &textures.icon_language, icon_size);
            if btn(ui, "Options...", half_w, BtnKind::Ghost, true) {
                self.screen = Screen::Options;
            }
            if btn(ui, "Quit Game", half_w, BtnKind::Ghost, true) {
                *action = MenuAction::Quit;
            }
            icon_btn(ui, &textures.icon_accessibility, icon_size);
        });
    }

    fn refresh_servers(&self) {
        ping_all_servers(&self.rt, &self.server_list.servers, &self.ping_results);
    }

    fn draw_server_list(
        &mut self,
        ui: &mut egui::Ui,
        action: &mut MenuAction,
        gs: f32,
    ) {
        if ui.input(|i| i.key_pressed(egui::Key::F5)) {
            self.refresh_servers();
        }

        let screen = ui.max_rect();
        let margin = 32.0 * gs;
        let gap = 8.0 * gs;
        let font_size = BODY_SIZE * gs;
        let top_btn_w = 100.0 * gs;
        let bottom_btn_w = 74.0 * gs;

        ui.vertical_centered(|ui| {
            ui.add_space(margin);
            mc_label_centered(ui, "Multiplayer", TITLE_SIZE * gs, TEXT_PRIMARY);
            ui.add_space(20.0 * gs);

            let btn_h = BTN_H * gs;
            let footer_h = btn_h * 2.0 + gap * 3.0;
            let list_h = (screen.height() - margin * 2.0 - TITLE_SIZE * gs - 20.0 * gs - footer_h - 20.0 * gs).max(60.0 * gs);

            let entry_h = ENTRY_H * gs;
            let list_w = (screen.width() - margin * 2.0).min(400.0 * gs);
            let ping_results = self.ping_results.read().clone();
            let mut any_pinging = false;

            egui::ScrollArea::vertical()
                .max_height(list_h)
                .show(ui, |ui| {
                    if self.server_list.servers.is_empty() {
                        ui.add_space(40.0 * gs);
                        mc_label_centered(ui, "No servers added", font_size, TEXT_DIM);
                    }

                    for (i, server) in self.server_list.servers.iter().enumerate() {
                        let selected = self.selected_server == Some(i);
                        let (card_rect, response) = ui.allocate_exact_size(
                            Vec2::new(list_w, entry_h),
                            egui::Sense::click(),
                        );

                        let bg = if selected {
                            CARD_SELECTED
                        } else if response.hovered() {
                            CARD_HOVER
                        } else {
                            CARD
                        };
                        ui.painter().rect_filled(card_rect, ROUND, bg);

                        if selected {
                            let bar = Rect::from_min_size(
                                card_rect.min,
                                Vec2::new(ACCENT_BAR * gs, card_rect.height()),
                            );
                            ui.painter().rect_filled(
                                bar,
                                egui::CornerRadius { nw: ROUND as u8, sw: ROUND as u8, ne: 0, se: 0 },
                                ACCENT,
                            );
                        }

                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        let text_x = card_rect.min.x + 12.0 * gs;
                        let name_y = card_rect.min.y + 8.0 * gs;

                        mc_text(
                            ui.painter(), ui.ctx(),
                            Pos2::new(text_x, name_y),
                            &server.name, font_size, TEXT_PRIMARY, true,
                        );

                        let motd_y = name_y + font_size + 4.0 * gs;
                        draw_server_status(
                            ui, &ping_results, &server.address,
                            &EntryLayout { text_x, motd_y, entry_rect: card_rect, font_size, gs },
                            &mut any_pinging,
                        );

                        if response.clicked() {
                            self.selected_server = Some(i);
                        }
                        if response.double_clicked() {
                            *action = MenuAction::Connect {
                                server: server.address.clone(),
                                username: self.username.clone(),
                            };
                        }

                        ui.add_space(4.0 * gs);
                    }
                });

            if any_pinging {
                ui.ctx().request_repaint();
            }

            ui.add_space(gap * 2.0);

            let has_selection = self.selected_server.is_some();

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let row_w = top_btn_w * 3.0 + gap * 2.0;
                let offset = (ui.available_width() - row_w) / 2.0;
                ui.add_space(offset.max(0.0));

                if btn(ui, "Join", top_btn_w, BtnKind::Accent, has_selection) {
                    if let Some(idx) = self.selected_server {
                        if let Some(server) = self.server_list.servers.get(idx) {
                            *action = MenuAction::Connect {
                                server: server.address.clone(),
                                username: self.username.clone(),
                            };
                        }
                    }
                }

                if btn(ui, "Direct Connect", top_btn_w, BtnKind::Ghost, true) {
                    self.edit_address = self.last_mp_ip.clone();
                    self.screen = Screen::DirectConnect;
                }

                if btn(ui, "Add Server", top_btn_w, BtnKind::Ghost, true) {
                    self.edit_name = String::new();
                    self.edit_address = String::new();
                    self.screen = Screen::AddServer;
                }
            });

            ui.add_space(gap);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let row_w = bottom_btn_w * 4.0 + gap * 3.0;
                let offset = (ui.available_width() - row_w) / 2.0;
                ui.add_space(offset.max(0.0));

                if btn(ui, "Edit", bottom_btn_w, BtnKind::Ghost, has_selection) {
                    if let Some(idx) = self.selected_server {
                        if let Some(server) = self.server_list.servers.get(idx) {
                            self.edit_name = server.name.clone();
                            self.edit_address = server.address.clone();
                            self.screen = Screen::EditServer(idx);
                        }
                    }
                }

                if btn(ui, "Delete", bottom_btn_w, BtnKind::Danger, has_selection) {
                    if let Some(idx) = self.selected_server {
                        self.screen = Screen::ConfirmDelete(idx);
                    }
                }

                if btn(ui, "Refresh", bottom_btn_w, BtnKind::Ghost, true) {
                    self.refresh_servers();
                }

                if btn(ui, "Back", bottom_btn_w, BtnKind::Ghost, true) {
                    self.screen = Screen::Main;
                    self.selected_server = None;
                }
            });
        });
    }

    fn draw_confirm_delete(&mut self, ui: &mut egui::Ui, gs: f32) {
        let Screen::ConfirmDelete(idx) = self.screen else {
            return;
        };

        let form_w = FORM_W * gs;
        let warning = self
            .server_list
            .servers
            .get(idx)
            .map(|s| format!("'{}' will be lost forever! (A long time!)", s.name))
            .unwrap_or_default();

        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            mc_label_centered(ui, "Are you sure?", TITLE_SIZE * gs, TEXT_PRIMARY);
            ui.add_space(12.0 * gs);
            mc_label_centered(ui, &warning, BODY_SIZE * gs, TEXT_DIM);
            ui.add_space(32.0 * gs);

            if btn(ui, "Delete", form_w, BtnKind::Danger, true) {
                self.server_list.remove(idx);
                self.selected_server = None;
                self.screen = Screen::ServerList;
            }
            ui.add_space(8.0 * gs);
            if btn(ui, "Cancel", form_w, BtnKind::Ghost, true) {
                self.screen = Screen::ServerList;
            }
        });
    }

    fn draw_direct_connect(
        &mut self,
        ui: &mut egui::Ui,
        action: &mut MenuAction,
        gs: f32,
    ) {
        let form_w = FORM_W * gs;

        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.25);
            mc_label_centered(ui, "Direct Connect", TITLE_SIZE * gs, TEXT_PRIMARY);
            ui.add_space(28.0 * gs);

            mc_label(ui, "Server Address", BODY_SIZE * gs, TEXT_DIM, form_w);
            ui.add_space(6.0 * gs);
            let response = themed_input(ui, &mut self.edit_address, form_w, gs);
            let enter_pressed =
                response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.add_space(28.0 * gs);

            let valid = is_valid_address(&self.edit_address);
            if btn(ui, "Join Server", form_w, BtnKind::Accent, valid) || (enter_pressed && valid) {
                self.last_mp_ip = self.edit_address.clone();
                *action = MenuAction::Connect {
                    server: self.edit_address.clone(),
                    username: self.username.clone(),
                };
            }
            ui.add_space(8.0 * gs);
            if btn(ui, "Cancel", form_w, BtnKind::Ghost, true) {
                self.screen = Screen::ServerList;
            }
        });
    }

    fn draw_edit_server(&mut self, ui: &mut egui::Ui, gs: f32) {
        let form_w = FORM_W * gs;
        let is_edit = matches!(self.screen, Screen::EditServer(_));
        let title = if is_edit { "Edit Server" } else { "Add Server" };

        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.18);
            mc_label_centered(ui, title, TITLE_SIZE * gs, TEXT_PRIMARY);
            ui.add_space(28.0 * gs);

            mc_label(ui, "Server Name", BODY_SIZE * gs, TEXT_DIM, form_w);
            ui.add_space(6.0 * gs);
            themed_input(ui, &mut self.edit_name, form_w, gs);
            ui.add_space(16.0 * gs);

            mc_label(ui, "Server Address", BODY_SIZE * gs, TEXT_DIM, form_w);
            ui.add_space(6.0 * gs);
            themed_input(ui, &mut self.edit_address, form_w, gs);
            ui.add_space(28.0 * gs);

            let valid = is_valid_address(&self.edit_address);
            if btn(ui, "Done", form_w, BtnKind::Accent, valid) {
                let name = if self.edit_name.is_empty() {
                    "Minecraft Server".to_string()
                } else {
                    self.edit_name.clone()
                };
                let addr = self.edit_address.clone();
                let entry = ServerEntry { name, address: addr.clone() };
                if let Screen::EditServer(idx) = self.screen {
                    self.server_list.update(idx, entry);
                } else {
                    self.server_list.add(entry);
                }
                ping_all_servers(
                    &self.rt,
                    &[ServerEntry { name: String::new(), address: addr }],
                    &self.ping_results,
                );
                self.screen = Screen::ServerList;
            }
            ui.add_space(8.0 * gs);
            if btn(ui, "Cancel", form_w, BtnKind::Ghost, true) {
                self.screen = Screen::ServerList;
            }
        });
    }

    fn draw_options(&mut self, ui: &mut egui::Ui, gs: f32) {
        let form_w = 280.0 * gs;
        let gap = 8.0 * gs;
        let row = OptionRow {
            width: form_w,
            height: BTN_H * gs,
            font_size: BODY_SIZE * gs,
            label_size: 7.0 * gs,
            gs,
        };

        let screen = ui.max_rect();
        let margin = 24.0 * gs;
        let btn_h = BTN_H * gs;
        let footer_h = btn_h + gap * 2.0;

        ui.vertical_centered(|ui| {
            ui.add_space(margin);
            mc_label_centered(ui, "Options", TITLE_SIZE * gs, TEXT_PRIMARY);
            ui.add_space(20.0 * gs);

            let scroll_h = (screen.height() - margin * 2.0 - TITLE_SIZE * gs - 20.0 * gs - footer_h).max(60.0 * gs);

            egui::ScrollArea::vertical()
                .max_height(scroll_h)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let s = &mut self.settings_file.settings;

                        row.slider(ui, "FOV", &mut s.fov, 30.0..=110.0,
                            |v| if (v - 70.0).abs() < 0.5 { "Normal".into() } else { format!("{:.0}", v) });

                        ui.add_space(gap);

                        row.slider(ui, "Sensitivity", &mut s.sensitivity, 0.0..=200.0,
                            |v| if (v - 100.0).abs() < 0.5 { "Normal".into() } else { format!("{:.0}%", v) });

                        ui.add_space(gap);

                        let mut vd = s.view_distance as f32;
                        row.slider(ui, "View Distance", &mut vd, 2.0..=32.0,
                            |v| format!("{:.0} chunks", v));
                        s.view_distance = vd.round() as u32;

                        ui.add_space(gap);

                        row.toggle(ui, "V-Sync", &mut s.vsync);

                        ui.add_space(gap);

                        let gui_values = ["Auto", "1", "2", "3", "4"];
                        let mut gui_idx = match s.gui_scale {
                            GuiScale::Auto => 0usize,
                            GuiScale::Fixed(n) => n as usize,
                        };
                        row.cycle(ui, "GUI Scale", &mut gui_idx, &gui_values);
                        s.gui_scale = if gui_idx == 0 { GuiScale::Auto } else { GuiScale::Fixed(gui_idx as u32) };
                    });
                });

            ui.add_space(gap * 2.0);

            if btn(ui, "Done", form_w, BtnKind::Accent, true) {
                self.settings_file.save();
                self.screen = Screen::Main;
            }
        });
    }
}

struct OptionRow {
    width: f32,
    height: f32,
    font_size: f32,
    label_size: f32,
    gs: f32,
}

impl OptionRow {
    fn allocate(&self, ui: &mut egui::Ui, label: &str, sense: egui::Sense) -> (Rect, egui::Response) {
        mc_label(ui, label, self.label_size, TEXT_DIM, self.width);
        ui.add_space(2.0 * self.gs);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(self.width, self.height), sense);
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        (rect, response)
    }

    fn draw_bg(&self, ui: &mut egui::Ui, rect: Rect, bg: Color32, border: Color32) {
        ui.painter().rect_filled(rect, ROUND, bg);
        ui.painter().rect_stroke(rect, ROUND, Stroke::new(1.0, border), egui::epaint::StrokeKind::Inside);
    }

    fn slider(&self, ui: &mut egui::Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, fmt: impl Fn(f32) -> String) {
        let (rect, response) = self.allocate(ui, label, egui::Sense::click_and_drag());
        self.draw_bg(ui, rect, INPUT_BG, INPUT_BORDER);

        let t = (*value - range.start()) / (range.end() - range.start());
        let fill_w = rect.width() * t.clamp(0.0, 1.0);
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height())),
            ROUND, ACCENT.gamma_multiply(0.5),
        );

        let knob_x = rect.min.x + fill_w;
        let knob_rect = Rect::from_min_size(
            Pos2::new((knob_x - 3.0 * self.gs).max(rect.min.x), rect.min.y),
            Vec2::new(6.0 * self.gs, rect.height()),
        );
        let knob_color = if response.hovered() || response.dragged() { ACCENT_HOVER } else { ACCENT };
        ui.painter().rect_filled(knob_rect, ROUND, knob_color);

        if response.dragged() || response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let t = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                *value = *range.start() + t * (range.end() - range.start());
            }
        }

        mc_text_centered(ui.painter(), ui.ctx(), rect.center(), &fmt(*value), self.font_size, TEXT_PRIMARY, true);
    }

    fn toggle(&self, ui: &mut egui::Ui, label: &str, value: &mut bool) {
        let (rect, response) = self.allocate(ui, label, egui::Sense::click());

        let (bg, border) = if *value { (ACCENT.gamma_multiply(0.3), ACCENT) } else { (INPUT_BG, INPUT_BORDER) };
        self.draw_bg(ui, rect, bg, border);

        let (display, text_color) = if *value { ("ON", ACCENT) } else { ("OFF", TEXT_DIM) };
        mc_text_centered(ui.painter(), ui.ctx(), rect.center(), display, self.font_size, text_color, true);

        if response.clicked() {
            *value = !*value;
        }
    }

    fn cycle(&self, ui: &mut egui::Ui, label: &str, index: &mut usize, values: &[&str]) {
        let (rect, response) = self.allocate(ui, label, egui::Sense::click());
        self.draw_bg(ui, rect, INPUT_BG, INPUT_BORDER);

        let display = values.get(*index).copied().unwrap_or("?");
        mc_text_centered(ui.painter(), ui.ctx(), rect.center(), display, self.font_size, TEXT_PRIMARY, true);

        if response.clicked() {
            *index = (*index + 1) % values.len();
        }
    }
}

fn btn(ui: &mut egui::Ui, label: &str, width: f32, kind: BtnKind, enabled: bool) -> bool {
    let gs = gui_scale(ui.ctx());
    let h = BTN_H * gs;
    let font_size = BODY_SIZE * gs;

    if !enabled {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, h), egui::Sense::hover());
        if !matches!(kind, BtnKind::Ghost) {
            ui.painter().rect_filled(rect, ROUND, Color32::from_rgb(18, 18, 26));
        }
        mc_text_centered(ui.painter(), ui.ctx(), rect.center(), label, font_size, TEXT_DISABLED, true);
        return false;
    }

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, h), egui::Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let hovered = response.hovered();
    let (bg, text_color) = match kind {
        BtnKind::Accent => {
            let bg = if hovered { ACCENT_HOVER } else { ACCENT };
            (bg, Color32::WHITE)
        }
        BtnKind::Danger => {
            let bg = if hovered { DANGER_HOVER } else { DANGER };
            (bg, Color32::WHITE)
        }
        BtnKind::Ghost => {
            let bg = if hovered { GHOST_HOVER } else { Color32::TRANSPARENT };
            let text = if hovered { TEXT_PRIMARY } else { TEXT_DIM };
            (bg, text)
        }
    };

    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, ROUND, bg);
    }
    mc_text_centered(ui.painter(), ui.ctx(), rect.center(), label, font_size, text_color, true);

    response.clicked()
}

fn icon_btn(ui: &mut egui::Ui, icon: &egui::TextureHandle, size: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.painter().rect_filled(rect, ROUND, GHOST_HOVER);
    }
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size * 0.45));
    let tint = if response.hovered() { Color32::WHITE } else { TEXT_DIM };
    ui.painter().image(icon.id(), icon_rect, UV_FULL, tint);
    response.clicked()
}

fn themed_input(ui: &mut egui::Ui, text: &mut String, width: f32, gs: f32) -> egui::Response {
    ui.scope(|ui| {
        let cr = egui::CornerRadius::same(ROUND as u8);
        let w = &mut ui.visuals_mut().widgets;
        w.inactive.bg_fill = INPUT_BG;
        w.inactive.bg_stroke = Stroke::new(1.0, INPUT_BORDER);
        w.inactive.corner_radius = cr;
        w.hovered.bg_fill = INPUT_BG;
        w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(44, 44, 58));
        w.hovered.corner_radius = cr;
        w.active.bg_fill = INPUT_BG;
        w.active.bg_stroke = Stroke::new(1.5, ACCENT);
        w.active.corner_radius = cr;

        ui.add_sized([width, BTN_H * gs], egui::TextEdit::singleline(text))
    }).inner
}

fn draw_background(painter: &egui::Painter, screen: Rect) {
    painter.rect_filled(screen, 0.0, BG);
    let center = screen.center();
    let radius = screen.width().max(screen.height()) * 0.6;
    for i in 0..8 {
        let t = i as f32 / 8.0;
        let r = radius * (1.0 - t);
        let alpha = (4.0 * (1.0 - t)) as u8;
        let glow = Rect::from_center_size(center, Vec2::splat(r * 2.0));
        painter.rect_filled(glow, r, Color32::from_rgba_premultiplied(alpha, alpha, alpha, alpha));
    }
}

fn mc_label_centered(ui: &mut egui::Ui, text: &str, font_size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), font_size),
        egui::Sense::hover(),
    );
    mc_text_centered(ui.painter(), ui.ctx(), rect.center(), text, font_size, color, true);
}

fn mc_label(ui: &mut egui::Ui, text: &str, font_size: f32, color: Color32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(width, font_size),
        egui::Sense::hover(),
    );
    mc_text(ui.painter(), ui.ctx(), rect.min, text, font_size, color, true);
}

fn draw_title_logo(
    ui: &mut egui::Ui,
    textures: &HudTextures,
    splash: &Option<String>,
    start_time: Instant,
    gs: f32,
) {
    let logo_w = LOGO_WIDTH * gs;
    let logo_rect = draw_scaled_image(ui, &textures.title_logo, logo_w);
    ui.add_space(-EDITION_OVERLAP * gs - ui.spacing().item_spacing.y);
    draw_scaled_image(ui, &textures.edition_badge, EDITION_WIDTH * gs);

    if let Some(splash_text) = splash {
        draw_splash(ui, splash_text, logo_rect, start_time, gs);
    }
}

fn draw_scaled_image(ui: &mut egui::Ui, texture: &egui::TextureHandle, width: f32) -> Rect {
    let aspect = texture.size()[0] as f32 / texture.size()[1] as f32;
    let size = Vec2::new(width, width / aspect);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .image(texture.id(), rect, UV_FULL, Color32::WHITE);
    rect
}

fn draw_splash(ui: &mut egui::Ui, text: &str, logo_rect: Rect, start_time: Instant, gs: f32) {
    let elapsed_ms = start_time.elapsed().as_millis() as f32;
    let cycle = (elapsed_ms % 1000.0) / 1000.0 * std::f32::consts::TAU;
    let pulse = 1.8 - cycle.sin().abs() * 0.1;

    let base_scale = 16.0 * gs;
    let text_w = mc_text_width(ui.ctx(), text, base_scale);
    let splash_scale = pulse * 100.0 / (text_w + 32.0 * gs);
    let font_scale = base_scale * splash_scale;

    let center = Pos2::new(
        logo_rect.center().x + 123.0 * gs,
        logo_rect.min.y + 69.0 * gs,
    );

    let yellow = Color32::from_rgb(255, 255, 0);
    let rotation = -std::f32::consts::PI / 9.0;

    if let Some(font) = super::font::McFont::get(ui.ctx()) {
        let w = font.text_width(text, font_scale);
        let pos = Pos2::new(center.x - w / 2.0, center.y - font_scale / 2.0);
        let transform = RotatedText { pivot: center, angle: rotation };
        font.draw_text_rotated(ui.painter(), pos, text, font_scale, yellow, transform);
    }

    ui.ctx().request_repaint();
}

fn draw_bottom_text(ui: &mut egui::Ui, screen: Rect, gs: f32) {
    let painter = ui.painter();
    let ctx = ui.ctx();
    let font_size = 7.0 * gs;
    let pad = 4.0 * gs;
    let y = screen.max.y - pad - font_size;

    let left = "Minecraft 1.21.11";
    let name = "Ferrite";
    let tag = "early dev";
    let tag_size = font_size * 0.65;
    let gap = 2.0 * gs;

    mc_text(painter, ctx, Pos2::new(pad, y), left, font_size, TEXT_DIM, true);

    let tag_w = mc_text_width(ctx, tag, tag_size);
    let name_w = mc_text_width(ctx, name, font_size);
    let total_w = name_w + gap + tag_w;
    let name_x = screen.max.x - pad - total_w;
    mc_text(painter, ctx, Pos2::new(name_x, y), name, font_size, TEXT_DIM, true);
    mc_text(painter, ctx, Pos2::new(name_x + name_w + gap, y), tag, tag_size, TEXT_DIM, true);
}

struct EntryLayout {
    text_x: f32,
    motd_y: f32,
    entry_rect: Rect,
    font_size: f32,
    gs: f32,
}

fn draw_server_status(
    ui: &mut egui::Ui,
    ping_results: &std::collections::HashMap<String, PingState>,
    address: &str,
    layout: &EntryLayout,
    any_pinging: &mut bool,
) {
    let EntryLayout { text_x, motd_y, ref entry_rect, font_size, gs } = *layout;

    let Some(state) = ping_results.get(address) else {
        mc_text(ui.painter(), ui.ctx(), Pos2::new(text_x, motd_y), address, font_size, TEXT_DIM, true);
        return;
    };

    match state {
        PingState::Pinging => {
            *any_pinging = true;
            let dots = match (ui.ctx().input(|i| i.time) * 2.0) as usize % 4 {
                0 => "Pinging",
                1 => "Pinging.",
                2 => "Pinging..",
                _ => "Pinging...",
            };
            mc_text(ui.painter(), ui.ctx(), Pos2::new(text_x, motd_y), dots, font_size, TEXT_DIM, true);
        }
        PingState::Success { motd, online, max, latency_ms, .. } => {
            mc_text(ui.painter(), ui.ctx(), Pos2::new(text_x, motd_y), motd, font_size, TEXT_DIM, true);

            let player_text = format!("{online}/{max}");
            let right_x = entry_rect.max.x - 10.0 * gs;

            let player_w = mc_text_width(ui.ctx(), &player_text, font_size);
            mc_text(
                ui.painter(), ui.ctx(),
                Pos2::new(right_x - player_w, entry_rect.min.y + 8.0 * gs),
                &player_text, font_size, TEXT_DIM, true,
            );

            let (bars, bar_color) = ping_level(*latency_ms);
            let bar_w = 10.0 * gs;
            let bar_h = 8.0 * gs;
            let bar_x = right_x - player_w - 6.0 * gs - bar_w;
            let bar_y = entry_rect.min.y + 8.0 * gs;
            draw_ping_bars(ui.painter(), Pos2::new(bar_x, bar_y), bar_w, bar_h, bars, bar_color);
        }
        PingState::Failed(err) => {
            let display = if err.len() > 40 { "Can't connect to server" } else { err };
            mc_text(ui.painter(), ui.ctx(), Pos2::new(text_x, motd_y), display, font_size, Color32::from_rgb(229, 57, 53), true);
        }
    }
}

const PING_THRESHOLDS: [(u64, u8, Color32); 5] = [
    (150,  5, Color32::from_rgb(67, 160, 71)),
    (300,  4, Color32::from_rgb(129, 199, 132)),
    (600,  3, Color32::from_rgb(255, 238, 88)),
    (1000, 2, Color32::from_rgb(255, 167, 38)),
    (u64::MAX, 1, Color32::from_rgb(229, 57, 53)),
];

fn ping_level(ms: u64) -> (u8, Color32) {
    for &(threshold, bars, color) in &PING_THRESHOLDS {
        if ms < threshold {
            return (bars, color);
        }
    }
    (1, PING_THRESHOLDS[4].2)
}

fn draw_ping_bars(painter: &egui::Painter, pos: Pos2, w: f32, h: f32, bars: u8, color: Color32) {
    let bar_w = w / 5.0;
    let inactive = Color32::from_rgb(30, 30, 40);

    for i in 0..5u8 {
        let bar_h = h * (i as f32 + 1.0) / 5.0;
        let bar_x = pos.x + i as f32 * bar_w;
        let bar_y = pos.y + h - bar_h;
        let c = if i < bars { color } else { inactive };
        let rect = Rect::from_min_size(Pos2::new(bar_x, bar_y), Vec2::new(bar_w - 1.0, bar_h));
        painter.rect_filled(rect, 1.0, c);
    }
}
