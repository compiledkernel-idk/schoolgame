use macroquad::prelude::*;
use ::rand::Rng;
use ::rand::thread_rng;
use std::fs;

const PLAYER_SPEED: f32 = 360.0;
const PLAYER_RADIUS: f32 = 15.0;
const DASH_SPEED: f32 = 920.0;
const DASH_TIME: f32 = 0.16;
const DASH_COOLDOWN: f32 = 0.9;

const ENEMY_BASE_SPEED: f32 = 120.0;
const ENEMY_SPAWN_START: f32 = 1.15;
const ENEMY_SPAWN_MIN: f32 = 0.26;

const SHARD_SPAWN_RATE: f32 = 1.1;
const SHARD_RADIUS: f32 = 9.0;

const NEAR_MISS_DIST: f32 = 36.0;
const NEAR_MISS_BONUS: i32 = 3;

const COMBO_TIME: f32 = 2.2;
const COMBO_INC: f32 = 0.1;
const TRAIL_MAX: usize = 42;


const SAVE_FILE: &str = "neon_rush.sav";

#[derive(Clone, Default)]
struct Upgrades {
    speed: u32,
    dash_cd: u32,
    dash_time: u32,
    shard_value: u32,
    magnet: u32,
}

impl Upgrades {
    fn player_speed(&self) -> f32 { PLAYER_SPEED * (1.0 + 0.06 * self.speed as f32) }
    fn dash_cd(&self) -> f32 { DASH_COOLDOWN * (0.88f32).powf(self.dash_cd as f32) }
    fn dash_time(&self) -> f32 { DASH_TIME * (1.0 + 0.08 * self.dash_time as f32) }
    fn shard_currency_bonus(&self) -> i32 { (self.shard_value as i32) * 2 }
    fn magnet_speed(&self) -> f32 { 120.0 + 50.0 * self.magnet as f32 }

    fn cost_speed(&self) -> i32 { 60 + (self.speed as i32) * 45 }
    fn cost_dash_cd(&self) -> i32 { 80 + (self.dash_cd as i32) * 50 }
    fn cost_dash_time(&self) -> i32 { 80 + (self.dash_time as i32) * 50 }
    fn cost_shard_value(&self) -> i32 { 40 + (self.shard_value as i32) * 30 }
    fn cost_magnet(&self) -> i32 { 50 + (self.magnet as i32) * 40 }
}

fn save_to_disk(currency: i32, upgrades: &Upgrades, best: i32) {
    let s = format!(
        "currency={}\nspeed={}\ndash_cd={}\ndash_time={}\nshard_value={}\nmagnet={}\nbest={}\n",
        currency, upgrades.speed, upgrades.dash_cd, upgrades.dash_time, upgrades.shard_value, upgrades.magnet, best
    );
    let _ = fs::write(SAVE_FILE, s);
}

fn load_from_disk() -> Option<(i32, Upgrades, i32)> {
    if let Ok(s) = fs::read_to_string(SAVE_FILE) {
        let mut cur = 0;
        let mut up = Upgrades::default();
        let mut best = 0;
        for line in s.lines() {
            let mut it = line.splitn(2, '=');
            if let (Some(k), Some(v)) = (it.next(), it.next()) {
                match k.trim() {
                    "currency" => cur = v.trim().parse().unwrap_or(0),
                    "speed" => up.speed = v.trim().parse().unwrap_or(0),
                    "dash_cd" => up.dash_cd = v.trim().parse().unwrap_or(0),
                    "dash_time" => up.dash_time = v.trim().parse().unwrap_or(0),
                    "shard_value" => up.shard_value = v.trim().parse().unwrap_or(0),
                    "magnet" => up.magnet = v.trim().parse().unwrap_or(0),
                    "best" => best = v.trim().parse().unwrap_or(0),
                    _ => {}
                }
            }
        }
        return Some((cur, up, best));
    }
    None
}


fn window_conf() -> Conf {
    Conf {
        window_title: String::from("Neon Rush — Rust + macroquad"),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        fullscreen: false,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();
    loop {
        let dt = get_frame_time();
        game.handle_input();
        if !game.paused { game.update(dt); }
        game.draw();
        next_frame().await;
    }
}


fn clamp(v: f32, lo: f32, hi: f32) -> f32 { v.max(lo).min(hi) }

fn hsla(h: f32, s: f32, l: f32, a: u8) -> Color {

    let (r, g, b) = hsl_to_rgb(h.fract(), s.clamp(0.0,1.0), l.clamp(0.0,1.0));
    Color::from_rgba((r*255.0) as u8, (g*255.0) as u8, (b*255.0) as u8, a)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 { return (l, l, l); }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    fn t(mut tc: f32, p: f32, q: f32) -> f32 {
        if tc < 0.0 { tc += 1.0; }
        if tc > 1.0 { tc -= 1.0; }
        if tc < 1.0/6.0 { return p + (q-p)*6.0*tc; }
        if tc < 1.0/2.0 { return q; }
        if tc < 2.0/3.0 { return p + (q-p)*(2.0/3.0 - tc)*6.0; }
        p
    }
    (t(h + 1.0/3.0, p, q), t(h, p, q), t(h - 1.0/3.0, p, q))
}


struct Particle { pos: Vec2, vel: Vec2, life: f32, size: f32, color: Color }
impl Particle {
    fn update(&mut self, dt: f32) { self.pos += self.vel * dt; self.life -= dt; self.size = (self.size - 18.0*dt).max(0.0); }
    fn draw(&self, shake: Vec2) { if self.life>0.0 && self.size>0.0 { draw_circle(self.pos.x+shake.x, self.pos.y+shake.y, self.size, self.color); } }
}

struct TextFx { pos: Vec2, vel: Vec2, life: f32, text: String, color: Color }
impl TextFx { fn update(&mut self, dt: f32){ self.pos += self.vel*dt; self.life -= dt; } }

struct Player {
    pos: Vec2,
    vel: Vec2,
    r: f32,
    dash_cd: f32,
    dash_t: f32,
    invuln: f32,
    dashes_left: i32,
    dashes_max: i32,
    trail: Vec<(Vec2, f32)>,
}
impl Player {
    fn new() -> Self { Self { pos: vec2(screen_width()/2.0, screen_height()/2.0), vel: Vec2::ZERO, r: PLAYER_RADIUS, dash_cd: 0.0, dash_t: 0.0, invuln: 0.0, dashes_left: 1, dashes_max: 1, trail: Vec::new() } }
    fn is_dashing(&self) -> bool { self.dash_t > 0.0 }
    fn update(&mut self, dt: f32, move_speed: f32) {
        let left = is_key_down(KeyCode::A) || is_key_down(KeyCode::Left);
        let right= is_key_down(KeyCode::D) || is_key_down(KeyCode::Right);
        let up   = is_key_down(KeyCode::W) || is_key_down(KeyCode::Up);
        let down = is_key_down(KeyCode::S) || is_key_down(KeyCode::Down);
        let mut mv = vec2((right as i32 - left as i32) as f32, (down as i32 - up as i32) as f32);
        if mv.length_squared() > 0.0 { mv = mv.normalize(); }

        if self.is_dashing(){
            self.dash_t -= dt;
        } else {
            self.vel = mv * move_speed;
            if self.dash_cd>0.0 {
                self.dash_cd -= dt;
                if self.dash_cd <= 0.0 { self.dashes_left = self.dashes_max; self.dash_cd = 0.0; }
            }
        }
        self.pos += self.vel * dt;
        self.pos.x = clamp(self.pos.x, self.r, screen_width()-self.r);
        self.pos.y = clamp(self.pos.y, self.r, screen_height()-self.r);
        if self.invuln>0.0 { self.invuln -= dt; }

        self.trail.push((self.pos, 0.35));
        if self.trail.len()>TRAIL_MAX { self.trail.remove(0); }
        for p in &mut self.trail { p.1 -= dt; }
        self.trail.retain(|p| p.1 > 0.0);
    }
    fn try_dash(&mut self, dash_time: f32, dash_cd_total: f32) -> bool {
        if self.dashes_left > 0 && !self.is_dashing(){
            let dir = if self.vel.length_squared()==0.0 { vec2(1.0,0.0) } else { self.vel.normalize() };
            self.vel = dir * DASH_SPEED;
            self.dash_t = dash_time;
            self.invuln = dash_time;
            self.dashes_left -= 1;
            if self.dashes_left == 0 { self.dash_cd = dash_cd_total; }
            return true;
        }
        false
    }
    fn draw(&self, t: f32, shake: Vec2) {

        for (i, a) in [30u8, 60, 100].iter().enumerate(){
            let rad = self.r + (i as f32)*6.0;
            let c = hsla(0.33 + 0.05*(t*2.0).sin(), 0.9, 0.55, *a);
            draw_circle(self.pos.x+shake.x, self.pos.y+shake.y, rad, c);
        }
        let core = hsla(0.55 + 0.25*(t*1.2).sin(), 0.8, 0.55, 255);
        for (p, life) in &self.trail {
            let sz = self.r*0.6 + life*10.0;
            let c = hsla(0.52, 0.9, 0.7, (150.0*life) as u8);
            draw_circle(p.x+shake.x, p.y+shake.y, sz, c);
        }
        draw_circle(self.pos.x+shake.x, self.pos.y+shake.y, self.r, core);
        if self.is_dashing(){
            draw_circle_lines(self.pos.x+shake.x, self.pos.y+shake.y, self.r*1.9, 2.0, hsla(0.52,0.9,0.7,220));
        }
    }
}

struct Enemy { pos: Vec2, kind: i32, r: f32, angle: f32, speed: f32, cool: f32 }
impl Enemy {
    fn new(pos: Vec2, kind: i32) -> Self { Self{ pos, kind, r: if kind!=2 {12.0} else {10.0}, angle: rand_angle(), speed: ENEMY_BASE_SPEED*(1.0+0.15*(kind as f32)), cool: 0.8 } }
    fn update(&mut self, dt: f32, player: &Player, t: f32){
        let v = match self.kind {
            0 => {
                let to_c = (vec2(screen_width()/2.0, screen_height()/2.0) - self.pos) * 0.2;
                let n = vec2((t*1.7 + self.pos.x*0.01).cos(), (t*1.3 + self.pos.y*0.01).sin());
                let sum = to_c + n*120.0; if sum.length_squared()>0.0 { sum.normalize() } else { Vec2::ZERO }
            }
            1 => {
                let mut v = player.pos - self.pos; if v.length_squared()>0.0 { v = v.normalize(); } v
            }
            _ => {
                let offset = vec2(self.angle.cos(), self.angle.sin()) * 120.0;
                let mut v = (player.pos + offset) - self.pos; if v.length_squared()>0.0 { v = v.normalize(); }
                self.angle += 2.5*dt; v
            }
        };
        self.pos += v * self.speed * dt;
    }
    fn draw(&self, t: f32, shake: Vec2){
        let hue = match self.kind {0=>0.03,1=>0.93,_=>0.66};
        let core = hsla(hue + 0.05*(t*2.0).sin(), 0.85, 0.55, 255);
        let ring = hsla(hue, 0.9, 0.75, 255);
        draw_circle_lines(self.pos.x+shake.x, self.pos.y+shake.y, self.r+3.0, 2.0, ring);
        draw_circle(self.pos.x+shake.x, self.pos.y+shake.y, self.r, core);
    }
}

struct Shard { pos: Vec2, r: f32, t: f32 }
impl Shard {
    fn new(pos: Vec2) -> Self { Self { pos, r: SHARD_RADIUS, t: rand_angle() } }
    fn draw(&self, t: f32, shake: Vec2){
        let hue = (0.5 + 0.1*(t*2.0 + self.t).sin()).fract();
        let glow = hsla(hue, 0.8, 0.6, 90);
        draw_circle(self.pos.x+shake.x, self.pos.y+shake.y, self.r*1.9, glow);
        draw_circle_lines(self.pos.x+shake.x, self.pos.y+shake.y, self.r, 2.0, hsla(hue,0.9,0.75,255));
    }
}

fn rand_angle() -> f32 { thread_rng().gen::<f32>() * std::f32::consts::TAU }


struct Star { pos: Vec2, vel: f32, chr: u8, hue: f32 }
impl Star {
    fn update(&mut self, dt: f32){ self.pos.x += self.vel*dt; if self.pos.x>screen_width()+10.0 { self.pos.x = -10.0; self.pos.y = thread_rng().gen_range(0.0..screen_height()); } }
    fn draw(&self){
        let c = hsla(self.hue, 0.3, 0.6, 160);
        draw_circle(self.pos.x, self.pos.y, self.chr as f32 * 0.3 + 0.7, c);
    }
}

struct Bullet { pos: Vec2, vel: Vec2, r: f32, hostile: bool, life: f32 }
#[derive(Copy, Clone)]
enum PowerUpKind { Invuln, Magnet, DoubleDash }
struct PowerUp { pos: Vec2, kind: PowerUpKind, r: f32, spin: f32 }
struct Boss { pos: Vec2, r: f32, timer: f32, phase: f32 }


struct Game {
    player: Player,
    enemies: Vec<Enemy>,
    shards: Vec<Shard>,
    particles: Vec<Particle>,
    textfx: Vec<TextFx>,
    spawn_timer: f32,
    shard_timer: f32,
    score: i32,
    best: i32,
    combo: f32,
    combo_t: f32,
    shake: f32,
    paused: bool,
    over: bool,
    stars: Vec<Star>,
    enemy_rate_boost: f32,
    fullscreen: bool,
    currency: i32,
    upgrades: Upgrades,
    shop_open: bool,
    bullets: Vec<Bullet>,
    powerups: Vec<PowerUp>,
    powerup_timer: f32,
    power_invuln: f32,
    power_magnet: f32,
    power_ddash: f32,
    boss: Option<Boss>,
    next_boss_score: i32,
}

impl Game {
    fn new() -> Self {
        let mut g = Self {
            player: Player::new(),
            enemies: vec![],
            shards: vec![],
            particles: vec![],
            textfx: vec![],
            spawn_timer: ENEMY_SPAWN_START,
            shard_timer: SHARD_SPAWN_RATE,
            score: 0,
            best: 0,
            combo: 0.0,
            combo_t: 0.0,
            shake: 0.0,
            paused: false,
            over: false,
            stars: vec![],
            enemy_rate_boost: 0.0,
            fullscreen: false,
            currency: 0,
            upgrades: Upgrades::default(),
            shop_open: false,
            bullets: vec![],
            powerups: vec![],
            powerup_timer: 6.0,
            power_invuln: 0.0,
            power_magnet: 0.0,
            power_ddash: 0.0,
            boss: None,
            next_boss_score: 200,
        };
        g.init_stars();
        if let Some((cur, ups, best)) = load_from_disk() {
            g.currency = cur;
            g.upgrades = ups;
            g.best = g.best.max(best);
        }
        g
    }
    fn init_stars(&mut self){
        let mut rng = thread_rng();
        self.stars.clear();
        let n = (screen_width()*screen_height()/15000.0).max(40.0) as usize;
        for _ in 0..n { self.stars.push(Star{ pos: vec2(rng.gen_range(0.0..screen_width()), rng.gen_range(0.0..screen_height())), vel: rng.gen_range(40.0..140.0), chr: rng.gen_range(1..4), hue: rng.gen::<f32>() }); }
    }

    fn reset_round(&mut self){
        self.player = Player::new();
        self.enemies.clear();
        self.shards.clear();
        self.particles.clear();
        self.textfx.clear();
        self.spawn_timer = ENEMY_SPAWN_START;
        self.shard_timer = SHARD_SPAWN_RATE;
        self.score = 0;
        self.combo = 0.0;
        self.combo_t = 0.0;
        self.shake = 0.0;
        self.paused = false;
        self.over = false;
        self.enemy_rate_boost = 0.0;
        self.bullets.clear();
        self.powerups.clear();
        self.powerup_timer = 6.0;
        self.power_invuln = 0.0;
        self.power_magnet = 0.0;
        self.power_ddash = 0.0;
        self.boss = None;
        self.init_stars();
    }

    fn handle_input(&mut self){
        if is_key_pressed(KeyCode::Escape) { save_to_disk(self.currency, &self.upgrades, self.best); std::process::exit(0); }
        if is_key_pressed(KeyCode::P) { self.paused = !self.paused; }
        if is_key_pressed(KeyCode::R) { self.reset_round(); }
        if !self.shop_open && !self.over && is_key_pressed(KeyCode::Space) {
            if self.player.try_dash(self.upgrades.dash_time(), self.upgrades.dash_cd()){
                self.add_particles(self.player.pos, hsla(0.52,0.8,0.7,200), 40, 400.0);
                self.shake = self.shake.max(10.0);
            }
        }

        if is_key_pressed(KeyCode::F11) {
            self.fullscreen = !self.fullscreen;
            set_fullscreen(self.fullscreen);
            self.init_stars();
        }

        if is_key_pressed(KeyCode::F6) { request_new_screen_size(1280.0, 720.0); self.init_stars(); }
        if is_key_pressed(KeyCode::F7) { request_new_screen_size(1600.0, 900.0); self.init_stars(); }
        if is_key_pressed(KeyCode::F8) { request_new_screen_size(1920.0,1080.0); self.init_stars(); }
        if is_key_pressed(KeyCode::F9) { request_new_screen_size(2560.0,1440.0); self.init_stars(); }


        if !self.over && is_key_pressed(KeyCode::U) {
            self.shop_open = !self.shop_open;
            self.paused = self.shop_open;
        }
        if !self.over && self.shop_open {
            if is_key_pressed(KeyCode::Key1) { self.try_buy(1); }
            if is_key_pressed(KeyCode::Key2) { self.try_buy(2); }
            if is_key_pressed(KeyCode::Key3) { self.try_buy(3); }
            if is_key_pressed(KeyCode::Key4) { self.try_buy(4); }
            if is_key_pressed(KeyCode::Key5) { self.try_buy(5); }
        }
    }

    fn update(&mut self, dt: f32){

        if self.over {
            for s in &mut self.stars { s.update(dt); }
            for p in &mut self.particles { p.update(dt); }
            self.particles.retain(|p| p.life>0.0 && p.size>0.0);
            for t in &mut self.textfx { t.update(dt); }
            self.textfx.retain(|t| t.life>0.0);
            self.shake = (self.shake - dt*18.0).max(0.0);
            return;
        }


        if self.power_invuln > 0.0 { self.power_invuln -= dt; }
        if self.power_magnet > 0.0 { self.power_magnet -= dt; }
        if self.power_ddash > 0.0 { self.power_ddash -= dt; }
        self.player.dashes_max = if self.power_ddash > 0.0 { 2 } else { 1 };

        self.player.update(dt, self.upgrades.player_speed());
        for s in &mut self.stars { s.update(dt); }

        self.enemy_rate_boost += dt*0.03;

        self.spawn_timer -= dt;
        let rate = (ENEMY_SPAWN_START - self.enemy_rate_boost).max(ENEMY_SPAWN_MIN);
        if self.spawn_timer <= 0.0 { self.spawn_enemy(); self.spawn_timer = rate; }


        if self.boss.is_none() && self.score >= self.next_boss_score {
            self.boss = Some(Boss{ pos: vec2(screen_width()*0.5, screen_height()*0.35), r: 46.0, timer: 20.0, phase: 0.0 });
            self.textfx.push(TextFx{ pos: vec2(screen_width()*0.5, 80.0), vel: vec2(0.0,0.0), life: 2.0, text: "BOSS".into(), color: hsla(0.9,0.9,0.8,235)});
        }
        if let Some(mut b) = self.boss.take() {
            b.phase += dt; b.timer -= dt;
            b.pos.x = screen_width()*0.5 + (b.phase*1.2).sin()* (screen_width()*0.35);

            if (b.phase % 1.6) < dt {
                for i in 0..16 {
                    let ang = i as f32 / 16.0 * std::f32::consts::TAU;
                    let v = vec2(ang.cos(), ang.sin()) * 240.0;
                    self.bullets.push(Bullet{ pos: b.pos, vel: v, r: 6.0, hostile: true, life: 6.0 });
                }
            }

            if (b.phase % 0.6) < dt {
                let mut dir = self.player.pos - b.pos; if dir.length_squared()>0.0 { dir = dir.normalize(); }
                self.bullets.push(Bullet{ pos: b.pos, vel: dir * 420.0, r: 5.0, hostile: true, life: 5.0 });
            }

            let player_hit_boss = self.player.pos.distance(b.pos) <= self.player.r + b.r
                && !(self.player.invuln>0.0 || self.power_invuln>0.0);
            if player_hit_boss { self.game_over(); }

            if b.timer <= 0.0 {
                self.textfx.push(TextFx{ pos: b.pos, vel: vec2(0.0,-40.0), life: 1.2, text: "BOSS CLEARED".into(), color: hsla(0.33,0.9,0.8,235)});
                self.currency += 50; self.bump_score(50);
                self.next_boss_score += 250; // do not restore boss -> it despawns
            } else {
                self.boss = Some(b);
            }
        }

        self.shard_timer -= dt;
        if self.shard_timer <= 0.0 { self.spawn_shard(); self.shard_timer = SHARD_SPAWN_RATE; }


        self.powerup_timer -= dt;
        if self.powerup_timer <= 0.0 {
            let k = if thread_rng().gen::<f32>() < 0.34 { PowerUpKind::Invuln } else if thread_rng().gen::<f32>() < 0.5 { PowerUpKind::Magnet } else { PowerUpKind::DoubleDash };
            let pos = vec2(thread_rng().gen_range(40.0..(screen_width()-40.0)), thread_rng().gen_range(40.0..(screen_height()-40.0)));
            self.powerups.push(PowerUp{ pos, kind: k, r: 12.0, spin: rand_angle() });
            self.powerup_timer = thread_rng().gen_range(7.0..13.0);
        }

        for e in &mut self.enemies { e.update(dt, &self.player, get_time() as f32); }


        for e in &mut self.enemies {
            if e.kind == 4 {
                e.cool -= dt;
                if e.cool <= 0.0 {
                    let mut dir = self.player.pos - e.pos; if dir.length_squared()>0.0 { dir = dir.normalize(); }
                    self.bullets.push(Bullet{ pos: e.pos, vel: dir * 360.0, r: 5.0, hostile: true, life: 5.0 });
                    e.cool = thread_rng().gen_range(0.9..1.6);
                }
            }
        }

        let pr = self.player.r; let ppos = self.player.pos;

        for e in &self.enemies {
            let d = ppos.distance(e.pos);
            if d>pr && d<NEAR_MISS_DIST && self.player.invuln<=0.0 {
                if thread_rng().gen::<f32>() < 0.02 {
                    self.score += NEAR_MISS_BONUS;
                    self.currency += 1;
                    self.textfx.push(TextFx{ pos: e.pos, vel: vec2(0.0,-40.0), life: 0.8, text: String::from("near!"), color: hsla(0.1,0.9,0.7,235) });
                }
            }
        }

        let mut keep: Vec<Enemy> = Vec::with_capacity(self.enemies.len());
        let mut hit_player = false;
        let drained_enemies: Vec<Enemy> = self.enemies.drain(..).collect();
        for e in drained_enemies {
            let d = ppos.distance(e.pos);
            if d <= pr + e.r {
                if self.player.invuln>0.0 {
                    self.add_particles(e.pos, hsla(0.96,0.9,0.7,220), 32, 360.0);
                    self.bump_score(10);
                    self.currency += 2;
                    if thread_rng().gen::<f32>() < 0.5 { self.shards.push(Shard::new(e.pos)); }
                } else {
                    hit_player = true;
                }
            } else { keep.push(e); }
        }
        self.enemies = keep;
        if hit_player { self.game_over(); }

        let mut kept: Vec<Shard> = Vec::with_capacity(self.shards.len());
        let drained_shards: Vec<Shard> = self.shards.drain(..).collect();
        for sh in drained_shards {
            if ppos.distance(sh.pos) <= pr + sh.r {
                let bonus = (5.0 * (1.0+self.combo)) as i32;
                self.bump_score(bonus);
                let cadd = 3 + self.upgrades.shard_currency_bonus();
                self.currency += cadd;
                self.textfx.push(TextFx{ pos: sh.pos, vel: vec2(0.0,-40.0), life: 0.8, text: format!("+{}", bonus), color: hsla(0.55,0.9,0.8,235)});
                self.add_particles(sh.pos, hsla(0.55,0.9,0.7,200), 22, 280.0);
            } else {
                let mut s = sh;
                let d = ppos - s.pos; let dist2 = d.length_squared();
                let pull = self.upgrades.magnet_speed() + if self.power_magnet>0.0 { 220.0 } else { 0.0 };
                let radius = if self.power_magnet>0.0 { 260.0 } else { 180.0 };
                if dist2 < radius*radius && dist2>0.0 { s.pos += d.normalize()* (pull*dt); }
                kept.push(s);
            }
        }
        self.shards = kept;


        let drained_powerups: Vec<PowerUp> = self.powerups.drain(..).collect();
        let mut kept_pu: Vec<PowerUp> = Vec::with_capacity(drained_powerups.len());
        for pu in drained_powerups {
            if self.player.pos.distance(pu.pos) <= self.player.r + pu.r {
                match pu.kind {
                    PowerUpKind::Invuln => { self.power_invuln = 5.0; self.textfx.push(TextFx{ pos: pu.pos, vel: vec2(0.0,-40.0), life: 0.9, text: "INVULN".into(), color: hsla(0.14,0.9,0.8,235)}); }
                    PowerUpKind::Magnet => { self.power_magnet = 6.0; self.textfx.push(TextFx{ pos: pu.pos, vel: vec2(0.0,-40.0), life: 0.9, text: "MAGNET".into(), color: hsla(0.58,0.9,0.8,235)}); }
                    PowerUpKind::DoubleDash => { self.power_ddash = 8.0; self.textfx.push(TextFx{ pos: pu.pos, vel: vec2(0.0,-40.0), life: 0.9, text: "DOUBLE DASH".into(), color: hsla(0.33,0.9,0.8,235)}); }
                }
                self.add_particles(pu.pos, hsla(0.52,0.9,0.7,220), 28, 300.0);
            } else {
                kept_pu.push(pu);
            }
        }
        self.powerups = kept_pu;

        if self.combo_t>0.0 { self.combo_t -= dt; if self.combo_t<=0.0 { self.combo = (self.combo-0.5).max(0.0); self.combo_t = 0.0; } }


        let drained_bullets: Vec<Bullet> = self.bullets.drain(..).collect();
        let mut kept_bullets: Vec<Bullet> = Vec::with_capacity(drained_bullets.len());
        for mut b in drained_bullets {
            b.pos += b.vel * dt; b.life -= dt;
            if b.life <= 0.0 { continue; }
            if b.pos.x < -10.0 || b.pos.x > screen_width()+10.0 || b.pos.y < -10.0 || b.pos.y > screen_height()+10.0 { continue; }
            if b.hostile {
                if self.player.pos.distance(b.pos) <= self.player.r + b.r {
                    if self.player.invuln>0.0 || self.power_invuln>0.0 { self.add_particles(b.pos, hsla(0.0,0.0,1.0,180), 10, 180.0); }
                    else { self.game_over(); continue; }
                }
            }
            kept_bullets.push(b);
        }
        self.bullets = kept_bullets;

        for p in &mut self.particles { p.update(dt); }
        self.particles.retain(|p| p.life>0.0 && p.size>0.0);
        for t in &mut self.textfx { t.update(dt); }
        self.textfx.retain(|t| t.life>0.0);
        self.shake = (self.shake - dt*18.0).max(0.0);
    }

    fn draw(&self){

        clear_background(Color::from_rgba(6, 8, 20, 255));
        let t = get_time() as f32;
        for i in 0..8 { let y = ((t*0.3 + i as f32).sin()*0.5+0.5) * screen_height(); draw_rectangle(0.0, y, screen_width(), 14.0, hsla(0.66 - i as f32*0.04, 0.25, 0.06, 40)); }
        for s in &self.stars { s.draw(); }

        let sv = if self.shake>0.0 { vec2(rand_f(-self.shake, self.shake), rand_f(-self.shake, self.shake)) } else { Vec2::ZERO };

        for sh in &self.shards { sh.draw(t, sv); }
        for e in &self.enemies { e.draw(t, sv); }
        self.player.draw(t, sv);
        for p in &self.particles { p.draw(sv); }
        for tf in &self.textfx { draw_text(&tf.text, tf.pos.x+sv.x, tf.pos.y+sv.y, 24.0, tf.color); }

        for pu in &self.powerups {
            let col = match pu.kind { PowerUpKind::Invuln => hsla(0.14,0.9,0.7,220), PowerUpKind::Magnet => hsla(0.58,0.9,0.7,220), PowerUpKind::DoubleDash => hsla(0.33,0.9,0.7,220) };
            draw_circle(pu.pos.x+sv.x, pu.pos.y+sv.y, pu.r*1.8, Color::from_rgba(255,255,255,30));
            draw_circle_lines(pu.pos.x+sv.x, pu.pos.y+sv.y, pu.r, 2.0, col);
            draw_circle(pu.pos.x+sv.x, pu.pos.y+sv.y, pu.r*0.6, col);
        }
        for b in &self.bullets { draw_circle(b.pos.x+sv.x, b.pos.y+sv.y, b.r, hsla(0.95,0.9,0.7,235)); }
        if let Some(b) = &self.boss {
            draw_circle(b.pos.x+sv.x, b.pos.y+sv.y, b.r*1.8, Color::from_rgba(255,140,160,30));
            draw_circle_lines(b.pos.x+sv.x, b.pos.y+sv.y, b.r, 3.0, hsla(0.93,0.9,0.7,235));
            draw_circle(b.pos.x+sv.x, b.pos.y+sv.y, b.r*0.6, hsla(0.93,0.7,0.6,235));
        }

        let hud = format!(
            "Credits: {}   Score: {}   Combo: x{:.1}   [INV {:.0}s] [MAG {:.0}s] [DD {:.0}s]",
            self.currency, self.score, 1.0+self.combo,
            self.power_invuln.max(0.0), self.power_magnet.max(0.0), self.power_ddash.max(0.0)
        );
        draw_text(&hud, 16.0, 24.0, 28.0, Color::from_rgba(230,240,250,255));

        if self.player.dash_cd>0.0 {
            let w = 160.0; let x = screen_width()-24.0-w; let y = 16.0;
            draw_rectangle_lines(x, y, w, 10.0, 1.0, Color::from_rgba(70,90,120,220));
            let cdw = w * (1.0 - (self.player.dash_cd / self.upgrades.dash_cd()).clamp(0.0,1.0));
            draw_rectangle(x, y, cdw, 10.0, Color::from_rgba(120,200,255,255));
            draw_text("dash", x, y+20.0, 20.0, Color::from_rgba(150,200,255,220));
        }

        if self.paused { self.center_msg("Paused — press P to resume", Color::from_rgba(220,220,240,255)); }
        if self.over { self.center_msg(&format!("Game Over  •  Score {}  •  Best {}\nPress R to restart", self.score, self.best), Color::from_rgba(250,210,210,255)); }
        if self.shop_open { self.draw_shop(); }
    }

    fn center_msg(&self, text: &str, color: Color){
        let lines: Vec<&str> = text.split('\n').collect();
        let total_h = lines.len() as f32 * 64.0 + (lines.len().saturating_sub(1) as f32)*8.0;
        let mut y = screen_height()/2.0 - total_h/2.0;
        for line in lines {
            let m = measure_text(line, None, 64, 1.0);
            draw_text(line, screen_width()/2.0 - m.width/2.0, y, 64.0, color);
            y += 72.0;
        }
    }

    fn add_particles(&mut self, pos: Vec2, color: Color, amount: usize, speed: f32){
        let mut rng = thread_rng();
        for _ in 0..amount {
            let ang = rng.gen::<f32>() * std::f32::consts::TAU;
            let sp  = rng.gen::<f32>() * speed;
            let vel = vec2(ang.cos(), ang.sin()) * sp;
            self.particles.push(Particle{ pos, vel, life: 0.6 + rng.gen::<f32>()*0.6, size: 6.0 + rng.gen::<f32>()*6.0, color });
        }
    }

    fn spawn_enemy(&mut self){
        let mut rng = thread_rng();
        let side = rng.gen_range(0..4);
        let m = 24.0;
        let pos = match side {
            0 => vec2(rng.gen_range(0.0..screen_width()), -m),
            1 => vec2(rng.gen_range(0.0..screen_width()), screen_height()+m),
            2 => vec2(-m, rng.gen_range(0.0..screen_height())),
            _ => vec2(screen_width()+m, rng.gen_range(0.0..screen_height())),
        };
        let kind = rng.gen_range(0..=2);
        self.enemies.push(Enemy::new(pos, kind));
    }

    fn spawn_shard(&mut self){
        let mut rng = thread_rng();
        let pos = vec2(
            rng.gen_range(40.0..(screen_width() - 40.0)),
            rng.gen_range(40.0..(screen_height() - 40.0)),
        );
        self.shards.push(Shard::new(pos));
    }

    fn bump_score(&mut self, base: i32){
        self.score += base;
        self.combo += COMBO_INC; self.combo_t = COMBO_TIME;
    }

    fn game_over(&mut self){
        self.shake = 20.0; self.add_particles(self.player.pos, hsla(0.0,0.9,0.7,230), 80, 420.0);
        self.best = self.best.max(self.score); self.over = true;
        self.shop_open = false;
        self.paused = false;
        save_to_disk(self.currency, &self.upgrades, self.best);
    }

    fn try_buy(&mut self, idx: u32){
        let (name, cost, apply, effect_text) = match idx {
            1 => ("Speed", self.upgrades.cost_speed(), 0, "+6% move"),
            2 => ("Dash CD", self.upgrades.cost_dash_cd(), 1, "-12% cooldown"),
            3 => ("Dash Time", self.upgrades.cost_dash_time(), 2, "+8% duration"),
            4 => ("Shard Value", self.upgrades.cost_shard_value(), 3, "+2 credits/shard"),
            5 => ("Magnet", self.upgrades.cost_magnet(), 4, "+50 pull speed"),
            _ => return,
        };
        if self.currency < cost { return; }
        self.currency -= cost;
        match apply {
            0 => { self.upgrades.speed += 1; },
            1 => { self.upgrades.dash_cd += 1; },
            2 => { self.upgrades.dash_time += 1; },
            3 => { self.upgrades.shard_value += 1; },
            4 => { self.upgrades.magnet += 1; },
            _ => {}
        }
        self.textfx.push(TextFx{ pos: self.player.pos, vel: vec2(0.0,-40.0), life: 0.9, text: format!("{}! {}", name, effect_text), color: hsla(0.33,0.9,0.8,235)});
        save_to_disk(self.currency, &self.upgrades, self.best);
    }

    fn draw_shop(&self){
        let x = screen_width()*0.5 - 360.0;
        let y = 80.0;
        let w = 720.0;
        let h = 420.0;
        draw_rectangle(x, y, w, h, Color::from_rgba(20, 24, 44, 240));
        draw_rectangle_lines(x, y, w, h, 2.0, Color::from_rgba(120, 150, 200, 200));
        draw_text("UPGRADES — press 1–5 to buy, U to close", x+20.0, y+40.0, 28.0, Color::from_rgba(230,240,250,255));
        let mut yy = y + 90.0;
        let line_h = 44.0;
        let white = Color::from_rgba(230,240,250,255);
        let grey = Color::from_rgba(160,180,210,220);

        let lines = [
            format!("1) Speed (lvl {})  — cost {}  — +6% move", self.upgrades.speed, self.upgrades.cost_speed()),
            format!("2) Dash Cooldown (lvl {})  — cost {}  — -12% cooldown", self.upgrades.dash_cd, self.upgrades.cost_dash_cd()),
            format!("3) Dash Duration (lvl {})  — cost {}  — +8% duration", self.upgrades.dash_time, self.upgrades.cost_dash_time()),
            format!("4) Shard Value (lvl {})  — cost {}  — +2 credits/shard", self.upgrades.shard_value, self.upgrades.cost_shard_value()),
            format!("5) Magnet (lvl {})  — cost {}  — +50 pull speed", self.upgrades.magnet, self.upgrades.cost_magnet()),
        ];
        for (i, line) in lines.iter().enumerate() {
            let color = if (i as i32) % 2 == 0 { white } else { grey };
            draw_text(line, x+24.0, yy, 26.0, color);
            yy += line_h;
        }
        draw_text(&format!("Credits: {}", self.currency), x+24.0, y+h-20.0, 26.0, Color::from_rgba(200,230,255,230));
    }
}


fn rand_f(a: f32, b: f32) -> f32 { thread_rng().gen_range(a..b) }