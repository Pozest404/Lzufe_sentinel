package com.lancai.sentinel.core;

// [OWNER: UI] 提醒闹钟触发：只把 Rust 排好的提醒显示成系统通知。

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

public class ReminderReceiver extends BroadcastReceiver {

    public static final String ACTION = "com.lancai.sentinel.REMIND";

    @Override
    public void onReceive(Context context, Intent intent) {
        if (intent == null) return;
        long id = intent.getLongExtra("id", 0);
        if (intent.getBooleanExtra("signup", false)) {
            NotificationHelper.postSignup(context,
                    id,
                    intent.getStringExtra("title"),
                    intent.getStringExtra("note"),
                    intent.getStringExtra("url"));
            return;
        }
        NotificationHelper.post(context,
                id,
                intent.getStringExtra("kind"),
                intent.getStringExtra("title"),
                intent.getStringExtra("note"),
                intent.getStringExtra("url"),
                intent.getLongExtra("due_ms", 0));
    }
}
