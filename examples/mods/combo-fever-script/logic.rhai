// Stosup MOD ロジックスクリプト実働サンプル（Rhai）
// 特別なコンパイル作業は一切不要です。このファイルを編集して
// MOD 管理画面の「再読み込み」を押すだけで即座に挙動が変わります。

const COMBO_WINDOW_MS = 15_000;

fn on_comment(ctx) {
    if ctx.is_banned {
        return;
    }

    // 「草 / www / 888 / パチパチ / ナイス / gg」を検出して 15 秒ウィンドウでコンボ集計
    if !contains_any(ctx.message, ["草", "www", "888", "パチパチ", "ナイス", "gg"]) {
        return;
    }

    let now = now_ms();
    let last_ms = state_get_or("combo_last_ms", 0);
    let combo = if now - last_ms <= COMBO_WINDOW_MS {
        state_inc("combo_count", 1)
    } else {
        state_set("combo_count", 1);
        1
    };
    state_set("combo_last_ms", now);

    log(`[コンボ] ${ctx.user_name}: 「${ctx.message}」 → ${combo} COMBO`);

    // 3 コンボ到達時、または以降 3 コンボごとに専用演出をコンボ数連動で発火
    if combo >= 3 && combo % 3 == 0 {
        let tier = if combo >= 12 {
            "legend"
        } else if combo >= 6 {
            "fever"
        } else {
            "normal"
        };
        let color = if tier == "legend" {
            "#F59E0B"
        } else if tier == "fever" {
            "#EC4899"
        } else {
            "#22C55E"
        };
        let label = if tier == "legend" {
            "LEGEND FEVER!!"
        } else if tier == "fever" {
            "SUPER FEVER!"
        } else {
            "COMBO HIT!"
        };

        trigger_mod_effect(
            "combo_burst",
            #{
                combo: combo,
                label: label,
                accentColor: color,
                tier: tier,
            },
            `${combo} コンボ達成！みんな盛り上がってます！`,
            3800
        );
    }
}

fn on_superchat(ctx) {
    if !ctx.is_superchat {
        return;
    }

    let amount = ctx.amount_jpy;
    let color = if amount >= 10000 {
        "#EF4444"
    } else if amount >= 5000 {
        "#EC4899"
    } else if amount >= 1000 {
        "#F59E0B"
    } else {
        "#38BDF8"
    };
    let tier = if amount >= 5000 { "legend" } else { "fever" };

    log(`[スパチャ演出] ${ctx.user_name} さんから ${ctx.amount_display} (${amount}円)`);

    trigger_mod_effect(
        "combo_burst",
        #{
            combo: amount,
            label: `THANK YOU ${ctx.amount_display}`,
            accentColor: color,
            tier: tier,
        },
        `${ctx.user_name} さん ${ctx.amount_display} ありがとうございます！`,
        4800
    );
}

fn on_first_time(ctx) {
    let total = state_inc("session_first_timers", 1);
    log(`[初見歓迎] ${ctx.user_name} さんいらっしゃい！（配信内 ${total} 人目）`);

    if total % 5 == 0 {
        trigger_builtin_effect(
            "confetti",
            `初見さん累計 ${total} 人突破！ようこそ！`,
            "🎉,✨,👋,🌟",
            45,
            4000
        );
    }
}

// MOD 管理画面の「テスト実行」ボタンから呼ばれる動作確認用フック
fn on_test(ctx) {
    let test_combo = state_inc("test_run_combo", 3);
    log(`テスト実行: ${test_combo} COMBO で combo_burst を発火します (入力: ${ctx.message})`);
    trigger_mod_effect(
        "combo_burst",
        #{
            combo: test_combo,
            label: "SCRIPT TEST FEVER!",
            accentColor: "#A855F7",
            tier: if test_combo >= 9 { "legend" } else { "fever" },
        },
        `Rhai スクリプト連携テスト (${test_combo} COMBO!)`,
        3600
    );
}
