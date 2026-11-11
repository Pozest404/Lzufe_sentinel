package com.lancai.sentinel.core;

// [OWNER: UI] 开机/应用更新后重新排期提醒（AlarmManager 会随重启丢失）。
// 使用 WakeLock 确保排期线程在 Doze/低功耗状态下也能跑完。

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.os.PowerManager;

public class BootReceiver extends BroadcastReceiver {

    private static final String WAKE_TAG = "lancai:BootReceiver";

    @Override
    public void onReceive(final Context context, Intent intent) {
        if (intent == null) return;
        // Doze/低功耗模式下给排期线程一点时间跑完；超时或异常都会自动释放。
        PowerManager pm = (PowerManager) context.getSystemService(Context.POWER_SERVICE);
        if (pm == null) {
            new Thread(new Runnable() {
                @Override
                public void run() {
                    ReminderScheduler.reschedule(context.getApplicationContext(), true);
                }
            }, "lancai-remind-boot").start();
            return;
        }
        PowerManager.WakeLock wl;
        if (Build.VERSION.SDK_INT >= 17) {
            wl = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, WAKE_TAG);
        } else {
            //noinspection deprecation
            wl = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, WAKE_TAG);
        }
        wl.setReferenceCounted(false);
        wl.acquire(15000L); // 最长 15s 后自动释放，兜底防泄漏

        final PowerManager.WakeLock finalWl = wl;
        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    ReminderScheduler.reschedule(context.getApplicationContext(), true);
                } finally {
                    if (finalWl.isHeld()) finalWl.release();
                }
            }
        }, "lancai-remind-boot").start();
    }
}
