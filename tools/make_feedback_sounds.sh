#!/bin/bash
# Builds the kill-feedback one-shots: the headshot chime and the multi-kill
# announcements.
#
# Nothing is sampled here. The chime is three decaying partials written out by
# ffmpeg, and the announcements are the system voice pitched down and given a
# room -- which is what an arena shooter's announcer is, a voice in a hall.
# Both are generated rather than fetched so there is no third-party licence to
# carry for them; the Sonniss bundle in make_weapon_sounds.sh covers the
# weapons and nothing else.
#
# The ladder is the Unreal one: two kills inside the window is a double, three
# a triple, four or more a multi. MULTI_KILL_WINDOW in lib.rs is what decides
# "inside"; this file only decides what it sounds like.
set -e
cd "$(dirname "$0")/.."
out="crates/client/assets/audio/feedback"
work="${TMPDIR:-/tmp}/feedback_sounds"
mkdir -p "$out" "$work"

SR=48000
# Lower than the voice speaks, because an announcer is not a narrator. The
# tempo correction puts the duration back so the words still land on the beat.
PITCH=0.82
VOICE=Daniel

# WAV, like the weapon one-shots. These are under two seconds each, and this
# ffmpeg carries no vorbis encoder worth the argument.
pack() { ffmpeg -y -hide_banner -loglevel error -i "$1" -c:a pcm_s16le -ac 1 -ar $SR "$2"; }

# bc drops the leading zero on values under one and ffmpeg rejects ".36" as a
# duration, so every number handed to a filter goes through here.
calc() { awk "BEGIN{printf \"%.6f\", $1}"; }

# Three partials on a stretched-harmonic ratio, each decaying faster than the
# one below it. That is what makes a struck metal sound rather than a beep:
# the top of the spectrum leaves first.
chime() {
  local f=$1 dur=$2 gain=$3 dst=$4
  ffmpeg -y -hide_banner -loglevel error -f lavfi -i \
    "aevalsrc=\
${gain}*sin(2*PI*${f}*t)*exp(-14*t)\
+$(calc "$gain*0.55")*sin(2*PI*$(calc "$f*2.76")*t)*exp(-24*t)\
+$(calc "$gain*0.30")*sin(2*PI*$(calc "$f*5.40")*t)*exp(-44*t)\
:d=${dur}:s=${SR}" \
    -af "afade=t=out:st=$(calc "$dur-0.04"):d=0.04" \
    "$dst"
}

echo "headshot chime"
chime 1760 0.40 0.70 "$work/headshot_raw.wav"
pack "$work/headshot_raw.wav" "$out/headshot.wav"

# A shorter, lower sibling for an ordinary kill: present enough to confirm the
# hit landed, quiet enough to fire twenty times a minute without wearing out.
echo "kill tick"
chime 880 0.18 0.34 "$work/kill_raw.wav"
pack "$work/kill_raw.wav" "$out/kill.wav"

say_line() {
  local text=$1 dst=$2
  say -v "$VOICE" -o "$work/raw.aiff" "$text"
  # asetrate drops pitch and lengthens together; atempo puts the length back.
  # aecho is a small hard room -- two taps, nothing long enough to smear the
  # consonants, which is what makes "triple" and "multi" tell apart.
  ffmpeg -y -hide_banner -loglevel error -i "$work/raw.aiff" -af "\
asetrate=${SR}*${PITCH},aresample=${SR},atempo=$(calc "1/$PITCH"),\
aecho=0.8:0.85:28|53:0.28|0.16,\
dynaudnorm=f=180:g=9,\
afade=t=in:d=0.01" -ar $SR "$work/line.wav"
  pack "$work/line.wav" "$dst"
}

for pair in "double_kill:Double Kill" "triple_kill:Triple Kill" "multi_kill:Multi Kill"; do
  name=${pair%%:*}
  line=${pair#*:}
  echo "$name  \"$line\""
  say_line "$line" "$out/$name.wav"
done

echo
ls -la "$out"
