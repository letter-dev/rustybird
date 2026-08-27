#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use macroquad::audio::{load_sound, play_sound, PlaySoundParams, Sound};

use macroquad::prelude::*;
use std::f32::consts::FRAC_PI_2;

const W: f32 = 288.0;
const H: f32 = 512.0;
const GROUND_Y: f32 = H - 112.0;
const BIRD_X: f32 = 60.0;
const BIRD_W: f32 = 34.0;
const BIRD_H: f32 = 24.0;
const GRAVITY: f32 = 1300.0;
const FLAP_VY: f32 = -370.0;
const MAX_FALL: f32 = 620.0;
const PIPE_SPEED: f32 = 120.0;
const PIPE_W: f32 = 52.0;
const PIPE_H: f32 = 320.0;
const PIPE_GAP: f32 = 120.0;
const PIPE_SPACING: f32 = 172.0;
const GAP_MIN: f32 = 60.0;
const GAP_MAX: f32 = 235.0;
const INTRO_T: f32 = 0.5;
const MENU_MOVE_T: f32 = 0.4;
const BLINK_T: f32 = 1.0;
const BLINK_STEP: f32 = 0.13;
const MAIN_X: f32 = 52.0;
const MAIN_Y: f32 = 70.0;
const START_X: f32 = 94.0;
const START_Y: f32 = 330.0;
const START_W: f32 = 100.0;
const START_H: f32 = 56.0;
const MENU_BIRD_X: f32 = 144.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Intro,
    Menu,
    Transition,
    Ready,
    Playing,
    Dying,
    Over,
}

struct Pipe {
    x: f32,
    gap_y: f32,
    passed: bool,
}

struct Assets {
    bg: [Texture2D; 2],
    base: Texture2D,
    pipe: [Texture2D; 2],
    bird: [[Texture2D; 3]; 3],
    digits: [Texture2D; 10],
    msg: Texture2D,
    over: Texture2D,
    main_menu: Texture2D,
    start: Texture2D,
    wing: Sound,
    point: Sound,
    hit: Sound,
    die: Sound,
    swoosh: Sound,
}

async fn tex(path: &str) -> Texture2D {
    let t = load_texture(path)
        .await
        .unwrap_or_else(|_| panic!("missing texture {path}"));
    t.set_filter(FilterMode::Nearest);
    t
}

async fn snd(path: &str) -> Sound {
    load_sound(path)
        .await
        .unwrap_or_else(|_| panic!("missing sound {path}"))
}

async fn load_assets() -> Assets {
    let colors = ["yellow", "red", "blue"];
    let flaps = ["upflap", "midflap", "downflap"];
    let mut birds: Vec<[Texture2D; 3]> = Vec::new();
    for c in colors {
        let mut set: Vec<Texture2D> = Vec::new();
        for f in flaps {
            set.push(tex(&format!("sprites/{c}bird-{f}.png")).await);
        }
        birds.push(set.try_into().unwrap());
    }
    let bird: [[Texture2D; 3]; 3] = birds.try_into().unwrap();
    let mut digs: Vec<Texture2D> = Vec::new();
    for i in 0..10 {
        digs.push(tex(&format!("sprites/{i}.png")).await);
    }
    let digits: [Texture2D; 10] = digs.try_into().unwrap();
    Assets {
        bg: [
            tex("sprites/background-day.png").await,
            tex("sprites/background-night.png").await,
        ],
        base: tex("sprites/base.png").await,
        pipe: [
            tex("sprites/pipe-green.png").await,
            tex("sprites/pipe-red.png").await,
        ],
        bird,
        digits,
        msg: tex("sprites/message.png").await,
        over: tex("sprites/gameover.png").await,
        main_menu: tex("sprites/main.png").await,
        start: tex("sprites/start.png").await,
        wing: snd("audio/wing.wav").await,
        point: snd("audio/point.wav").await,
        hit: snd("audio/hit.wav").await,
        die: snd("audio/die.wav").await,
        swoosh: snd("audio/swoosh.wav").await,
    }
}

fn play(s: &Sound, vol: f32) {
    play_sound(s, PlaySoundParams { looped: false, volume: vol });
}

#[derive(Clone, Copy)]
struct Game {
    state: State,
    score: u32,
    bird_y: f32,
    vy: f32,
    rot: f32,
    anim: f32,
    bg_x: f32,
    base_x: f32,
    bg_i: usize,
    pipe_i: usize,
    bird_i: usize,
    flash: f32,
    die_timer: f32,
    over_t: f32,
    lock: f32,
    t: f32,
    bird_x: f32,
}

impl Game {
    fn new() -> Self {
        Game {
            state: State::Intro,
            t: 0.0,
            bird_x: MENU_BIRD_X,
            score: 0,
            bird_y: 255.0,
            vy: 0.0,
            rot: 0.0,
            anim: 0.0,
            bg_x: 0.0,
            base_x: 0.0,
            bg_i: 0,
            pipe_i: 0,
            bird_i: 0,
            flash: 0.0,
            die_timer: 0.0,
            over_t: 0.0,
            lock: 0.0,
        }
    }

    fn new_round(&mut self) {
        self.score = 0;
        self.bird_y = 255.0;
        self.vy = 0.0;
        self.rot = 0.0;
        self.flash = 0.0;
        self.die_timer = 0.0;
        self.over_t = 0.0;
        self.lock = 0.0;
        self.bg_i = rand::gen_range(0, 2) as usize;
        self.pipe_i = rand::gen_range(0, 2) as usize;
        self.bird_i = rand::gen_range(0, 3) as usize;
    }

    fn flap(&mut self, a: &Assets) {
        self.vy = FLAP_VY;
        play(&a.wing, 0.8);
    }

    fn enter_over(&mut self, best: &mut u32, a: &Assets) {
        if self.score > *best {
            *best = self.score;
            let _ = std::fs::write("best.txt", best.to_string());
        }
        self.state = State::Over;
        self.over_t = 0.0;
        self.lock = 0.55;
        play(&a.swoosh, 0.8);
    }
}

fn aabb(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

fn draw_number(d: &[Texture2D; 10], n: u32, cx: f32, y: f32, scale: f32) {
    let s = n.to_string();
    let widths: Vec<f32> = s
        .bytes()
        .map(|b| d[(b - b'0') as usize].width() as f32 * scale)
        .collect();
    let spacing = 2.0 * scale;
    let total: f32 = widths.iter().sum::<f32>() + spacing * widths.len().saturating_sub(1) as f32;
    let mut x = cx - total / 2.0;
    for (b, w) in s.bytes().zip(widths) {
        let t = &d[(b - b'0') as usize];
        draw_texture_ex(
            t,
            x,
            y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(w, t.height() as f32 * scale)),
                ..Default::default()
            },
        );
        x += w + spacing;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "RustyBird".to_owned(),
        window_width: W as i32,
        window_height: H as i32,
        window_resizable: false,
        icon: load_icon(),
        ..Default::default()
    }
}

fn resize_to<const N: usize>(
    img: &image::RgbaImage,
    size: u32,
) -> Option<[u8; N]> {
    image::imageops::resize(img, size, size, image::imageops::FilterType::Nearest)
        .into_raw()
        .try_into()
        .ok()
}

fn load_icon() -> Option<macroquad::miniquad::conf::Icon> {
    let bytes = std::fs::read("favicon.ico").ok()?;
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Ico)
        .ok()?
        .to_rgba8();
    Some(macroquad::miniquad::conf::Icon {
        small: resize_to::<{ 16 * 16 * 4 }>(&img, 16)?,
        medium: resize_to::<{ 32 * 32 * 4 }>(&img, 32)?,
        big: resize_to::<{ 64 * 64 * 4 }>(&img, 64)?,
    })
}

fn viewport() -> (f32, f32, f32) {
    let sw = screen_width();
    let sh = screen_height();
    if sw <= 0.0 || sh <= 0.0 {
        return (0.0, 0.0, 1.0);
    }
    let scale = (sw / W).min(sh / H);
    ((sw - W * scale) * 0.5, (sh - H * scale) * 0.5, scale)
}

fn to_game(x: f32, y: f32) -> (f32, f32) {
    let (ox, oy, s) = viewport();
    ((x - ox) / s, (y - oy) / s)
}

#[macroquad::main(window_conf)]
async fn main() {
    let a = load_assets().await;
    let mut best: u32 = std::fs::read_to_string("best.txt")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let mut g = Game::new();
    g.new_round();
    let mut pipes: Vec<Pipe> = Vec::new();

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);
        let mut press = is_mouse_button_pressed(MouseButton::Left)
            || is_mouse_button_pressed(MouseButton::Right)
            || is_mouse_button_pressed(MouseButton::Middle)
            || touches().iter().any(|t| t.phase == TouchPhase::Started);

        let (mx, my) = to_game(mouse_position().0, mouse_position().1);
        let mut px = mx;
        let mut py = my;
        let mut held = is_mouse_button_down(MouseButton::Left);
        let mut touch_released = false;
        for t in touches() {
            let (tx, ty) = to_game(t.position.x, t.position.y);
            match t.phase {
                TouchPhase::Started | TouchPhase::Moved => {
                    held = true;
                    px = tx;
                    py = ty;
                }
                TouchPhase::Ended => {
                    px = tx;
                    py = ty;
                    touch_released = true;
                }
                _ => {}
            }
        }
        let over_start = px >= START_X && px <= START_X + START_W && py >= START_Y && py <= START_Y + START_H;
        let start_held = g.state == State::Menu && held && over_start;
        let start_clicked = g.state == State::Menu
            && (is_mouse_button_released(MouseButton::Left) || touch_released)
            && over_start;

        match g.state {
            State::Intro => {
                g.bg_x -= PIPE_SPEED * 0.5 * dt;
                g.base_x -= PIPE_SPEED * dt;
                g.anim += dt;
                g.bird_y = 255.0 + (g.anim * 4.5).sin() * 6.0;
                g.t += dt;
                if g.t >= INTRO_T {
                    g.state = State::Menu;
                }
            }
            State::Menu => {
                g.bg_x -= PIPE_SPEED * 0.5 * dt;
                g.base_x -= PIPE_SPEED * dt;
                g.anim += dt;
                g.bird_y = 255.0 + (g.anim * 4.5).sin() * 6.0;
                if start_clicked {
                    g.state = State::Transition;
                    g.t = 0.0;
                    play(&a.swoosh, 0.8);
                }
            }
            State::Transition => {
                g.bg_x -= PIPE_SPEED * 0.5 * dt;
                g.base_x -= PIPE_SPEED * dt;
                g.anim += dt;
                g.bird_y = 255.0 + (g.anim * 4.5).sin() * 6.0;
                g.t += dt;
                if g.t < MENU_MOVE_T {
                    let p = g.t / MENU_MOVE_T;
                    let e = p * p * (3.0 - 2.0 * p);
                    g.bird_x = MENU_BIRD_X + (BIRD_X - MENU_BIRD_X) * e;
                } else {
                    g.bird_x = BIRD_X;
                }
                if g.t >= MENU_MOVE_T + BLINK_T {
                    g.state = State::Ready;
                    g.lock = 0.15;
                }
            }
            State::Ready => {
                g.bg_x -= PIPE_SPEED * 0.5 * dt;
                g.base_x -= PIPE_SPEED * dt;
                g.anim += dt;
                g.bird_y = 255.0 + (g.anim * 4.5).sin() * 6.0;
                g.rot = 0.0;
                if press && g.lock <= 0.0 {
                    g.state = State::Playing;
                    g.flap(&a);
                }
            }
            State::Playing => {
                if press {
                    g.flap(&a);
                }
                g.anim += dt;
                g.vy = (g.vy + GRAVITY * dt).min(MAX_FALL);
                g.bird_y += g.vy * dt;
                if g.bird_y < 12.0 {
                    g.bird_y = 12.0;
                    g.vy = g.vy.max(0.0);
                }
                let target = if g.vy < 0.0 {
                    -0.45
                } else {
                    (g.vy / 560.0).min(1.0) * FRAC_PI_2
                };
                g.rot += (target - g.rot) * (dt * 10.0).min(1.0);

                g.bg_x -= PIPE_SPEED * 0.5 * dt;
                g.base_x -= PIPE_SPEED * dt;
                for p in pipes.iter_mut() {
                    p.x -= PIPE_SPEED * dt;
                }
                pipes.retain(|p| p.x > -(PIPE_W + 10.0));
                let next_x = pipes
                    .last()
                    .map(|p| p.x + PIPE_SPACING)
                    .unwrap_or(W + 40.0);
                if next_x <= W + 60.0 {
                    pipes.push(Pipe {
                        x: next_x,
                        gap_y: rand::gen_range(GAP_MIN, GAP_MAX),
                        passed: false,
                    });
                }
                for p in pipes.iter_mut() {
                    if !p.passed && p.x + PIPE_W < BIRD_X {
                        p.passed = true;
                        g.score += 1;
                        play(&a.point, 0.8);
                    }
                }

                let bw = BIRD_W - 8.0;
                let bh = BIRD_H - 8.0;
                let bx = BIRD_X - bw / 2.0;
                let by = g.bird_y - bh / 2.0;
                let mut dead = false;
                for p in &pipes {
                    if aabb(bx, by, bw, bh, p.x, -1000.0, PIPE_W, 1000.0 + p.gap_y)
                        || aabb(bx, by, bw, bh, p.x, p.gap_y + PIPE_GAP, PIPE_W, 300.0)
                    {
                        dead = true;
                        break;
                    }
                }
                if dead {
                    play(&a.hit, 0.9);
                    g.flash = 1.0;
                    g.die_timer = 0.35;
                    g.vy = 0.0;
                    g.state = State::Dying;
                } else if g.bird_y + bh / 2.0 >= GROUND_Y {
                    g.bird_y = GROUND_Y - bh / 2.0;
                    play(&a.hit, 0.9);
                    g.enter_over(&mut best, &a);
                }
            }
            State::Dying => {
                g.vy = (g.vy + GRAVITY * dt).min(MAX_FALL);
                g.bird_y += g.vy * dt;
                g.rot += (FRAC_PI_2 - g.rot) * (dt * 6.0).min(1.0);
                if g.bird_y + BIRD_H / 2.0 - 4.0 >= GROUND_Y {
                    g.bird_y = GROUND_Y - BIRD_H / 2.0 + 4.0;
                    g.enter_over(&mut best, &a);
                }
            }
            State::Over => {
                g.over_t += dt;
                if press && g.lock <= 0.0 {
                    play(&a.swoosh, 0.8);
                    pipes.clear();
                    g.new_round();
                    g.bird_x = BIRD_X;
                    g.state = State::Ready;
                    g.lock = 0.0;
                }
            }
        }

        if g.lock > 0.0 {
            g.lock -= dt;
        }
        if g.die_timer > 0.0 {
            g.die_timer -= dt;
            if g.die_timer <= 0.0 {
                play(&a.die, 0.9);
            }
        }
        if g.flash > 0.0 {
            g.flash -= dt * 2.5;
        }
        if g.bg_x <= -W {
            g.bg_x += W;
        }
        let base_w = a.base.width() as f32;
        if g.base_x <= -base_w {
            g.base_x += base_w;
        }

        let (ox, oy, s) = viewport();
        let sw = screen_width();
        let sh = screen_height();
        set_camera(&Camera2D {
            zoom: vec2(2.0 * s / sw, 2.0 * s / sh),
            target: vec2(W / 2.0, H / 2.0),
            ..Default::default()
        });
        clear_background(BLACK);

        let bg = &a.bg[g.bg_i];
        draw_texture(bg, g.bg_x, 0.0, WHITE);
        draw_texture(bg, g.bg_x + W, 0.0, WHITE);

        for p in &pipes {
            draw_texture(
                &a.pipe[g.pipe_i],
                p.x,
                p.gap_y + PIPE_GAP,
                WHITE,
            );
            draw_texture_ex(
                &a.pipe[g.pipe_i],
                p.x,
                p.gap_y - PIPE_H,
                WHITE,
                DrawTextureParams {
                    flip_y: true,
                    ..Default::default()
                },
            );
        }

        draw_texture(&a.base, g.base_x, GROUND_Y, WHITE);
        draw_texture(&a.base, g.base_x + base_w, GROUND_Y, WHITE);

        match g.state {
            State::Intro | State::Menu => {
                draw_texture(&a.main_menu, MAIN_X, MAIN_Y, WHITE);
                let tint = if start_held {
                    Color::new(0.68, 0.68, 0.68, 1.0)
                } else {
                    WHITE
                };
                draw_texture_ex(
                    &a.start,
                    START_X,
                    START_Y,
                    tint,
                    DrawTextureParams {
                        dest_size: Some(vec2(START_W, START_H)),
                        ..Default::default()
                    },
                );
                if g.state == State::Intro {
                    let a = 1.0 - (g.t / INTRO_T).min(1.0);
                    draw_rectangle(0.0, 0.0, W, H, Color::new(0.0, 0.0, 0.0, a));
                }
            }
            State::Transition => {
                if g.t < MENU_MOVE_T {
                    let p = (g.t / MENU_MOVE_T).min(1.0);
                    let e = p * p * (3.0 - 2.0 * p);
                    draw_texture(&a.main_menu, MAIN_X, MAIN_Y - e * 400.0, WHITE);
                    draw_texture_ex(
                        &a.start,
                        START_X,
                        START_Y + e * 250.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(START_W, START_H)),
                            ..Default::default()
                        },
                    );
                } else {
                    let bt = g.t - MENU_MOVE_T;
                    let blink_end = BLINK_T - 0.35;
                    let alpha = if bt < blink_end {
                        ((bt / BLINK_STEP) as i32 % 2 == 0) as u8 as f32
                    } else {
                        1.0
                    };
                    draw_texture(
                        &a.msg,
                        (W - a.msg.width() as f32) / 2.0,
                        70.0,
                        Color::new(1.0, 1.0, 1.0, alpha),
                    );
                }
            }
            State::Ready => {
                draw_texture(&a.msg, (W - a.msg.width() as f32) / 2.0, 70.0, WHITE);
            }
            State::Playing | State::Dying => {
                draw_number(&a.digits, g.score, W / 2.0, 44.0, 1.0);
            }
            State::Over => {
                let t = (g.over_t / 0.45).min(1.0);
                let e = 1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t);
                let y = -70.0 + 200.0 * e;
                draw_texture(&a.over, (W - a.over.width() as f32) / 2.0, y, WHITE);
                if g.over_t > 0.3 {
                    draw_number(&a.digits, g.score, W / 2.0, 210.0, 1.0);
                    let label = "BEST";
                    let m = measure_text(label, None, 18, 1.0);
                    let lx = W / 2.0 - m.width / 2.0;
                    draw_text(label, lx + 1.0, 297.0, 18.0, Color::from_rgba(87, 54, 24, 255));
                    draw_text(label, lx, 296.0, 18.0, WHITE);
                    draw_number(&a.digits, best, W / 2.0, 306.0, 0.66);
                }
            }
        }

        let frame = if g.state == State::Dying || g.state == State::Over {
            1
        } else {
            ((g.anim / 0.09) as usize) % 3
        };
        draw_texture_ex(
            &a.bird[g.bird_i][frame],
            g.bird_x - BIRD_W / 2.0,
            g.bird_y - BIRD_H / 2.0,
            WHITE,
            DrawTextureParams {
                rotation: g.rot,
                pivot: Some(vec2(g.bird_x, g.bird_y)),
                ..Default::default()
            },
        );

        if g.flash > 0.0 {
            draw_rectangle(0.0, 0.0, W, H, Color::new(1.0, 1.0, 1.0, g.flash.min(1.0)));
        }

        set_default_camera();

        next_frame().await
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn quad_main() {
    main();
}
