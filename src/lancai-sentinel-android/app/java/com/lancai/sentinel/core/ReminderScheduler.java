package com.lancai.sentinel.core;

// [OWNER: UI] 把 Rust 计算好的提醒排进 AlarmManager。
// 这里只做系统调度与 SharedPreferences 记录，不解析日程内容。

import android.app.AlarmManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.net.Uri;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.HashSet;
import java.util.Set;

public final class ReminderScheduler {

    private static final String TAG = "LCS";
    private static final String SP = "lancai_reminders";
    private static final String K_IDS = "scheduled_ids";
    private static final String K_SIGNUP_IDS = "signup_ids";
    private static final long HORIZON_MS = 30L * 24 * 3600 * 1000;

    /** 去重节流：1 分钟内不重复重排同一份数据。
     * 防止 save → toggle → delete 连续操作时多次全量重建 AlarmManager。 */
    private static volatile long lastRescheduleAtMs;
    private static volatile String lastRescheduleDataKey;

    private ReminderScheduler() {
    }

    /** 取消旧闹钟，再按当前数据重新排期。可能访问 JNI，建议在后台线程调用。 */
    public static void reschedule(Context c) {
        reschedule(c, false);
    }

    /**
     * @param force true 时跳过 1 分钟节流（开机、手动开关提醒等关键路径）。
     */
    public static void reschedule(Context c, boolean force) {
        try {
            cancelAll(c);
            if (!Config.get(c).isRemindEnabled()) return;
            if (!NotificationHelper.canNotify(c)) return;

            String json = RustCore.scheduleUpcoming(c, System.currentTimeMillis(), HORIZON_MS);
            if (json == null) return;

            // 节流：同一分钟内、同一份数据 → 跳过（上一轮一定已经排好了）。
            if (!force) {
                long now = System.currentTimeMillis();
                if (now - lastRescheduleAtMs < 60_000L && json.equals(lastRescheduleDataKey)) {
                    return;
                }
                lastRescheduleAtMs = now;
                lastRescheduleDataKey = json;
            }

            JSONArray arr = new JSONArray(json);
            if (arr.length() == 0) {
                prefs(c).edit().remove(K_IDS).remove(K_SIGNUP_IDS).apply();
                return;
            }

            AlarmManager am = (AlarmManager) c.getSystemService(Context.ALARM_SERVICE);
            if (am == null) return;
            Set<String> ids = new HashSet<>();
            Set<String> signupIds = new HashSet<>();
            for (int i = 0; i < arr.length(); i++) {
                JSONObject o = arr.optJSONObject(i);
                if (o == null) continue;
                long id = o.optLong("id", 0);
                long trigger = o.optLong("trigger_ms", 0);
                if (id == 0 || trigger <= System.currentTimeMillis()) continue;

                boolean signup = o.optBoolean("signup", false);
                Intent intent = new Intent(c, ReminderReceiver.class);
                intent.setAction(ReminderReceiver.ACTION);
                intent.setData(Uri.parse((signup ? "lancai://signup/" : "lancai://reminder/") + id));
                intent.putExtra("id", id);
                intent.putExtra("signup", signup);
                intent.putExtra("kind", o.optString("kind", "todo"));
                intent.putExtra("title", o.optString("title", ""));
                intent.putExtra("note", o.optString("note", ""));
                intent.putExtra("url", o.optString("url", ""));
                intent.putExtra("due_ms", o.optLong("due_ms", 0));

                int req = signup ? signupCode(id) : (int) (id & 0x7fffffffL);
                PendingIntent pi = PendingIntent.getBroadcast(c, req,
                        intent, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
                am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, trigger, pi);
                if (signup) {
                    signupIds.add(String.valueOf(id));
                } else {
                    ids.add(String.valueOf(id));
                }
            }
            prefs(c).edit()
                    .putStringSet(K_IDS, ids)
                    .putStringSet(K_SIGNUP_IDS, signupIds)
                    .apply();
        } catch (Throwable t) {
            Log.e(TAG, "重新排期提醒失败", t);
        }
    }

    /** 取消全部已记录的闹钟（关闭提醒、删除条目后调用）。 */
    public static void cancelAll(Context c) {
        try {
            cancelSet(c, K_IDS, false);
            cancelSet(c, K_SIGNUP_IDS, true);
            prefs(c).edit().remove(K_IDS).remove(K_SIGNUP_IDS).apply();
        } catch (Throwable t) {
            Log.e(TAG, "取消提醒失败", t);
        }
    }

    private static void cancelSet(Context c, String key, boolean signup) {
        Set<String> ids = prefs(c).getStringSet(key, new HashSet<String>());
        AlarmManager am = (AlarmManager) c.getSystemService(Context.ALARM_SERVICE);
        if (am == null || ids == null) return;
        for (String raw : new HashSet<>(ids)) {
            long id;
            try {
                id = Long.parseLong(raw);
            } catch (Throwable t) {
                continue;
            }
            Intent intent = new Intent(c, ReminderReceiver.class);
            intent.setAction(ReminderReceiver.ACTION);
            intent.setData(Uri.parse((signup ? "lancai://signup/" : "lancai://reminder/") + id));
            int req = signup ? signupCode(id) : (int) (id & 0x7fffffffL);
            PendingIntent pi = PendingIntent.getBroadcast(c, req, intent,
                    PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
            am.cancel(pi);
            pi.cancel();
        }
    }

    private static int signupCode(long id) {
        return (int) ((id & 0x7fffffffL) ^ 0x20000000);
    }

    private static SharedPreferences prefs(Context c) {
        return c.getSharedPreferences(SP, Context.MODE_PRIVATE);
    }
}
