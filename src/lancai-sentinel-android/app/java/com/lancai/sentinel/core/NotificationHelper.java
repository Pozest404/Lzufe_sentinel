package com.lancai.sentinel.core;

// [OWNER: UI] 系统通知：渠道、权限与提醒卡片。
// 只负责把 Rust 已经算好的提醒显示出来，不做任何业务计算。

import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;

import com.lancai.sentinel.R;
import com.lancai.sentinel.ui.DektActivity;
import com.lancai.sentinel.ui.ScheduleActivity;

public final class NotificationHelper {

    public static final String CHANNEL_ID = "lancai_reminder";
    public static final int PERMISSION_REQ = 9101;

    private NotificationHelper() {
    }

    public static void ensureChannel(Context c) {
        if (Build.VERSION.SDK_INT < 26) return;
        try {
            NotificationManager nm = (NotificationManager)
                    c.getSystemService(Context.NOTIFICATION_SERVICE);
            if (nm == null) return;
            NotificationChannel ch = new NotificationChannel(CHANNEL_ID,
                    "日程与活动提醒", NotificationManager.IMPORTANCE_HIGH);
            ch.setDescription("待办事项与第二课堂活动的定时提醒");
            nm.createNotificationChannel(ch);
        } catch (Throwable ignore) {
        }
    }

    public static boolean canNotify(Context c) {
        if (Build.VERSION.SDK_INT < 33) return true;
        try {
            return c.checkSelfPermission("android.permission.POST_NOTIFICATIONS")
                    == PackageManager.PERMISSION_GRANTED;
        } catch (Throwable t) {
            return true;
        }
    }

    public static void requestPermission(Activity a) {
        if (a == null || Build.VERSION.SDK_INT < 33 || canNotify(a)) return;
        try {
            a.requestPermissions(new String[]{"android.permission.POST_NOTIFICATIONS"},
                    PERMISSION_REQ);
        } catch (Throwable ignore) {
        }
    }

    /** 自动报名时间到：点击直接打开报名页并自动填写。 */
    public static void postSignup(Context c, long id, String title,
                                  String note, String url) {
        try {
            ensureChannel(c);
            if (!canNotify(c)) return;
            String u = url == null ? "" : url.trim();
            if (u.isEmpty()) {
                post(c, id, "activity", title, note, url, 0);
                return;
            }
            NotificationManager nm = (NotificationManager)
                    c.getSystemService(Context.NOTIFICATION_SERVICE);
            if (nm == null) return;

            Intent intent = new Intent(c, DektActivity.class);
            intent.putExtra(DektActivity.EXTRA_URL, u);
            intent.putExtra(DektActivity.EXTRA_AUTO_ENROLL, true);
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);

            int req = (int) ((id & 0x7fffffffL) ^ 0x20000000);
            PendingIntent pi = PendingIntent.getActivity(c, req, intent,
                    PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);

            String nText = note == null ? "" : note.trim();
            if (nText.isEmpty()) nText = "点击打开报名页，将自动填写报名信息";

            Notification.Builder b;
            if (Build.VERSION.SDK_INT >= 26) {
                b = new Notification.Builder(c, CHANNEL_ID);
            } else {
                b = new Notification.Builder(c);
                b.setPriority(Notification.PRIORITY_HIGH);
            }
            b.setSmallIcon(R.drawable.ic_stat_notify)
                    .setContentTitle("可以报名了：" + (title == null ? "活动" : title))
                    .setContentText(nText)
                    .setStyle(new Notification.BigTextStyle().bigText(nText))
                    .setContentIntent(pi)
                    .setAutoCancel(true)
                    .setShowWhen(true);
            nm.notify(req, b.build());
        } catch (Throwable ignore) {
        }
    }

    /** 点击通知：活动带链接时打开应用内第二课堂，否则打开日程页。 */
    public static void post(Context c, long id, String kind, String title,
                            String note, String url, long dueMs) {
        try {
            ensureChannel(c);
            if (!canNotify(c)) return;
            NotificationManager nm = (NotificationManager)
                    c.getSystemService(Context.NOTIFICATION_SERVICE);
            if (nm == null) return;

            Intent intent;
            String u = url == null ? "" : url.trim();
            if (!u.isEmpty()) {
                intent = new Intent(c, DektActivity.class);
                intent.putExtra(DektActivity.EXTRA_URL, u);
            } else {
                intent = new Intent(c, ScheduleActivity.class);
            }
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);

            int req = (int) (id & 0x7fffffffL);
            PendingIntent pi = PendingIntent.getActivity(c, req, intent,
                    PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);

            boolean activity = "activity".equals(kind);
            String nTitle = (activity ? "活动提醒：" : "待办提醒：")
                    + (title == null ? "" : title);
            String nText = note == null ? "" : note.trim();
            if (nText.isEmpty()) {
                nText = activity ? "第二课堂活动即将开始，点击查看" : "待办事项即将到期，点击查看";
            }

            Notification.Builder b;
            if (Build.VERSION.SDK_INT >= 26) {
                b = new Notification.Builder(c, CHANNEL_ID);
            } else {
                b = new Notification.Builder(c);
                b.setPriority(Notification.PRIORITY_HIGH);
            }
            b.setSmallIcon(R.drawable.ic_stat_notify)
                    .setContentTitle(nTitle)
                    .setContentText(nText)
                    .setStyle(new Notification.BigTextStyle().bigText(nText))
                    .setContentIntent(pi)
                    .setAutoCancel(true)
                    .setShowWhen(true)
                    .setWhen(dueMs > 0 ? dueMs : System.currentTimeMillis());
            nm.notify(req, b.build());
        } catch (Throwable ignore) {
        }
    }
}
