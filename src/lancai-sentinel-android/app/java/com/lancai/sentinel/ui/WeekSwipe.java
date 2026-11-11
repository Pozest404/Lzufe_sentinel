package com.lancai.sentinel.ui;

// [OWNER: UI] 课表左右拖动切周：只旁路观察触摸事件，不打断子控件点击与纵向滚动。

import android.graphics.Rect;
import android.view.MotionEvent;
import android.view.View;

public final class WeekSwipe {

    public interface Host {
        /** 参与平移与切周动画的视图（课表容器）。 */
        View weekView();

        /** 数据未就绪时不响应滑动。 */
        boolean canSwitch();

        /** delta=+1 下一周 / -1 上一周；outDir 为滑出方向（-1 向左）。 */
        void switchWeek(int delta, int outDir);
    }

    private final Host host;
    private final Rect rect = new Rect();
    private float downX, downY, lastDx;
    private boolean dragging, blocked;

    public WeekSwipe(Host host) {
        this.host = host;
    }

    /** 在 Activity.dispatchTouchEvent 中调用；返回 true 表示消费本次事件（阻止松手后的误点击）。 */
    public boolean onTouch(MotionEvent ev) {
        View v = host.weekView();
        if (v == null) return false;

        switch (ev.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
                rect.setEmpty();
                v.getGlobalVisibleRect(rect);
                downX = ev.getRawX();
                downY = ev.getRawY();
                lastDx = 0;
                dragging = false;
                blocked = !rect.contains((int) downX, (int) downY) || !host.canSwitch();
                return false;

            case MotionEvent.ACTION_MOVE:
                if (blocked) return false;
                float dx = ev.getRawX() - downX;
                float dy = ev.getRawY() - downY;
                if (!dragging && Math.abs(dx) > dp(v, 10)
                        && Math.abs(dx) > Math.abs(dy) * 1.4f) {
                    dragging = true;
                }
                if (dragging) {
                    lastDx = dx;
                    v.setTranslationX(dx * 0.9f);
                }
                return false;

            case MotionEvent.ACTION_UP:
                if (!dragging) {
                    blocked = false;
                    return false;
                }
                dragging = false;
                int w = Math.max(1, v.getWidth());
                if (Math.abs(lastDx) > w * 0.18f) {
                    host.switchWeek(lastDx < 0 ? 1 : -1, lastDx < 0 ? -1 : 1);
                } else {
                    v.animate().translationX(0f).setDuration(130).start();
                }
                boolean consume = Math.abs(lastDx) > dp(v, 12);
                lastDx = 0;
                return consume;

            case MotionEvent.ACTION_CANCEL:
                if (dragging) v.animate().translationX(0f).setDuration(130).start();
                dragging = false;
                lastDx = 0;
                blocked = false;
                return false;

            default:
                return false;
        }
    }

    private static int dp(View v, float value) {
        return (int) (value * v.getResources().getDisplayMetrics().density + 0.5f);
    }
}
