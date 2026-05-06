@echo off
set "MIDI_OUT=UM-ONE"
set "AUDIO_IN=Neva Uno 1&2 (ESI Audio Device (WDM) - Neva Uno)"

for /L %%i in (1,1,8) do (
  cargo run -p xdx-compare --bin record-preset-dir -- ^
    "calibration/preset_dx100_%%i" ^
    --midi-out "%MIDI_OUT%" ^
    --audio-in "%AUDIO_IN%"
)
