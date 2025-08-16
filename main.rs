use macroquad::prelude::*;
use ::rand::Rng;
use ::rand::thread_rng;


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
    trail: Vec<(Vec2, f32)>, 
}
impl Player {
    fn new() -> Self { Self { pos: vec2(screen_width()/2.0, screen_height()/2.0), vel: Vec2::ZERO, r: PLAYER_RADIUS, dash_cd: 0.0, dash_t: 0.0, invuln: 0.0, trail: Vec::new() } }
    fn is_dashing(&self) -> bool { self.dash_t > 0.0 }
    fn update(&mut self, dt: f32) {
        let left = is_key_down(KeyCode::A) || is_key_down(KeyCode::Left);
        let right= is_key_down(KeyCode::D) || is_key_down(KeyCode::Right);
        let up   = is_key_down(KeyCode::W) || is_key_down(KeyCode::Up);
        let down = is_key_down(KeyCode::S) || is_key_down(KeyCode::Down);
        let mut mv = vec2((right as i32 - left as i32) as f32, (down as i32 - up as i32) as f32);
        if mv.length_squared() > 0.0 { mv = mv.normalize(); }

        if self.is_dashing(){
            self.dash_t -= dt; 
        } else {
            self.vel = mv * PLAYER_SPEED;
            if self.dash_cd>0.0 { self.dash_cd -= dt; }
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
    fn try_dash(&mut self) -> bool {
        if self.dash_cd<=0.0 && !self.is_dashing(){
            let dir = if self.vel.length_squared()==0.0 { vec2(1.0,0.0) } else { self.vel.normalize() };
            self.vel = dir * DASH_SPEED;
            self.dash_t = DASH_TIME;
            self.invuln = DASH_TIME;
            self.dash_cd = DASH_COOLDOWN;
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

struct Enemy { pos: Vec2, kind: i32, r: f32, angle: f32, speed: f32 }
impl Enemy {
    fn new(pos: Vec2, kind: i32) -> Self { Self{ pos, kind, r: if kind!=2 {12.0} else {10.0}, angle: rand_angle(), speed: ENEMY_BASE_SPEED*(1.0+0.15*(kind as f32)) } }
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
        };
        g.init_stars();
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
        self.init_stars();
    }

    fn handle_input(&mut self){
        if is_key_pressed(KeyCode::Escape) { std::process::exit(0); }
        if is_key_pressed(KeyCode::P) { self.paused = !self.paused; }
        if is_key_pressed(KeyCode::R) { self.reset_round(); }
        if !self.over && is_key_pressed(KeyCode::Space) {
            if self.player.try_dash(){
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
    }

    fn update(&mut self, dt: f32){
        self.player.update(dt);
        for s in &mut self.stars { s.update(dt); }

        
        self.enemy_rate_boost += dt*0.03;

        
        self.spawn_timer -= dt;
        let rate = (ENEMY_SPAWN_START - self.enemy_rate_boost).max(ENEMY_SPAWN_MIN);
        if self.spawn_timer <= 0.0 { self.spawn_enemy(); self.spawn_timer = rate; }

        
        self.shard_timer -= dt;
        if self.shard_timer <= 0.0 { self.spawn_shard(); self.shard_timer = SHARD_SPAWN_RATE; }

        
        for e in &mut self.enemies { e.update(dt, &self.player, get_time() as f32); }

        
        let pr = self.player.r; let ppos = self.player.pos;
        
        for e in &self.enemies {
            let d = ppos.distance(e.pos);
            if d>pr && d<NEAR_MISS_DIST && self.player.invuln<=0.0 {
                if thread_rng().gen::<f32>() < 0.02 {
                    self.score += NEAR_MISS_BONUS;
                    self.textfx.push(TextFx{ pos: e.pos, vel: vec2(0.0,-40.0), life: 0.8, text: String::from("near!"), color: hsla(0.1,0.9,0.7,235) });
                }
            }
        }
        
        let mut keep: Vec<Enemy> = Vec::with_capacity(self.enemies.len());
        let mut hit_player = false;
        let drained_enemies: Vec<Enemy> = self.enemies.drain(..).collect();
        for mut e in drained_enemies {
            let d = ppos.distance(e.pos);
            if d <= pr + e.r {
                if self.player.invuln>0.0 {
                    self.add_particles(e.pos, hsla(0.96,0.9,0.7,220), 32, 360.0);
                    self.bump_score(10);
                    
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
                self.textfx.push(TextFx{ pos: sh.pos, vel: vec2(0.0,-40.0), life: 0.8, text: format!("+{}", bonus), color: hsla(0.55,0.9,0.8,235)});
                self.add_particles(sh.pos, hsla(0.55,0.9,0.7,200), 22, 280.0);
            } else {
                let mut s = sh;
                let d = ppos - s.pos; let dist2 = d.length_squared();
                if dist2 < 180.0*180.0 && dist2>0.0 { s.pos += d.normalize()* (120.0*dt); }
                kept.push(s);
            }
        }
        self.shards = kept;

        
        if self.combo_t>0.0 { self.combo_t -= dt; if self.combo_t<=0.0 { self.combo = (self.combo-0.5).max(0.0); self.combo_t = 0.0; } }

        
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

        
        let hud = format!("Score: {}   Combo: x{:.1}", self.score, 1.0+self.combo);
        draw_text(&hud, 16.0, 24.0, 28.0, Color::from_rgba(230,240,250,255));

        if self.player.dash_cd>0.0 {
            let w = 160.0; let x = screen_width()-24.0-w; let y = 16.0;
            draw_rectangle_lines(x, y, w, 10.0, 1.0, Color::from_rgba(70,90,120,220));
            let cdw = w * (1.0 - (self.player.dash_cd / DASH_COOLDOWN).clamp(0.0,1.0));
            draw_rectangle(x, y, cdw, 10.0, Color::from_rgba(120,200,255,255));
            draw_text("dash", x, y+20.0, 20.0, Color::from_rgba(150,200,255,220));
        }

        if self.paused { self.center_msg("Paused — press P to resume", Color::from_rgba(220,220,240,255)); }
        if self.over { self.center_msg(&format!("Game Over  •  Score {}  •  Best {}\nPress R to restart", self.score, self.best), Color::from_rgba(250,210,210,255)); }
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
    }
}


fn rand_f(a: f32, b: f32) -> f32 { thread_rng().gen_range(a..b) }