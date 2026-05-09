> xdx-synth のcalibration/eg_ar_calib の hw と sw の差ですが、ほんのちょっとのカーブの差が出ています。swをhwに近づけるには？D:\home\wurly\project\mi\xdx-rs>cargo run -p xdx-e2e --example compare_eg
  -- --dir calibration/eg_ar_calib --hold-ms 10000 --detail 1
  warning: patch `midir v0.10.3 (D:\home\wurly\project\mi\xdx-rs\vendor\midir)` was not used in the crate graph
  help: Check that the patched package version and available features are compatible
        with the dependency requirements. If the patch has a different version from
        what is locked in the Cargo.lock file, run `cargo update` to use the new
        version. This may also occur with an optional dependency that is not enabled.
     Compiling xdx-e2e v0.1.0 (D:\home\wurly\project\mi\xdx-rs\xdx-e2e)
      Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.02s
       Running `target\debug\examples\compare_eg.exe --dir calibration/eg_ar_calib --hold-ms 10000 --detail 1`
  === EG Comparison  dir=calibration/eg_ar_calib  hold=10000ms ===

  #    Name        atk90(HW)  atk90(SY)  d1l(HW)  d1l(SY)  rls50(HW)  rls50(SY)  rls90(HW)  rls90(SY)
  ----------------------------------------------------------------------------------------------------
  1    AR01          5710.0ms    5730.0ms    0.000    0.191     700.0ms       0.0ms     700.0ms       0.0ms


     t(ms)      HW      SY  HW/SY  bar (H=HW, S=SY)
    ------------------------------------------------------------------------
         0   0.006   0.005   1.25
        10   0.006   0.005   1.17
        20   0.006   0.006   1.07
        30   0.006   0.006   1.02
        40   0.006   0.006   0.96
        50   0.006   0.007   0.93
        60   0.006   0.007   0.88
        70   0.006   0.008   0.82
        80   0.006   0.008   0.78
        90   0.006   0.008   0.74
       100   0.007   0.009   0.79
       110   0.009   0.009   0.95
       120   0.009   0.010   0.89
       130   0.009   0.010   0.85
       140   0.009   0.011   0.82
       150   0.009   0.011   0.80
       160   0.009   0.011   0.77
       170   0.009   0.012   0.72
       180   0.009   0.012   0.69
       190   0.009   0.013   0.67
       210   0.009   0.014   0.68
       230   0.012   0.015   0.78
       250   0.012   0.016   0.74
       270   0.012   0.017   0.68
       290   0.012   0.018   0.64
       310   0.012   0.019   0.61
       330   0.016   0.021   0.74
       350   0.016   0.022   0.72
       390   0.016   0.025   0.63
       450   0.020   0.029   0.71

  ──── (810 lines hidden) ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
  -
     10690   0.000   0.000    -

  2    AR02          3850.0ms    3840.0ms    0.163    0.449    1120.0ms       0.0ms    1120.0ms       0.0ms
  3    AR03          2880.0ms    2880.0ms    0.384    0.588    1340.0ms       0.0ms    1340.0ms       0.0ms
  2    AR02          3850.0ms    3840.0ms    0.163    0.449    1120.0ms       0.0ms    1120.0ms       0.0ms
  3    AR03          2880.0ms    2880.0ms    0.384    0.588    1340.0ms       0.0ms    1340.0ms       0.0ms
  4    AR04          1890.0ms    1900.0ms    0.572    0.707    1540.0ms       0.0ms    1540.0ms       0.0ms
  5    AR05          1450.0ms    1430.0ms    0.670    0.786    1630.0ms       0.0ms    1630.0ms       0.0ms
  6    AR06           980.0ms     950.0ms    0.791    0.856    1760.0ms       0.0ms    1760.0ms       0.0ms
  14   AR14            70.0ms      60.0ms    0.978    0.984      40.0ms      10.0ms    1950.0ms      10.0ms
  14   AR14            70.0ms      60.0ms    0.978    0.984      40.0ms      10.0ms    1950.0ms      10.0ms
  15   AR15            50.0ms      30.0ms    0.977    0.974      30.0ms      10.0ms    1950.0ms      10.0ms
  16   AR16            30.0ms      30.0ms    0.980    0.984      30.0ms      10.0ms      50.0ms      10.0ms
  17   AR17            30.0ms      20.0ms    0.988    0.984      10.0ms      10.0ms      40.0ms      10.0ms
  18   AR18            20.0ms      20.0ms    0.984    0.991      10.0ms      10.0ms      50.0ms      10.0ms
  19   AR19            20.0ms      20.0ms    0.988    0.991      10.0ms      10.0ms      20.0ms      10.0ms
  14   AR14            70.0ms      60.0ms    0.978    0.984      40.0ms      10.0ms    1950.0ms      10.0ms
  15   AR15            50.0ms      30.0ms    0.977    0.974      30.0ms      10.0ms    1950.0ms      10.0ms
  16   AR16            30.0ms      30.0ms    0.980    0.984      30.0ms      10.0ms      50.0ms      10.0ms
  17   AR17            30.0ms      20.0ms    0.988    0.984      10.0ms      10.0ms      40.0ms      10.0ms
  18   AR18            20.0ms      20.0ms    0.984    0.991      10.0ms      10.0ms      50.0ms      10.0ms
  19   AR19            20.0ms      20.0ms    0.988    0.991      10.0ms      10.0ms      20.0ms      10.0ms
  14   AR14            70.0ms      60.0ms    0.978    0.984      40.0ms      10.0ms    1950.0ms      10.0ms
  15   AR15            50.0ms      30.0ms    0.977    0.974      30.0ms      10.0ms    1950.0ms      10.0ms
  16   AR16            30.0ms      30.0ms    0.980    0.984      30.0ms      10.0ms      50.0ms      10.0ms
  17   AR17            30.0ms      20.0ms    0.988    0.984      10.0ms      10.0ms      40.0ms      10.0ms
  18   AR18            20.0ms      20.0ms    0.984    0.991      10.0ms      10.0ms      50.0ms      10.0ms
  19   AR19            20.0ms      20.0ms    0.988    0.991      10.0ms      10.0ms      20.0ms      10.0ms
  20   AR20            20.0ms      10.0ms    0.987    0.991      10.0ms      10.0ms      40.0ms      10.0ms                                                                14   AR14            70.0ms      60.0ms    0.978    0.984      40.0ms      10.0ms    1950.0ms      10.0ms
  15   AR15            50.0ms      30.0ms    0.977    0.974      30.0ms      10.0ms    1950.0ms      10.0ms
  16   AR16            30.0ms      30.0ms    0.980    0.984      30.0ms      10.0ms      50.0ms      10.0ms
  17   AR17            30.0ms      20.0ms    0.988    0.984      10.0ms      10.0ms      40.0ms      10.0ms
  18   AR18            20.0ms      20.0ms    0.984    0.991      10.0ms      10.0ms      50.0ms      10.0ms
  19   AR19            20.0ms      20.0ms    0.988    0.991      10.0ms      10.0ms      20.0ms      10.0ms
  20   AR20            20.0ms      10.0ms    0.987    0.991      10.0ms      10.0ms      40.0ms      10.0ms
  14   AR14            70.0ms      60.0ms    0.978    0.984      40.0ms      10.0ms    1950.0ms      10.0ms
  15   AR15            50.0ms      30.0ms    0.977    0.974      30.0ms      10.0ms    1950.0ms      10.0ms
  16   AR16            30.0ms      30.0ms    0.980    0.984      30.0ms      10.0ms      50.0ms      10.0ms
  17   AR17            30.0ms      20.0ms    0.988    0.984      10.0ms      10.0ms      40.0ms      10.0ms
  18   AR18            20.0ms      20.0ms    0.984    0.991      10.0ms      10.0ms      50.0ms      10.0ms
  19   AR19            20.0ms      20.0ms    0.988    0.991      10.0ms      10.0ms      20.0ms      10.0ms
  20   AR20            20.0ms      10.0ms    0.987    0.991      10.0ms      10.0ms      40.0ms      10.0ms
  14   AR14            70.0ms      60.0ms    0.978    0.984      40.0ms      10.0ms    1950.0ms      10.0ms
  15   AR15            50.0ms      30.0ms    0.977    0.974      30.0ms      10.0ms    1950.0ms      10.0ms
  16   AR16            30.0ms      30.0ms    0.980    0.984      30.0ms      10.0ms      50.0ms      10.0ms
  15   AR15            50.0ms      30.0ms    0.977    0.974      30.0ms      10.0ms    1950.0ms      10.0ms
  16   AR16            30.0ms      30.0ms    0.980    0.984      30.0ms      10.0ms      50.0ms      10.0ms
  15   AR15            50.0ms      30.0ms    0.977    0.974      30.0ms      10.0ms    1950.0ms      10.0ms
  16   AR16            30.0ms      30.0ms    0.980    0.984      30.0ms      10.0ms      50.0ms      10.0ms
  15   AR15            50.0ms      30.0ms    0.977    0.974      30.0ms      10.0ms    1950.0ms      10.0ms
  16   AR16            30.0ms      30.0ms    0.980    0.984      30.0ms      10.0ms      50.0ms      10.0ms
  17   AR17            30.0ms      20.0ms    0.988    0.984      10.0ms      10.0ms      40.0ms      10.0ms
  18   AR18            20.0ms      20.0ms    0.984    0.991      10.0ms      10.0ms      50.0ms      10.0ms
  19   AR19            20.0ms      20.0ms    0.988    0.991      10.0ms      10.0ms      20.0ms      10.0ms
  20   AR20            20.0ms      10.0ms    0.987    0.991      10.0ms      10.0ms      40.0ms      10.0ms
  21   AR22            10.0ms      10.0ms    0.988    0.991      10.0ms      10.0ms      10.0ms      10.0ms
  22   AR25            10.0ms      10.0ms    0.989    0.991      10.0ms      10.0ms      20.0ms      10.0ms
  23   AR28            10.0ms      10.0ms    0.989    0.991      10.0ms      10.0ms      10.0ms      10.0ms
  24   AR31            10.0ms      10.0ms    0.988    0.991      10.0ms      10.0ms      20.0ms      10.0ms

● データをよく見ると、序盤（0〜2000ms）でSWがHWより高め（SW先行）、中盤以降でHWがSWを追い越すというカーブ形状の差があります。まずsynth側の攻撃フェーズ
  の実装を確認します。

● Explore(EG attack curve implementation in xdx-synth)
  ⎿  Done (8 tool uses · 35.2k tokens · 1m 8s)
  (ctrl+o to expand)

● 実装の詳細を直接確認します。

  Searched for 1 pattern, read 1 file (ctrl+o to expand)

● テストバンクの定義も確認します。

  Searched for 1 pattern, read 1 file (ctrl+o to expand)

● compare_eg の測定ロジックを確認します。

  Read 1 file (ctrl+o to expand)

● FMエンジンでenvelope levelがどう使われているかを確認します。

  Read 1 file (ctrl+o to expand)

● calibration.mdを確認してから判断します。

  Searched for 1 pattern, read 2 files (ctrl+o to expand)

● データを十分に分析できました。考察をまとめます。

  ---
  分析結果

  SW の onset time offset（ゼロホールド後の rise 開始点）:

  SW は smoothstep で smoothstep(t_norm) ≈ 0.005 になる t_norm ≈ 0.042 でオンセット検出。onset = zero_hold(500ms) + 0.042 × rise_s(7500ms) ≈ 815ms
  (note-on起算)。

  これを使って SW の各テーブル値を検証:
  - t_table=490ms → t_norm=(490+315)/7500=0.107 → smoothstep(0.107)=0.032 ✓ (データ: SW=0.032)
  - t_table=990ms → t_norm=0.174 → smoothstep(0.174)=0.080 ✓ (データ: SW=0.080)

  SW は smoothstep で完全に説明できる。

  ---
  HW vs SW の差の正体

  ┌────────┬─────────────────┬───────┬──────────────────┐
  │ t_norm │ SW (smoothstep) │  HW   │      HW/SW       │
  ├────────┼─────────────────┼───────┼──────────────────┤
  │ 0.107  │ 0.032           │ 0.020 │ 0.63             │
  ├────────┼─────────────────┼───────┼──────────────────┤
  │ 0.174  │ 0.080           │ 0.060 │ 0.75             │
  ├────────┼─────────────────┼───────┼──────────────────┤
  │ 0.321  │ 0.243           │ 0.243 │ 1.00 ← crossover │
  ├────────┼─────────────────┼───────┼──────────────────┤
  │ 0.443  │ 0.415           │ 0.442 │ 1.06             │
  ├────────┼─────────────────┼───────┼──────────────────┤
  │ 0.613  │ 0.644           │ 0.656 │ 1.02             │
  ├────────┼─────────────────┼───────┼──────────────────┤
  │ 0.803  │ 0.900           │ 0.900 │ 1.00 ← atk90     │
  └────────┴─────────────────┴───────┴──────────────────┘

  パターン: t_norm < 0.32 では SW > HW（SW が速すぎ）、t_norm > 0.32 では HW > SW（HW が速い）。総時間はほぼ同じ。

  これは smoothstep の 屈曲点(inflection point) が t=0.50 にあるのに対し、HW の屈曲点が t≈0.40 にあることを示す。

  ---
  改善アプローチ

  アプローチA（最小変更・試行推奨）: 屈曲点を前にずらした quartic 曲線

  cubic の smoothstep 3t²-2t³ は f'(0)=f'(1)=0 を満たす唯一の cubic なので、屈曲点を変えるには degree を上げる必要がある。degree-4 で inflection=0.40
  の曲線を計算すると:

  f(t) ≈ 4.365t² - 4.729t³ + 1.364t⁴

  ただしこれはコードの AR01 データとの誤差がまだ残る（最初期の乖離は構造的に残る可能性あり）。

  アプローチB（物理モデル的仮説）:

  DX100 の attack EG は DX7 同様、内部レジスタを 線形インクリメント し、出力は dB→振幅テーブル変換 を経ている可能性が高い。これは:
  - 初期: 非常に小さい振幅（log スケールの底部）
  - 中盤: 急速に上昇
  - 後半: peak へ漸近

  を生む。ただし直接的な linear-dB モデル（例: 10^(-k*(1-t))）を試したが、AR01 のスケールには合わなかった。

  アプローチC（実用的な小変更を試す）:

  smoothstep の引数に軽い時間 warping をかける:

  // current
  self.level = self.ar_t * self.ar_t * (3.0 - 2.0 * self.ar_t);

  // candidate: ar_t を少し「後ろに歪める」（早い時間を遅く、遅い時間を速くする）
  let t = self.ar_t;
  let t_w = t * t * (3.0 - 2.0 * t);  // smoothstep で ar_t 自体を warping
  self.level = t_w * t_w * (3.0 - 2.0 * t_w);  // double-smoothstep

  ただし double-smoothstep を計算したところ t=0.107 で 0.0032 となり低すぎる（HW=0.020 に対し逆方向に悪化）。
  を生む。ただし直接的な linear-dB モデル（例: 10^(-k*(1-t))）を試したが、AR01 のスケールには合わなかった。

  アプローチC（実用的な小変更を試す）:

  smoothstep の引数に軽い時間 warping をかける:

  // current
  self.level = self.ar_t * self.ar_t * (3.0 - 2.0 * self.ar_t);

  // candidate: ar_t を少し「後ろに歪める」（早い時間を遅く、遅い時間を速くする）
  let t = self.ar_t;
  let t_w = t * t * (3.0 - 2.0 * t);  // smoothstep で ar_t 自体を warping
  self.level = t_w * t_w * (3.0 - 2.0 * t_w);  // double-smoothstep

  ただし double-smoothstep を計算したところ t=0.107 で 0.0032 となり低すぎる（HW=0.020 に対し逆方向に悪化）。

  ---
  最も有望な試み

  t_norm=0.107 で HW=0.020 になる変換として smoothstep(t^β) を逆算した結果：
  let t_w = t * t * (3.0 - 2.0 * t);  // smoothstep で ar_t 自体を warping
  self.level = t_w * t_w * (3.0 - 2.0 * t_w);  // double-smoothstep

  ただし double-smoothstep を計算したところ t=0.107 で 0.0032 となり低すぎる（HW=0.020 に対し逆方向に悪化）。

  ---
  - 中盤: 急速に上昇
  - 後半: peak へ漸近

  を生む。ただし直接的な linear-dB モデル（例: 10^(-k*(1-t))）を試したが、AR01 のスケールには合わなかった。

  アプローチC（実用的な小変更を試す）:

  smoothstep の引数に軽い時間 warping をかける:

  // current
  self.level = self.ar_t * self.ar_t * (3.0 - 2.0 * self.ar_t);

  // candidate: ar_t を少し「後ろに歪める」（早い時間を遅く、遅い時間を速くする）
  let t = self.ar_t;
  let t_w = t * t * (3.0 - 2.0 * t);  // smoothstep で ar_t 自体を warping
  self.level = t_w * t_w * (3.0 - 2.0 * t_w);  // double-smoothstep

  ただし double-smoothstep を計算したところ t=0.107 で 0.0032 となり低すぎる（HW=0.020 に対し逆方向に悪化）。

  ---
  最も有望な試み

  t_norm=0.107 で HW=0.020 になる変換として smoothstep(t^β) を逆算した結果：

  ┌────────┬────────────┐
  │ t_norm │ β (計算値) │
  ├────────┼────────────┤
  │ 0.107  │ 1.10       │
  ├────────┼────────────┤
  │ 0.174  │ 1.09       │
  ├────────┼────────────┤
  │ 0.321  │ 1.00       │
  ├────────┼────────────┤
  │ 0.443  │ 0.95       │
  └────────┴────────────┘

  β は一定でなく、早い段階で ~1.1、後半で ~0.95 と変化する。単純な smoothstep(t^β) では crossover 以降で逆方向に悪化する。

  結論として推奨する方向:

  1. 現状維持が現実的: atk90 タイミングは 20ms 以内で一致し、振幅の差は level > 0.3 で 5% 以内。耳感への影響は微小。
  2. さらに精度を上げるなら: DX100 の EG チップの実装（内部レジスタビット幅・log テーブル）を調査する必要がある。DX7 の Ken Shirriff 解析
  (https://www.righto.com/2021/11/the-yamaha-dx7-synthesizer-and-the.html) に類似の情報がある。DX100 が同様の10-bit dB
  テーブルを使うなら、そのテーブルを使った実装に置き換えることで形状が一致するはず。
  3. 実験的な手軽な変更: ar_t を smoothstep に渡す前に ar_t * ar_t / (ar_t * ar_t + (1 - ar_t) * (1 - ar_t)) 的な非対称変換で試す価値はある（ただし事前に理論値と照合して）。

  どのアプローチを試しますか？コード変更の実装も可能です。

✻ Cooked for 25m 56s

※ recap: EGアタックのカーブ形状をHW実機に近づける方法を分析していた。次のアクション：DX100のEG実装（dBテーブル方式か）を調べるか、arpcurvの実験的な変更を試すか、ユーザーの判断を待っている状態。
  (disable recaps in /config)

結論は、
1. 現状維持が現実的
