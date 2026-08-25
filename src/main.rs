// ===========================================================================
// BATTLE RATS
// A 2D side-scrolling tower defense inspired by "The Battle Cats", built with
// macroquad. Every visual is a drawn shape (circle/rect/triangle/diamond) —
// no external image/audio assets required.
//
// GAMEPLAY
// - You defend a base on the LEFT. The enemy defends a base on the RIGHT.
// - Money trickles in automatically. Spend it on buttons at the bottom to
//   deploy units. Units auto-walk toward the enemy and fight whatever they
//   run into (unit or base).
// - Enemies spawn automatically and get tougher / more frequent over time.
// - Win by reducing the enemy base HP to 0. Lose if yours hits 0 first.
// ===========================================================================

use macroquad::prelude::*;
use macroquad::rand::gen_range;

// ---------------------------------------------------------------------------
// Tunable constants
// ---------------------------------------------------------------------------
const LANE_Y_FRAC: f32 = 0.62; // where the lane sits, as a fraction of screen height
const BASE_MARGIN: f32 = 60.0; // distance of each base from its screen edge
const BASE_MAX_HP: f32 = 1000.0;
const STARTING_MONEY: f32 = 120.0;
const INCOME_PER_SEC: f32 = 14.0;

// ---------------------------------------------------------------------------
// Team / unit definitions
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Debug)]
enum Team {
    Player,
    Enemy,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum PlayerKind {
    Basic,
    Tank,
    Spear,
    Bomb,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum EnemyKind {
    Grunt,
    Big,
    Runner,
    Boss,
}

#[derive(Clone, Copy)]
struct UnitStats {
    cost: f32,
    hp: f32,
    attack: f32,
    range: f32,
    speed: f32,       // px/sec
    atk_cooldown: f32, // seconds between attacks
    radius: f32,
    production_time: f32, // cooldown on the deploy button
}

impl PlayerKind {
    fn stats(self) -> UnitStats {
        match self {
            PlayerKind::Basic => UnitStats {
                cost: 40.0,
                hp: 90.0,
                attack: 14.0,
                range: 18.0,
                speed: 55.0,
                atk_cooldown: 0.8,
                radius: 16.0,
                production_time: 1.0,
            },
            PlayerKind::Tank => UnitStats {
                cost: 90.0,
                hp: 340.0,
                attack: 10.0,
                range: 18.0,
                speed: 30.0,
                atk_cooldown: 1.0,
                radius: 22.0,
                production_time: 2.6,
            },
            PlayerKind::Spear => UnitStats {
                cost: 65.0,
                hp: 55.0,
                attack: 16.0,
                range: 110.0,
                speed: 48.0,
                atk_cooldown: 1.1,
                radius: 14.0,
                production_time: 1.8,
            },
            PlayerKind::Bomb => UnitStats {
                cost: 130.0,
                hp: 150.0,
                attack: 55.0,
                range: 22.0,
                speed: 38.0,
                atk_cooldown: 1.8,
                radius: 18.0,
                production_time: 3.4,
            },
        }
    }

    fn name(self) -> &'static str {
        match self {
            PlayerKind::Basic => "Ratling",
            PlayerKind::Tank => "Tank Rat",
            PlayerKind::Spear => "Spear Rat",
            PlayerKind::Bomb => "Bomb Rat",
        }
    }

    fn color(self) -> Color {
        match self {
            PlayerKind::Basic => Color::from_rgba(120, 170, 255, 255),
            PlayerKind::Tank => Color::from_rgba(90, 110, 200, 255),
            PlayerKind::Spear => Color::from_rgba(150, 220, 255, 255),
            PlayerKind::Bomb => Color::from_rgba(80, 60, 180, 255),
        }
    }
}

impl EnemyKind {
    fn stats(self, wave_scale: f32) -> UnitStats {
        // wave_scale slowly buffs enemies as the match goes on.
        let base = match self {
            EnemyKind::Grunt => UnitStats {
                cost: 0.0,
                hp: 60.0,
                attack: 10.0,
                range: 16.0,
                speed: 46.0,
                atk_cooldown: 0.9,
                radius: 15.0,
                production_time: 0.0,
            },
            EnemyKind::Big => UnitStats {
                cost: 0.0,
                hp: 220.0,
                attack: 18.0,
                range: 18.0,
                speed: 26.0,
                atk_cooldown: 1.1,
                radius: 21.0,
                production_time: 0.0,
            },
            EnemyKind::Runner => UnitStats {
                cost: 0.0,
                hp: 35.0,
                attack: 8.0,
                range: 14.0,
                speed: 90.0,
                atk_cooldown: 0.7,
                radius: 12.0,
                production_time: 0.0,
            },
            EnemyKind::Boss => UnitStats {
                cost: 0.0,
                hp: 700.0,
                attack: 40.0,
                range: 24.0,
                speed: 22.0,
                atk_cooldown: 1.3,
                radius: 30.0,
                production_time: 0.0,
            },
        };
        UnitStats {
            hp: base.hp * wave_scale,
            attack: base.attack * (1.0 + (wave_scale - 1.0) * 0.6),
            ..base
        }
    }

    fn color(self) -> Color {
        match self {
            EnemyKind::Grunt => Color::from_rgba(220, 90, 90, 255),
            EnemyKind::Big => Color::from_rgba(170, 40, 40, 255),
            EnemyKind::Runner => Color::from_rgba(240, 150, 60, 255),
            EnemyKind::Boss => Color::from_rgba(120, 20, 130, 255),
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime unit instance
// ---------------------------------------------------------------------------
struct Unit {
    team: Team,
    label: &'static str,
    x: f32,
    hp: f32,
    max_hp: f32,
    attack: f32,
    range: f32,
    speed: f32,
    atk_cooldown: f32,
    atk_timer: f32,
    radius: f32,
    color: Color,
    shape: Shape,
    hit_flash: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Circle,
    Square,
    Triangle,
    Diamond,
}

impl Unit {
    fn from_player(kind: PlayerKind, x: f32) -> Self {
        let s = kind.stats();
        let shape = match kind {
            PlayerKind::Basic => Shape::Circle,
            PlayerKind::Tank => Shape::Square,
            PlayerKind::Spear => Shape::Triangle,
            PlayerKind::Bomb => Shape::Diamond,
        };
        Unit {
            team: Team::Player,
            label: kind.name(),
            x,
            hp: s.hp,
            max_hp: s.hp,
            attack: s.attack,
            range: s.range,
            speed: s.speed,
            atk_cooldown: s.atk_cooldown,
            atk_timer: 0.0,
            radius: s.radius,
            color: kind.color(),
            shape,
            hit_flash: 0.0,
        }
    }

    fn from_enemy(kind: EnemyKind, x: f32, wave_scale: f32) -> Self {
        let s = kind.stats(wave_scale);
        let shape = match kind {
            EnemyKind::Grunt => Shape::Circle,
            EnemyKind::Big => Shape::Square,
            EnemyKind::Runner => Shape::Triangle,
            EnemyKind::Boss => Shape::Diamond,
        };
        let label = match kind {
            EnemyKind::Grunt => "Grub",
            EnemyKind::Big => "Beetle",
            EnemyKind::Runner => "Roach",
            EnemyKind::Boss => "BOSS",
        };
        Unit {
            team: Team::Enemy,
            label,
            x,
            hp: s.hp,
            max_hp: s.hp,
            attack: s.attack,
            range: s.range,
            speed: s.speed,
            atk_cooldown: s.atk_cooldown,
            atk_timer: 0.0,
            radius: s.radius,
            color: kind.color(),
            shape,
            hit_flash: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Deploy button (bottom UI)
// ---------------------------------------------------------------------------
struct DeployButton {
    kind: PlayerKind,
    rect: Rect,
    cooldown: f32,
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------
struct Game {
    units: Vec<Unit>,
    money: f32,
    player_base_hp: f32,
    enemy_base_hp: f32,
    enemy_spawn_timer: f32,
    elapsed: f32,
    buttons: Vec<DeployButton>,
    game_over: Option<bool>, // Some(true)=win, Some(false)=lose
    floating_texts: Vec<FloatingText>,
    shake: f32,
}

struct FloatingText {
    x: f32,
    y: f32,
    text: String,
    life: f32,
    color: Color,
}

impl Game {
    fn new() -> Self {
        let buttons = vec![
            DeployButton { kind: PlayerKind::Basic, rect: Rect::new(0.0, 0.0, 0.0, 0.0), cooldown: 0.0 },
            DeployButton { kind: PlayerKind::Tank, rect: Rect::new(0.0, 0.0, 0.0, 0.0), cooldown: 0.0 },
            DeployButton { kind: PlayerKind::Spear, rect: Rect::new(0.0, 0.0, 0.0, 0.0), cooldown: 0.0 },
            DeployButton { kind: PlayerKind::Bomb, rect: Rect::new(0.0, 0.0, 0.0, 0.0), cooldown: 0.0 },
        ];
        Game {
            units: Vec::new(),
            money: STARTING_MONEY,
            player_base_hp: BASE_MAX_HP,
            enemy_base_hp: BASE_MAX_HP,
            enemy_spawn_timer: 2.5,
            elapsed: 0.0,
            buttons,
            game_over: None,
            floating_texts: Vec::new(),
            shake: 0.0,
        }
    }

    fn lane_y(&self) -> f32 {
        screen_height() * LANE_Y_FRAC
    }

    fn player_base_x(&self) -> f32 {
        BASE_MARGIN
    }

    fn enemy_base_x(&self) -> f32 {
        screen_width() - BASE_MARGIN
    }

    fn wave_scale(&self) -> f32 {
        1.0 + self.elapsed / 45.0
    }

    fn layout_buttons(&mut self) {
        let w = screen_width();
        let h = screen_height();
        let btn_w = 118.0;
        let btn_h = 78.0;
        let gap = 14.0;
        let total_w = btn_w * 4.0 + gap * 3.0;
        let start_x = (w - total_w) / 2.0;
        let y = h - btn_h - 16.0;
        let kinds = [PlayerKind::Basic, PlayerKind::Tank, PlayerKind::Spear, PlayerKind::Bomb];
        for (i, btn) in self.buttons.iter_mut().enumerate() {
            btn.kind = kinds[i];
            btn.rect = Rect::new(start_x + i as f32 * (btn_w + gap), y, btn_w, btn_h);
        }
    }

    fn reset(&mut self) {
        *self = Game::new();
    }

    fn update(&mut self, dt: f32) {
        if self.game_over.is_some() {
            if is_key_pressed(KeyCode::Space) {
                self.reset();
            }
            return;
        }

        self.elapsed += dt;
        self.money += INCOME_PER_SEC * dt;
        self.shake = (self.shake - dt * 3.0).max(0.0);

        for btn in self.buttons.iter_mut() {
            btn.cooldown = (btn.cooldown - dt).max(0.0);
        }

        // ---- input: deploy buttons ----
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let spawn_x = self.player_base_x() + 30.0;
            let mut money = self.money;
            let mut to_spawn: Option<PlayerKind> = None;
            for btn in self.buttons.iter_mut() {
                if btn.rect.contains(vec2(mx, my)) {
                    let stats = btn.kind.stats();
                    if btn.cooldown <= 0.0 && money >= stats.cost {
                        money -= stats.cost;
                        btn.cooldown = stats.production_time;
                        to_spawn = Some(btn.kind);
                    }
                }
            }
            self.money = money;
            if let Some(kind) = to_spawn {
                self.units.push(Unit::from_player(kind, spawn_x));
            }
        }

        // ---- enemy spawning ----
        self.enemy_spawn_timer -= dt;
        if self.enemy_spawn_timer <= 0.0 {
            let scale = self.wave_scale();
            let roll: f32 = gen_range(0.0, 1.0);
            let kind = if self.elapsed > 20.0 && roll < 0.06 {
                EnemyKind::Boss
            } else if roll < 0.45 {
                EnemyKind::Grunt
            } else if roll < 0.7 {
                EnemyKind::Runner
            } else {
                EnemyKind::Big
            };
            self.units.push(Unit::from_enemy(kind, self.enemy_base_x() - 30.0, scale));
            let base_interval = (1.9 - self.elapsed * 0.015).max(0.55);
            self.enemy_spawn_timer = base_interval + gen_range(-0.2, 0.2);
        }

        // ---- movement & combat ----
        let n = self.units.len();
        let mut damage: Vec<f32> = vec![0.0; n];
        let mut base_damage_player = 0.0f32;
        let mut base_damage_enemy = 0.0f32;

        for i in 0..n {
            let (dir, is_player) = match self.units[i].team {
                Team::Player => (1.0, true),
                Team::Enemy => (-1.0, false),
            };

            // find nearest opposing unit ahead of this unit
            let mut nearest_dist = f32::MAX;
            let mut nearest_idx: Option<usize> = None;
            for j in 0..n {
                if i == j { continue; }
                if self.units[j].team == self.units[i].team { continue; }
                let dx = self.units[j].x - self.units[i].x;
                let ahead = if is_player { dx > 0.0 } else { dx < 0.0 };
                if !ahead { continue; }
                let d = dx.abs();
                if d < nearest_dist {
                    nearest_dist = d;
                    nearest_idx = Some(j);
                }
            }

            let atk_range = self.units[i].range + self.units[i].radius
                + nearest_idx.map(|j| self.units[j].radius).unwrap_or(0.0);

            if let Some(j) = nearest_idx {
                if nearest_dist <= atk_range {
                    // fight
                    if self.units[i].atk_timer <= 0.0 {
                        damage[j] += self.units[i].attack;
                        self.units[i].atk_timer = self.units[i].atk_cooldown;
                    }
                    continue; // don't move while fighting
                }
            }

            // otherwise check base collision
            let base_x = if is_player { self.enemy_base_x() } else { self.player_base_x() };
            let dist_to_base = (base_x - self.units[i].x).abs();
            if dist_to_base <= self.units[i].range + 20.0 {
                if self.units[i].atk_timer <= 0.0 {
                    if is_player {
                        base_damage_enemy += self.units[i].attack;
                    } else {
                        base_damage_player += self.units[i].attack;
                    }
                    self.units[i].atk_timer = self.units[i].atk_cooldown;
                }
                continue;
            }

            // move
            self.units[i].x += dir * self.units[i].speed * dt;
        }

        for (i, u) in self.units.iter_mut().enumerate() {
            u.atk_timer = (u.atk_timer - dt).max(0.0);
            u.hit_flash = (u.hit_flash - dt * 4.0).max(0.0);
            if damage[i] > 0.0 {
                u.hp -= damage[i];
                u.hit_flash = 1.0;
            }
        }

        self.enemy_base_hp = (self.enemy_base_hp - base_damage_enemy).max(0.0);
        self.player_base_hp = (self.player_base_hp - base_damage_player).max(0.0);
        if base_damage_enemy > 0.0 || base_damage_player > 0.0 {
            self.shake = 1.0;
        }

        // floating damage-ish text for base hits
        if base_damage_enemy > 0.0 {
            self.floating_texts.push(FloatingText {
                x: self.enemy_base_x(),
                y: self.lane_y() - 60.0,
                text: format!("-{:.0}", base_damage_enemy),
                life: 1.0,
                color: RED,
            });
        }
        if base_damage_player > 0.0 {
            self.floating_texts.push(FloatingText {
                x: self.player_base_x(),
                y: self.lane_y() - 60.0,
                text: format!("-{:.0}", base_damage_player),
                life: 1.0,
                color: RED,
            });
        }
        for ft in self.floating_texts.iter_mut() {
            ft.y -= dt * 30.0;
            ft.life -= dt;
        }
        self.floating_texts.retain(|f| f.life > 0.0);

        // remove dead units, reward money for enemy kills
        let mut reward = 0.0;
        self.units.retain(|u| {
            if u.hp <= 0.0 {
                if u.team == Team::Enemy {
                    reward += 8.0 + u.max_hp * 0.05;
                }
                false
            } else {
                true
            }
        });
        self.money += reward;

        if self.enemy_base_hp <= 0.0 {
            self.game_over = Some(true);
        } else if self.player_base_hp <= 0.0 {
            self.game_over = Some(false);
        }
    }

    // -----------------------------------------------------------------
    // Drawing
    // -----------------------------------------------------------------
    fn draw(&self) {
        let w = screen_width();
        let h = screen_height();
        let shake_x = if self.shake > 0.0 { gen_range(-4.0, 4.0) * self.shake } else { 0.0 };

        // sky
        draw_rectangle(0.0, 0.0, w, h, Color::from_rgba(135, 196, 235, 255));
        // distant hill band
        draw_rectangle(0.0, h * 0.4, w, h * 0.25, Color::from_rgba(110, 180, 120, 255));
        // ground
        let lane_y = self.lane_y();
        draw_rectangle(0.0, lane_y - 10.0, w, h - (lane_y - 10.0), Color::from_rgba(96, 160, 90, 255));
        draw_rectangle(0.0, lane_y + 34.0, w, h - (lane_y + 34.0), Color::from_rgba(150, 111, 66, 255));

        // bases
        self.draw_base(self.player_base_x() + shake_x, lane_y, Color::from_rgba(70, 110, 220, 255), self.player_base_hp, "HOME");
        self.draw_base(self.enemy_base_x() + shake_x, lane_y, Color::from_rgba(190, 60, 60, 255), self.enemy_base_hp, "ENEMY");

        // units
        for u in &self.units {
            self.draw_unit(u, shake_x);
        }

        // floating texts
        for ft in &self.floating_texts {
            let a = (ft.life).clamp(0.0, 1.0);
            let c = Color::new(ft.color.r, ft.color.g, ft.color.b, a);
            draw_text(&ft.text, ft.x - 10.0, ft.y, 26.0, c);
        }

        self.draw_hud();

        if let Some(win) = self.game_over {
            self.draw_game_over(win);
        }
    }

    fn draw_base(&self, x: f32, lane_y: f32, color: Color, hp: f32, label: &str) {
        let bh = 90.0;
        draw_rectangle(x - 26.0, lane_y - bh, 52.0, bh, color);
        draw_rectangle_lines(x - 26.0, lane_y - bh, 52.0, bh, 3.0, BLACK);
        draw_triangle(
            vec2(x - 30.0, lane_y - bh),
            vec2(x + 30.0, lane_y - bh),
            vec2(x, lane_y - bh - 26.0),
            color,
        );
        draw_text(label, x - 24.0, lane_y - bh - 32.0, 18.0, DARKBROWN);
        // hp bar
        let frac = (hp / BASE_MAX_HP).clamp(0.0, 1.0);
        let bar_w = 70.0;
        draw_rectangle(x - bar_w / 2.0, lane_y - bh - 14.0, bar_w, 8.0, Color::from_rgba(40, 40, 40, 220));
        draw_rectangle(x - bar_w / 2.0, lane_y - bh - 14.0, bar_w * frac, 8.0, GREEN);
        draw_rectangle_lines(x - bar_w / 2.0, lane_y - bh - 14.0, bar_w, 8.0, 1.5, BLACK);
    }

    fn draw_unit(&self, u: &Unit, shake_x: f32) {
        let x = u.x + shake_x;
        let y = self.lane_y();
        let flash = u.hit_flash > 0.0;
        let col = if flash {
            Color::new(1.0, 1.0, 1.0, 1.0)
        } else {
            u.color
        };

        match u.shape {
            Shape::Circle => draw_circle(x, y - u.radius, u.radius, col),
            Shape::Square => draw_rectangle(x - u.radius, y - u.radius * 2.0, u.radius * 2.0, u.radius * 2.0, col),
            Shape::Triangle => draw_triangle(
                vec2(x - u.radius, y),
                vec2(x + u.radius, y),
                vec2(x, y - u.radius * 2.0),
                col,
            ),
            Shape::Diamond => draw_poly(x, y - u.radius, 4, u.radius, 45.0, col),
        }
        // outline so shapes read clearly against similar background colors
        let outline = if u.team == Team::Player { Color::from_rgba(20, 30, 90, 255) } else { Color::from_rgba(80, 10, 10, 255) };
        match u.shape {
            Shape::Circle => draw_circle_lines(x, y - u.radius, u.radius, 2.0, outline),
            Shape::Square => draw_rectangle_lines(x - u.radius, y - u.radius * 2.0, u.radius * 2.0, u.radius * 2.0, 2.0, outline),
            _ => {}
        }

        // simple eyes for personality
        draw_circle(x - u.radius * 0.35, y - u.radius * 1.3, 2.2, BLACK);
        draw_circle(x + u.radius * 0.35, y - u.radius * 1.3, 2.2, BLACK);

        // hp bar
        let bar_w = u.radius * 2.2;
        let frac = (u.hp / u.max_hp).clamp(0.0, 1.0);
        let bar_y = y - u.radius * 2.0 - 12.0;
        draw_rectangle(x - bar_w / 2.0, bar_y, bar_w, 5.0, Color::from_rgba(30, 30, 30, 200));
        let hp_color = if frac > 0.5 { GREEN } else if frac > 0.2 { ORANGE } else { RED };
        draw_rectangle(x - bar_w / 2.0, bar_y, bar_w * frac, 5.0, hp_color);

        // tiny label above the bar (skip when units are packed tight to avoid clutter)
        let dims = measure_text(u.label, None, 12, 1.0);
        draw_text(u.label, x - dims.width / 2.0, bar_y - 4.0, 12.0, Color::from_rgba(20, 20, 20, 200));
    }

    fn draw_hud(&self) {
        let w = screen_width();
        // top bar
        draw_rectangle(0.0, 0.0, w, 46.0, Color::from_rgba(30, 30, 40, 200));
        draw_text(&format!("$ {:.0}", self.money), 16.0, 30.0, 30.0, YELLOW);
        let wave_txt = format!("Time {:.0}s   Threat x{:.1}", self.elapsed, self.wave_scale());
        let dims = measure_text(&wave_txt, None, 22, 1.0);
        draw_text(&wave_txt, w - dims.width - 16.0, 28.0, 22.0, WHITE);

        // buttons
        for btn in &self.buttons {
            let stats = btn.kind.stats();
            let affordable = self.money >= stats.cost;
            let ready = btn.cooldown <= 0.0;
            let bg = if !ready {
                Color::from_rgba(90, 90, 90, 230)
            } else if affordable {
                Color::from_rgba(245, 245, 245, 235)
            } else {
                Color::from_rgba(120, 60, 60, 210)
            };
            draw_rectangle(btn.rect.x, btn.rect.y, btn.rect.w, btn.rect.h, bg);
            draw_rectangle_lines(btn.rect.x, btn.rect.y, btn.rect.w, btn.rect.h, 3.0, BLACK);

            // mini icon
            let icon_cx = btn.rect.x + btn.rect.w / 2.0;
            let icon_cy = btn.rect.y + 24.0;
            match btn.kind {
                PlayerKind::Basic => draw_circle(icon_cx, icon_cy, 13.0, btn.kind.color()),
                PlayerKind::Tank => draw_rectangle(icon_cx - 13.0, icon_cy - 13.0, 26.0, 26.0, btn.kind.color()),
                PlayerKind::Spear => draw_triangle(
                    vec2(icon_cx - 13.0, icon_cy + 10.0),
                    vec2(icon_cx + 13.0, icon_cy + 10.0),
                    vec2(icon_cx, icon_cy - 14.0),
                    btn.kind.color(),
                ),
                PlayerKind::Bomb => draw_poly(icon_cx, icon_cy, 4, 15.0, 45.0, btn.kind.color()),
            }

            draw_text(btn.kind.name(), btn.rect.x + 8.0, btn.rect.y + 52.0, 16.0, BLACK);
            draw_text(&format!("${:.0}", stats.cost), btn.rect.x + 8.0, btn.rect.y + 70.0, 16.0, DARKGRAY);

            if !ready {
                let cd_frac = (btn.cooldown / stats.production_time).clamp(0.0, 1.0);
                draw_rectangle(
                    btn.rect.x,
                    btn.rect.y + btn.rect.h * (1.0 - cd_frac),
                    btn.rect.w,
                    btn.rect.h * cd_frac,
                    Color::from_rgba(0, 0, 0, 140),
                );
            }
        }
    }

    fn draw_game_over(&self, win: bool) {
        let w = screen_width();
        let h = screen_height();
        draw_rectangle(0.0, 0.0, w, h, Color::from_rgba(0, 0, 0, 170));
        let title = if win { "VICTORY!" } else { "BASE DESTROYED" };
        let color = if win { GOLD } else { RED };
        let dims = measure_text(title, None, 60, 1.0);
        draw_text(title, w / 2.0 - dims.width / 2.0, h / 2.0 - 20.0, 60.0, color);
        let sub = "Press SPACE to play again";
        let dims2 = measure_text(sub, None, 26, 1.0);
        draw_text(sub, w / 2.0 - dims2.width / 2.0, h / 2.0 + 30.0, 26.0, WHITE);
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Battle Rats".to_owned(),
        window_width: 1100,
        window_height: 620,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();
    loop {
        clear_background(BLACK);
        game.layout_buttons();
        let dt = get_frame_time().min(0.05); // clamp to avoid huge steps on lag/tab-out
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}
