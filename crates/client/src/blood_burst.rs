//! The spray a round makes going in, and the frames it is drawn with.
//!
//! The burst used to be one picture -- `splatter_0.png`, the same dried
//! stain the floor gets -- scaled from 0.35 to 1.7 and faded out over 0.22s.
//! A still image growing and dimming reads as a decal expanding, not as
//! something thrown, which is why it never looked like the shipped games
//! however the numbers were tuned.
//!
//! What they draw is a flipbook: a real sequence, every frame a different
//! shape, played once. This generates that sequence rather than shipping it,
//! which also settles the motion-vector question -- motion vectors exist to
//! interpolate between frames so a sheet can ship fewer of them, and nothing
//! here is shipped. Frames are cheap when they are computed at load, so the
//! answer is simply to compute enough of them.

use bevy::image::Image;
use bevy::math::ops;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Frames in the burst sequence.
///
/// The burst lives `BLOOD_BURST_LIFETIME`, so at sixty frames a second the
/// eye gets about thirteen of these. More than that is paid for and never
/// seen; fewer and the sequence steps visibly.
pub(crate) const BURST_FRAMES: usize = 16;

/// Side of each generated frame, in pixels.
pub(crate) const BURST_FRAME_SIZE: u32 = 96;

/// Blobs thrown by one burst. They are the shape of it: a few large near the
/// middle and a scatter of small ones running ahead, which is what separates
/// a spray from a splash.
const BURST_BLOBS: usize = 26;

/// How far the furthest blob has travelled by the last frame, as a fraction
/// of the frame's half-width. Under 1.0 so the outermost blob still has its
/// falloff inside the texture rather than clipped at the edge.
const BURST_REACH: f32 = 0.86;

/// One deterministic stream, so a given variant is the same every run and a
/// frame can be regenerated without the sequence shifting under it.
fn hash01(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 13;
    (x & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

struct Blob {
    /// Direction out from the centre.
    dir: Vec2,
    /// How far along `BURST_REACH` this one gets, and how fast it leaves.
    reach: f32,
    speed: f32,
    /// Radius at birth, as a fraction of the half-width.
    radius: f32,
}

fn blobs_for(variant: usize) -> Vec<Blob> {
    (0..BURST_BLOBS)
        .map(|i| {
            let s = (variant * 977 + i * 131) as u32;
            let angle = hash01(s) * std::f32::consts::TAU;
            // Squared, so most blobs stay near the middle and a few run out
            // ahead. An even spread reads as a ring rather than a spray.
            let t = hash01(s + 7);
            Blob {
                dir: Vec2::new(ops::cos(angle), ops::sin(angle)),
                reach: t * t,
                speed: 0.55 + hash01(s + 13) * 0.45,
                radius: 0.07 + (1.0 - t) * 0.17 + hash01(s + 19) * 0.05,
            }
        })
        .collect()
}

/// One frame of the sequence, `step` of `BURST_FRAMES`.
///
/// White with an alpha shape: the material carries the colour, so the same
/// frames serve the entrance burst and the exit mist at different tints.
fn burst_frame(variant: usize, step: usize, size: u32) -> Image {
    let blobs = blobs_for(variant);
    let life = step as f32 / (BURST_FRAMES - 1).max(1) as f32;
    // Out fast, then coasting: a spray is given all its speed at once and
    // spends the rest of the frames slowing down.
    let eased = 1.0 - (1.0 - life) * (1.0 - life);
    // Thinning rather than fading: each blob shrinks as it goes, so the
    // sequence breaks up into droplets instead of dimming as one sheet.
    let thin = (1.0 - life * 0.75).max(0.05);

    let centre = (size as f32 - 1.0) * 0.5;
    let mut data = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let px = (x as f32 - centre) / centre;
            let py = (y as f32 - centre) / centre;
            let mut alpha: f32 = 0.0;
            for blob in &blobs {
                let travel = blob.reach * BURST_REACH * eased * blob.speed;
                let at = blob.dir * travel;
                let r = blob.radius * thin;
                if r <= 1e-4 {
                    continue;
                }
                let d = ((px - at.x).powi(2) + (py - at.y).powi(2)).sqrt() / r;
                if d >= 1.0 {
                    continue;
                }
                // Cubed falloff: a solid core with the softness spent at the
                // rim, the same shape the droplet sprite uses.
                let e = 1.0 - d;
                alpha = alpha.max(e * e * e);
            }
            let i = ((y * size + x) * 4) as usize;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        // Not RENDER_WORLD: the native extractor reads image.data, and a
        // frame whose CPU copy was dropped draws as an untextured square.
        crate::gltf_usages(),
    )
}

/// Every frame of every variant, as materials ready to swap between.
///
/// Indexed `[variant][frame]`. Built once at startup: sixteen frames of
/// 96 square is 590KB a variant, which is less than the single 512 splatter
/// each burst used to borrow.
pub(crate) fn build_burst_frames(
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    variants: usize,
    tint: Color,
) -> Vec<Vec<Handle<StandardMaterial>>> {
    (0..variants)
        .map(|variant| {
            (0..BURST_FRAMES)
                .map(|step| {
                    materials.add(StandardMaterial {
                        base_color: tint,
                        base_color_texture: Some(
                            images.add(burst_frame(variant, step, BURST_FRAME_SIZE)),
                        ),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        cull_mode: None,
                        ..default()
                    })
                })
                .collect()
        })
        .collect()
}

/// Which frame a burst is showing at this point in its life.
///
/// Clamped rather than wrapped: the sequence plays once and holds its last
/// frame for whatever is left, which is nothing at these lifetimes but would
/// otherwise restart the spray if a lifetime were ever raised.
pub(crate) fn burst_frame_index(life: f32) -> usize {
    ((life.clamp(0.0, 1.0) * BURST_FRAMES as f32) as usize).min(BURST_FRAMES - 1)
}

