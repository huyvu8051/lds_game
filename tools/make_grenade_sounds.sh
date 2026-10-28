#!/bin/bash
# The five sounds a throwable makes: the pin, the throw, the blast, the
# smoke's hiss and the flash.
#
# Synthesised rather than sampled, like the kill feedback. The Sonniss bundle
# that covers the weapons has no grenade material, and a blast is one of the
# easier things to build from noise: a crack, a body, and a tail.
set -e
cd "$(dirname "$0")/.."
out="crates/client/assets/audio/grenades"
work="${TMPDIR:-/tmp}/grenade_sounds"
mkdir -p "$out" "$work"

SR=48000
calc() { awk "BEGIN{printf \"%.6f\", $1}"; }
pack() { ffmpeg -y -hide_banner -loglevel error -i "$1" -c:a pcm_s16le -ac 1 -ar $SR "$2"; }

gen() { ffmpeg -y -hide_banner -loglevel error -f lavfi -i "aevalsrc=$1:d=$2:s=${SR}" "$3"; }

# Metal on metal: two short partials, gone almost before they arrive.
echo "pin"
gen "0.5*sin(2*PI*2400*t)*exp(-60*t)+0.3*sin(2*PI*3700*t)*exp(-90*t)" 0.12 "$work/pin.wav"
pack "$work/pin.wav" "$out/pin.wav"

# Cloth and air. Filtered noise with a soft swell rather than an onset.
echo "throw"
gen "0.35*random(0)*sin(PI*t/0.22)" 0.22 "$work/throw_raw.wav"
ffmpeg -y -hide_banner -loglevel error -i "$work/throw_raw.wav" \
  -af "lowpass=f=1800,highpass=f=300,volume=1.6" "$work/throw.wav"
pack "$work/throw.wav" "$out/throw.wav"

# The blast: a crack on top of a body, with a tail long enough to sit in.
# The exponents are what separate it from a drum -- the high end has to
# leave in a few milliseconds and the low end has to stay for a second.
echo "blast"
gen "0.9*random(0)*exp(-40*t)+0.8*random(0)*exp(-6*t)+0.6*sin(2*PI*55*t)*exp(-4*t)" 1.6 "$work/blast_raw.wav"
ffmpeg -y -hide_banner -loglevel error -i "$work/blast_raw.wav" \
  -af "lowpass=f=5000,aecho=0.9:0.6:120|260:0.35|0.18,volume=1.4,afade=t=out:st=1.3:d=0.3" \
  "$work/blast.wav"
pack "$work/blast.wav" "$out/blast.wav"

# Smoke: a long hiss that swells and holds. Narrow-band noise, no onset.
echo "hiss"
gen "0.5*random(0)*(1-exp(-6*t))" 3.0 "$work/hiss_raw.wav"
ffmpeg -y -hide_banner -loglevel error -i "$work/hiss_raw.wav" \
  -af "highpass=f=2200,lowpass=f=9000,volume=1.2,afade=t=out:st=2.6:d=0.4" "$work/hiss.wav"
pack "$work/hiss.wav" "$out/hiss.wav"

# Flash: a crack with nothing underneath it, then the ring in the ears --
# a pure tone decaying over a second and a half, which is the part anybody
# who has heard one remembers.
echo "flash"
gen "0.95*random(0)*exp(-70*t)+0.30*sin(2*PI*4200*t)*exp(-1.6*t)+0.12*sin(2*PI*6300*t)*exp(-1.2*t)" 2.0 "$work/flash_raw.wav"
ffmpeg -y -hide_banner -loglevel error -i "$work/flash_raw.wav" \
  -af "volume=1.3,afade=t=out:st=1.7:d=0.3" "$work/flash.wav"
pack "$work/flash.wav" "$out/flash.wav"

echo
ls -la "$out"
